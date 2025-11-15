use anyhow::Result;
use perplexica_service::PerplexicaService;
use rmcp::{ServiceExt, transport::stdio};

mod perplexica_service;

#[tokio::main]
async fn main() -> Result<()> {
    let service = PerplexicaService::new()?.serve(stdio()).await?;

    service.waiting().await?;

    Ok(())
}
