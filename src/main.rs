use anyhow::Result;
use perplexica_service::PerplexicaService;
use rmcp::{ServiceExt, transport::stdio};

mod perplexica_service;

#[tokio::main]
async fn main() -> Result<()> {
    // Create an instance of our Perplexica service
    let service = PerplexicaService::new()?.serve(stdio()).await?;

    // Wait for the service to complete
    service.waiting().await?;

    Ok(())
}
