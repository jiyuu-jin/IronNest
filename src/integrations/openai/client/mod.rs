use {
    crate::integrations::{iron_nest::types::Device, tplink::tplink_set_light_brightness},
    leptos::ServerFnError,
    serde_json::json,
};

cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
  use {
    crate::integrations::{
        roku::{roku_launch_app, roku_search, roku_send_keypress},
        tplink::{tplink_turn_plug_off, tplink_turn_plug_on, tplink_toggle_light},
    },
    async_openai::{
        types::{
            ChatCompletionFunctionsArgs, ChatCompletionRequestToolMessageArgs,
            ChatCompletionRequestUserMessageArgs, ChatCompletionToolArgs, ChatCompletionToolType,
            CreateChatCompletionRequestArgs,
        },
        Client,
    },
    sqlx::{Pool, Sqlite, Row},
};

pub enum AssistantFunction {
    RokuKeyPress { key: String },
    TPLinkTurnOn { ip: String },
    TPLinkTurnOff { ip: String },
    TPLinkToggleLight { ip: String, state: u8},
    TPLinkSetLightBrightness {ip: String, brightness: u8},
    RokuSearch { query: String },
    RokuLaunchApp { app_id: String },
}

impl AssistantFunction {
    async fn execute(self) -> Result<String, ServerFnError> {
        match self {
            AssistantFunction::RokuKeyPress { key } => {
                roku_send_keypress(&key).await;
                Ok(format!("Roku Key Pressed: {}", key))
            }
            AssistantFunction::TPLinkTurnOn { ip } => {
                tplink_turn_plug_on(&ip).await;
                Ok(format!("TP-link plug turned on"))
            }
            AssistantFunction::TPLinkTurnOff { ip } => {
                tplink_turn_plug_off(&ip).await;
                Ok(format!("TP-link plug turned off"))
            }
            AssistantFunction::TPLinkToggleLight { ip, state } => {
                tplink_toggle_light(&ip, state).await;
                Ok(format!("TP-link switch turned off"))
            }
            AssistantFunction::TPLinkSetLightBrightness { ip, brightness } => {
                tplink_set_light_brightness(&ip, brightness).await;
                Ok(format!("TP-link switch brightness set"))
            }
            AssistantFunction::RokuSearch { query } => {
                roku_search(&query).await;
                Ok(format!("Roku search sent"))
            }
            AssistantFunction::RokuLaunchApp { app_id } => {
                roku_launch_app(&app_id).await;
                Ok(format!("Roku app launched"))
            }
        }
    }
}
}}

pub async fn open_api_command(text: String, pool: &Pool<Sqlite>) -> Result<String, ServerFnError> {
    println!("calling assistant with {:?}", text);
    let client = Client::new();

    let mut tp_link_plug_ips: Vec<String> = Vec::new();
    let mut tp_link_light_ips: Vec<String> = Vec::new();
    let mut roku_ips: Vec<String> = Vec::new();
    let mut devices: Vec<Device> = Vec::new();

    let rows = sqlx::query("SELECT id, name, device_type, ip, power_state FROM devices")
        .fetch_all(pool)
        .await?;

    for row in rows {
        let state_value: i64 = row.get("power_state");
        let state: u8 = state_value.try_into().expect("Value out of range for u64");
        let ip: String = row.get("ip");

        devices.push(Device {
            id: row.get("id"),
            name: row.get("name"),
            device_type: row.get("device_type"),
            ip: ip.to_string(),
            state,
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
        devices
    );

    let request = CreateChatCompletionRequestArgs::default()
    .max_tokens(512u16)
    .model("gpt-3.5-turbo-1106")
    .messages([
        ChatCompletionRequestUserMessageArgs::default()
            .content(initial_system_prompt)
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
                            "key": { 
                                "type": "string", 
                                "enum": [ 
                                    "powerOn", "powerOff", "home", "rev", "fwd", "play", "select", "left", "right", "down", "up", "back", 
                                    "replay", "info", "backspace", "enter", "volumeDown", "volumeUp", "volumeMute", "inputTuner", 
                                    "inputHDMI1", "inputHDMI2", "inputHDMI3", "inputHDMI4", "inputAV1", "channelUp", "channelDown"
                                ]
                            },
                        },
                        "required": ["key"],
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
                    .name("tplink_toggle_light")
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
            .build().unwrap()
    ])
    .build().unwrap();

    println!("{}", serde_json::to_string(&request).unwrap());

    let response_message = client
        .chat()
        .create(request)
        .await
        .unwrap()
        .choices
        .get(0)
        .unwrap()
        .message
        .clone();

    println!(
        "response message {}",
        serde_json::to_string(&response_message).unwrap()
    );

    let value = if let Some(tool_calls) = response_message.tool_calls {
        let mut handles = Vec::new();
        for tool_call in tool_calls.iter() {
            let function_name = tool_call.function.name.to_string();
            let function_args: serde_json::Value = tool_call.function.arguments.parse().unwrap();
            let assistant_function = match function_name.as_str() {
                "roku_send_keypress" => {
                    let key = function_args["key"]
                        .to_string()
                        .trim_matches('"')
                        .to_string();
                    AssistantFunction::RokuKeyPress { key }
                }
                "tplink_turn_plug_on" => {
                    let ip = function_args["ip"].to_string();
                    AssistantFunction::TPLinkTurnOn { ip }
                }
                "tplink_turn_plug_off" => {
                    let ip = function_args["ip"].to_string();
                    AssistantFunction::TPLinkTurnOff { ip }
                }
                "tplink_toggle_light" => {
                    let ip = function_args["ip"].to_string();
                    let state: u8 = function_args["state"].to_string().parse().unwrap();
                    AssistantFunction::TPLinkToggleLight { ip, state }
                }
                "tplink_set_light_brightness" => {
                    let ip = function_args["ip"].to_string();
                    let brightness: u8 = function_args["brightness"].to_string().parse().unwrap();
                    AssistantFunction::TPLinkSetLightBrightness { ip, brightness }
                }
                "roku_search" => {
                    let query = function_args["query"].to_string();
                    AssistantFunction::RokuSearch { query }
                }
                "roku_launch_app" => {
                    let app_id = function_args["app_id"].to_string();
                    AssistantFunction::RokuLaunchApp { app_id }
                }
                &_ => return Err(ServerFnError::ServerError("Function not found".to_string())),
            };

            let function_response = assistant_function.execute().await?;
            handles.push((tool_call.id.to_string(), function_response));
        }

        let mut message = vec![ChatCompletionRequestUserMessageArgs::default()
            .content(text)
            .build()?
            .into()];

        // let tool_calls: Vec<ChatCompletionMessageToolCall> = handles
        //     .iter()
        //     .map(|(tool_call, _response_content)| tool_call.clone())
        //     .collect();

        for handle in handles {
            message.push(
                ChatCompletionRequestToolMessageArgs::default()
                    .tool_call_id(handle.0)
                    .content(handle.1)
                    .build()?
                    .into(),
            )
        }

        println!("{}", serde_json::to_string(&message).unwrap());

        let request = CreateChatCompletionRequestArgs::default()
            .max_tokens(512u16)
            .model("gpt-3.5-turbo-1106")
            .messages(message)
            .build()?;

        let response = client.chat().create(request).await.unwrap();
        format!(
            "{:?}",
            match response.choices[0].message.content {
                None => "No output found!",
                Some(ref x) => x,
            }
        )
    } else {
        "".to_string()
    };

    Ok(value)
}
