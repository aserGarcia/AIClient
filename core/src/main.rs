use async_openai::{
    Client,
    config::OpenAIConfig,
    traits::RequestOptionsBuilder,
    types::chat::{
        ChatCompletionRequestAssistantMessage as AssistantMessage,
        ChatCompletionRequestSystemMessage as SystemMessage,
        ChatCompletionRequestUserMessage as UserMessage,
        CreateChatCompletionRequestArgs as CreateRequestArgs,
    },
};
use convo_core::assistant::LlamaCpp;

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

            let request = CreateRequestArgs::default()
                .model("microsoft/Phi-3-mini-4k-instruct-gguf:Phi-3-mini-4k-instruct-q4.gguf")
                .messages([
                    SystemMessage::from("You are a helpful assistant.").into(),
                    UserMessage::from("Write a poem about programming").into(),
                ])
                .build()
                .expect("Unable to uild request args");

            let response = client.chat().create(request).await.expect("chat not work");

            println!("\nResponse:\n");
            for choice in response.choices {
                println!(
                    "{}: Role: {}  Content: {:?}",
                    choice.index, choice.message.role, choice.message.content
                );
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
