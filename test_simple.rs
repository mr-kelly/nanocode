use genai::{Client, chat::{ChatMessage, ChatRequest}};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("OPENAI_API_KEY", "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");
    std::env::set_var("OPENAI_API_BASE", "https://openrouter.ai/api/v1");
    
    let client = Client::default();
    let req = ChatRequest::new(vec![ChatMessage::user("Say hello")])
        .with_system("You are a helpful assistant");
    
    println!("Sending request...");
    let response = client.exec_chat("arcee-ai/trinity-large-preview:free", req, None).await?;
    println!("Response: {:?}", response.first_text());
    
    Ok(())
}
