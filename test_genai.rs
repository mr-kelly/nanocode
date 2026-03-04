use genai::{
    adapter::AdapterKind,
    chat::{ChatMessage, ChatRequest},
    resolver::{AuthData, Endpoint},
    Client, ModelIden, ServiceTarget,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = "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";
    let base_url = "https://openrouter.ai/api/v1/";
    
    let client = Client::builder()
        .with_service_target_resolver_fn(move |mut st: ServiceTarget| {
            st.endpoint = Endpoint::from_owned(base_url.to_string());
            st.auth = AuthData::from_single(key.to_string());
            st.model = ModelIden::new(AdapterKind::OpenAI, st.model.model_name);
            Ok(st)
        })
        .build();
    
    let messages = vec![ChatMessage::user("Say hello")];
    let req = ChatRequest::new(messages);
    
    println!("Sending request...");
    let response = client.exec_chat("arcee-ai/trinity-large-preview:free", req, None).await?;
    println!("Response: {}", response.content_text_as_str().unwrap_or(""));
    
    Ok(())
}
