use {
    crate::integrations::iron_nest::{execute_function, types::Device},
    chrono::Utc,
    futures::future::join_all,
    leptos::ServerFnError,
    log::info,
    serde_json::json,
    sqlx::{PgPool, Row},
};

cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
  use {
    futures::StreamExt,
    async_openai::{
        types::{
            ChatCompletionFunctionsArgs,
            ChatCompletionRequestUserMessageArgs, ChatCompletionToolArgs, ChatCompletionToolType,
            CreateChatCompletionRequestArgs,ChatCompletionMessageToolCall,
            ChatCompletionRequestAssistantMessageArgs,
            ChatCompletionRequestToolMessageArgs,
            ChatCompletionRequestMessage,
            CreateSpeechRequestArgs,
            Voice,
            SpeechModel,
        },
        Client,
        config::OpenAIConfig,
    },
    };

    impl Device {
        pub fn format_for_openapi(&self) -> String {
            format!("{} - {} - {} - {}\n", self.name, self.device_type, self.ip, self.power_state)
        }
    }

    pub fn format_devices(devices: Vec<Device>) -> String {
        devices.iter().map(|device| device.format_for_openapi()).collect()
    }
}}

pub async fn open_api_command(text: String, pool: &PgPool) -> Result<String, ServerFnError> {
    println!("calling assistant with {:?}", text);
    let client = Client::new();

    let mut tp_link_plug_ips: Vec<String> = Vec::new();
    let mut tp_link_light_ips: Vec<String> = Vec::new();
    let mut roku_ips: Vec<String> = Vec::new();
    let mut devices: Vec<Device> = Vec::new();

    let rows = sqlx::query("SELECT id, name, device_type, ip, power_state, last_seen FROM devices")
        .fetch_all(pool)
        .await?;

    for row in rows {
        let state_value: i32 = row.get("power_state");
        let state: i32 = state_value;
        let ip: String = row.get("ip");
        // let battery_percentage_value: i64 = row.get("battery_percentage");
        // let battery_percentage: u64 = battery_percentage_value
        //     .try_into()
        //     .expect("Value out of range for u64");

        devices.push(Device {
            id: row.get("id"),
            name: row.get("name"),
            device_type: row.get("device_type"),
            ip: ip.to_string(),
            power_state: state,
            battery_percentage: 0,
            last_seen: Utc::now(),
            mac_address: None,
            child_id: None,
        });

        let device_type: String = row.get("device_type");
        match device_type.as_str() {
            "roku" => roku_ips.push(ip.to_string()),
            "smart-plug" => tp_link_plug_ips.push(ip),
            "smart-light" => tp_link_light_ips.push(ip),
            _ => (),
        }
    }

    let initial_system_prompt = format!(
        "You are a home assistant named Iron Nest.
        Here are the following device names:
        {:?}
        Respond to the following input: {text}",
        format_devices(devices)
    );

    info!("Prompt Length: {}", initial_system_prompt.len());

    let request = CreateChatCompletionRequestArgs::default()
    .max_tokens(512u16)
    .model("gpt-3.5-turbo-1106")
    .messages([
        ChatCompletionRequestUserMessageArgs::default()
            .content(initial_system_prompt.to_string())
            .build()?
            .into()
    ])
    .tools(vec![
        ChatCompletionToolArgs::default()
            .r#type(ChatCompletionToolType::Function)
            .function(
                ChatCompletionFunctionsArgs::default()
                    .name("roku_send_keypress")
                    .description("Send a keypress to a roku tv device")
                    .parameters(json!({
                        "type": "object",
                        "properties": {
                            "ip": { 
                                "type": "string", 
                                "enum": roku_ips,
                            },
                            "key": { 
                                "type": "string", 
                                "enum": [ 
                                    "powerOn", "powerOff", "home", "rev", "fwd", "play", "select", "left", "right", "down", "up", "back", 
                                    "replay", "info", "backspace", "enter", "volumeDown", "volumeUp", "volumeMute", "inputTuner", 
                                    "inputHDMI1", "inputHDMI2", "inputHDMI3", "inputHDMI4", "inputAV1", "channelUp", "channelDown"
                                ]
                            },
                        },
                        "required": ["key", "ip"],
                    }))
                    .build().unwrap()
            )
            .build().unwrap(),
        ChatCompletionToolArgs::default()
            .r#type(ChatCompletionToolType::Function)
            .function(
                ChatCompletionFunctionsArgs::default()
                    .name("tplink_turn_plug_on")
                    .description("Turn on tplink smart plug")
                    .parameters(json!({
                        "type": "object",
                        "properties": {
                            "ip": { 
                                "type": "string", 
                                "enum": tp_link_plug_ips,
                            },
                        },
                        "required": ["ip"],
                    }))
                    .build().unwrap()
            )
            .build().unwrap(),
        ChatCompletionToolArgs::default()
            .r#type(ChatCompletionToolType::Function)
            .function(
                ChatCompletionFunctionsArgs::default()
                    .name("tplink_turn_plug_off")
                    .description("Turn off tplink smart plug")
                    .parameters(json!({
                        "type": "object",
                        "properties": {
                            "ip": { 
                                "type": "string", 
                                "enum": tp_link_plug_ips,
                            },
                        },
                        "required": ["ip"],
                    }))
                    .build().unwrap()
            )
            .build().unwrap(),
        ChatCompletionToolArgs::default()
            .r#type(ChatCompletionToolType::Function)
            .function(
                ChatCompletionFunctionsArgs::default()
                    .name("tplink_turn_light_on_off")
                    .description("Turn on or off tplink smart light")
                    .parameters(json!({
                        "type": "object",
                        "properties": {
                            "ip": { 
                                "type": "string", 
                                "enum": tp_link_light_ips,
                            },
                            "state": {
                                "type": "number",
                                "enum": [0, 1],
                            }
                        },
                        "required": ["ip", "state"],
                    }))
                    .build().unwrap()
            )
            .build().unwrap(),
        ChatCompletionToolArgs::default()
            .r#type(ChatCompletionToolType::Function)
            .function(
                ChatCompletionFunctionsArgs::default()
                    .name("tplink_set_light_brightness")
                    .description("Set tplink smart light brightness (1 - 100)")
                    .parameters(json!({
                        "type": "object",
                        "properties": {
                            "ip": { 
                                "type": "string", 
                                "enum": tp_link_light_ips,
                            },
                            "brightness": {
                                "type": "number",
                            }
                        },
                        "required": ["ip", "brightness"],
                    }))
                    .build().unwrap()
            )
            .build().unwrap(),
        ChatCompletionToolArgs::default()
            .r#type(ChatCompletionToolType::Function)
            .function(
                ChatCompletionFunctionsArgs::default()
                    .name("roku_search")
                    .description("Open the Roku search page with the given search params")
                    .parameters(json!({
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "The value to search for on the roku",
                            },
                        },
                        "required": [],
                    }))
                    .build().unwrap()
            )
            .build().unwrap(),
        ChatCompletionToolArgs::default()
            .r#type(ChatCompletionToolType::Function)
            .function(
                ChatCompletionFunctionsArgs::default()
                    .name("roku_launch_app")
                    .description("")
                    .parameters(json!({
                        "type": "object",
                        "properties": {
                            "app_id": {
                                "type": "string",
                                "description": "Roku app_id or name, e,g YouTube or 837",
                            },
                        },
                        "required": [],
                    }))
                    .build().unwrap()
            )
            .build().unwrap(),
            ChatCompletionToolArgs::default()
                .r#type(ChatCompletionToolType::Function)
                .function(
                    ChatCompletionFunctionsArgs::default()
                        .name("stoplight_toggle")
                        .description("Toggle current state of red, green, or yellow by name")
                        .parameters(json!({
                            "type": "object",
                            "properties": {
                                "color": {
                                    "type": "string",
                                    "description": "The color light to toggle",
                                    "enum": ["red", "green", "yellow"],
                                },
                            },
                            "required": [],
                        }))
                        .build().unwrap()
                )
                .build().unwrap(),
    ])
    .build().unwrap();

    println!("{}", serde_json::to_string(&request).unwrap());

    let response_message = client
        .chat()
        .create(request)
        .await
        .unwrap()
        .choices
        .first()
        .unwrap()
        .message
        .clone();

    let value = if let Some(tool_calls) = response_message.tool_calls {
        let tool_call_futs = tool_calls.iter().map(|tool_call| async {
            let function_name = tool_call.function.name.to_string();
            let function_args: serde_json::Value = tool_call.function.arguments.parse().unwrap();
            let function_response = execute_function(function_name, function_args).await;

            (tool_call.clone(), function_response.to_string())
        });
        let function_responses = join_all(tool_call_futs).await;

        let mut messages: Vec<ChatCompletionRequestMessage> =
            vec![ChatCompletionRequestUserMessageArgs::default()
                .content(initial_system_prompt)
                .build()?
                .into()];

        let tool_calls: Vec<ChatCompletionMessageToolCall> = function_responses
            .iter()
            .map(|(tool_call, _response_content)| tool_call.clone())
            .collect();

        let assistant_messages: ChatCompletionRequestMessage =
            ChatCompletionRequestAssistantMessageArgs::default()
                .tool_calls(tool_calls)
                .build()?
                .into();

        let tool_messages: Vec<ChatCompletionRequestMessage> = function_responses
            .iter()
            .map(|(tool_call, response_content)| {
                ChatCompletionRequestToolMessageArgs::default()
                    .content(response_content.to_string())
                    .tool_call_id(tool_call.id.clone())
                    .build()
                    .unwrap()
                    .into()
            })
            .collect();

        messages.push(assistant_messages);
        messages.extend(tool_messages);

        let subsequent_request = CreateChatCompletionRequestArgs::default()
            .max_tokens(512u16)
            .model("gpt-3.5-turbo-1106")
            .messages(messages)
            .build()?;

        let mut stream = client.chat().create_stream(subsequent_request).await?;
        let mut response_content = String::new();

        while let Some(result) = stream.next().await {
            match result {
                Ok(response) => {
                    for chat_choice in response.choices.iter() {
                        if let Some(ref content) = chat_choice.delta.content {
                            response_content.push_str(content);
                        }
                    }
                }
                Err(err) => println!("{err}"),
            }
        }
        // whisper_tts(client, &response_content).await.unwrap();
        response_content
    } else {
        "".to_string()
    };

    Ok(value)
}

pub async fn whisper_tts(client: Client<OpenAIConfig>, text: &str) -> Result<(), ServerFnError> {
    let request = CreateSpeechRequestArgs::default()
        .input(text)
        .voice(Voice::Nova)
        .model(SpeechModel::Tts1)
        .build()?;

    let response = client.audio().speech(request).await?;
    let _speech_bytes = response.bytes;

    Ok(())
}
