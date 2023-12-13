use {leptos::ServerFnError, serde_json::json};

cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
  use {
    crate::integrations::{
        roku::{roku_launch_app, roku_search, roku_send_keypress},
        tplink::{tplink_turn_off, tplink_turn_on},
    },
    async_openai::{
        types::{
            ChatCompletionFunctionsArgs, ChatCompletionRequestToolMessageArgs,
            ChatCompletionRequestUserMessageArgs, ChatCompletionToolArgs, ChatCompletionToolType,
            CreateChatCompletionRequestArgs,
        },
        Client,
    },
};

pub enum AssistantFunction {
    RokuKeyPress { key: String },
    TPLinkTurnOn {},
    TPLinkTurnOff {},
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
            AssistantFunction::TPLinkTurnOn {} => {
                tplink_turn_on().await;
                Ok(format!("TP-link switch turned on"))
            }
            AssistantFunction::TPLinkTurnOff {} => {
                tplink_turn_off().await;
                Ok(format!("TP-link switch turned off"))
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

pub async fn open_api_command(text: String) -> Result<String, ServerFnError> {
    println!("calling assistant with {:?}", text);
    let client = Client::new();

    let request = CreateChatCompletionRequestArgs::default()
    .max_tokens(512u16)
    .model("gpt-3.5-turbo-1106")
    .messages([
        ChatCompletionRequestUserMessageArgs::default()
            .content(text.to_string())
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
                    .name("tplink_turn_on")
                    .description("Turn on tplink switch")
                    .parameters(json!({
                        "type": "object",
                        "properties": {},
                        "required": [],
                    }))
                    .build().unwrap()
            )
            .build().unwrap(),
        ChatCompletionToolArgs::default()
            .r#type(ChatCompletionToolType::Function)
            .function(
                ChatCompletionFunctionsArgs::default()
                    .name("tplink_turn_off")
                    .description("Turn off tplink switch")
                    .parameters(json!({
                        "type": "object",
                        "properties": {},
                        "required": [],
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
                    let key = function_args["key"].as_str().unwrap().to_string();
                    AssistantFunction::RokuKeyPress { key }
                }
                "tplink_turn_on" => AssistantFunction::TPLinkTurnOn {},
                "tplink_turn_off" => AssistantFunction::TPLinkTurnOff {},
                "roku_search" => {
                    let query = function_args["query"].as_str().unwrap().to_string();
                    AssistantFunction::RokuSearch { query }
                }
                "roku_launch_app" => {
                    let app_id = function_args["app_id"].as_str().unwrap().to_string();
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
