use async_openai::{
    Client,
    config::OpenAIConfig,
    traits::RequestOptionsBuilder,
    types::chat::{
        ChatCompletionRequestSystemMessage as SystemMessage,
        ChatCompletionRequestUserMessage as UserMessage,
        CreateChatCompletionRequestArgs as CreateChatRequestArgs,
    },
};
use convo_core::assistant::LlamaCpp;
use futures::StreamExt;

#[tokio::main]
async fn main() {
    let model = LlamaCpp::boot().await;
    match model {
        Ok(mut llamacpp) => {
            if let Err(res) = llamacpp.wait_until_ready().await {
                println!("{}", res);
            }

            let config = OpenAIConfig::new().with_api_base(llamacpp.url());
            let client = Client::with_config(config);

            let request = CreateChatRequestArgs::default()
                .model("microsoft/Phi-3-mini-4k-instruct-gguf:Phi-3-mini-4k-instruct-q4.gguf")
                .messages([
                    SystemMessage::from("You are a helpful assistant.").into(),
                    UserMessage::from("Write a poem about programming").into(),
                ])
                .n(1)
                .stream(true)
                .build()
                .expect("Unable to uild request args");

            let mut stream = client
                .chat()
                .create_stream(request)
                .await
                .expect("chat not work");

            println!("\nResponse:\n");
            while let Some(resp) = stream.next().await {
                match resp {
                    Ok(ccr) => ccr.choices.iter().for_each(|c| {
                        if let Some(content) = c.delta.content.as_ref() {
                            print!("{}", content);
                        }
                    }),
                    Err(e) => println!("Error: {}", e),
                }
            }

            // let json_data = r#"{"prompt": "Write a short poem about programming."}"#;
            // let client = reqwest::Client::new();
            // let resp = client
            //     .post(format!("{}/completions", llamacpp.url()))
            //     .header("Content-Type", "application/json")
            //     .body(json_data.to_string())
            //     .send()
            //     .await
            //     .expect("Failed to send request");
            //
            // println!("{:?}", resp.text().await);
        }
        Err(e) => println!("Error: {}", e),
    }
}
