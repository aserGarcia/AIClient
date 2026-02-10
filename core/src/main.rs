use async_openai::types::chat::{
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage as SystemMessage,
    ChatCompletionRequestUserMessage as UserMessage,
};
use convo_core::assistant::LlamaCpp;
use sipper::Sipper;
use std::io::Write;

#[tokio::main]
async fn main() {
    let model = LlamaCpp::boot().await;
    match model {
        Ok(mut llamacpp) => {
            if let Err(res) = llamacpp.wait_until_ready().await {
                println!("{}", res);
            }

            let messages: Vec<ChatCompletionRequestMessage> = vec![
                SystemMessage::from("You are a helpful assistant.").into(),
                UserMessage::from("Write a poem about programming").into(),
            ];

            let mut stream = llamacpp.stream_response::<String>(messages).pin();
            while let Some(token) = stream.sip().await {
                print!("{}", token);
                std::io::stdout().flush().unwrap();
            }
        }
        Err(e) => println!("Error: {}", e),
    }
}
