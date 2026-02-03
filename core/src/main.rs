use convo_core::assistant::LlamaCpp;

#[tokio::main]
async fn main() {
    let model = LlamaCpp::boot().await;
    match model {
        Ok(mut llamacpp) => {
            if let Err(res) = llamacpp.wait_until_ready().await {
                println!("{}", res);
            }
        }
        Err(e) => println!("Error: {}", e),
    }
}
