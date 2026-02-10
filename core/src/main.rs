use convo_core::{assistant::LlamaCpp, chat::ChatMessage};
use sipper::Sipper;
use std::io::Write;

#[tokio::main]
async fn main() {
    let model = LlamaCpp::boot();
    match model {
        Ok(mut llamacpp) => {
            if let Err(res) = llamacpp.wait_until_ready().await {
                println!("{}", res);
            }

            let messages = vec![("Write a poem about programming.".to_string(), false)];

            let mut stream = llamacpp.stream_response::<String>(messages).pin();
            while let Some(token) = stream.sip().await {
                print!("{}", token);
                std::io::stdout().flush().unwrap();
            }
        }
        Err(e) => println!("Error: {}", e),
    }
}
