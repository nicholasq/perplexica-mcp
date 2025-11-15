use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ErrorData as McpError, *},
    schemars, tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt::Write;

#[derive(Debug, Clone)]
pub struct PerplexicaService {
    tool_router: ToolRouter<PerplexicaService>,
    search_url: String,
    providers_url: String,
    client: reqwest::Client,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PerplexicaSearchRequest {
    #[schemars(description = "The search query to send to Perplexica")]
    pub query: String,

    #[schemars(description = "The focus mode for search (e.g., 'webSearch', 'academicSearch')")]
    #[serde(default = "default_focus_mode")]
    pub focus_mode: Cow<'static, str>,

    #[schemars(description = "Whether to stream response")]
    #[serde(default = "default_stream")]
    pub stream: bool,

    #[schemars(description = "Chat history as array of [role, message] pairs")]
    #[serde(default)]
    pub history: Option<Vec<Vec<String>>>,

    #[schemars(description = "System instructions for search")]
    #[serde(default)]
    pub system_instructions: Option<String>,

    #[schemars(
        description = "Provider ID to use. DO NOT SET unless user explicitly specifies a provider. Will use default from environment variables if omitted."
    )]
    #[serde(default)]
    pub provider_id: Option<String>,

    #[schemars(
        description = "Chat model key to use. DO NOT SET unless user explicitly specifies a model. Will use default from environment variables if omitted."
    )]
    #[serde(default)]
    pub chat_model_key: Option<String>,

    #[schemars(
        description = "Embedding model key to use. DO NOT SET unless user explicitly specifies an embedding model. Will use default from environment variables if omitted."
    )]
    #[serde(default)]
    pub embedding_model_key: Option<String>,
}

fn default_focus_mode() -> Cow<'static, str> {
    Cow::Borrowed("webSearch")
}

fn default_stream() -> bool {
    false
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct ProvidersResponse {
    pub providers: Vec<Provider>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct Provider {
    pub id: String,
    pub name: String,
    #[serde(rename = "chatModels")]
    pub chat_models: Vec<Model>,
    #[serde(rename = "embeddingModels")]
    pub embedding_models: Vec<Model>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct Model {
    pub name: String,
    pub key: String,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct PerplexicaSearchResponse {
    pub message: String,
    pub sources: Vec<Source>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct Source {
    #[serde(rename = "pageContent")]
    pub page_content: String,
    pub metadata: SourceMetadata,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct SourceMetadata {
    pub title: String,
    pub url: String,
}

#[derive(Debug, Serialize)]
struct PerplexicaApiRequest {
    #[serde(rename = "chatModel")]
    chat_model: ChatModel,
    #[serde(rename = "embeddingModel")]
    embedding_model: EmbeddingModel,
    #[serde(rename = "optimizationMode")]
    optimization_mode: Cow<'static, str>,
    #[serde(rename = "focusMode")]
    focus_mode: Cow<'static, str>,
    query: String,
    history: Option<Vec<Vec<String>>>,
    #[serde(rename = "systemInstructions")]
    system_instructions: Option<String>,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct ChatModel {
    #[serde(rename = "providerId")]
    provider_id: String,
    key: String,
}

#[derive(Debug, Serialize)]
struct EmbeddingModel {
    #[serde(rename = "providerId")]
    provider_id: String,
    key: String,
}

#[tool_router]
impl PerplexicaService {
    pub fn new() -> anyhow::Result<Self> {
        let api_url = std::env::var("PERPLEXICA_API_URL")
            .map_err(|_| anyhow::anyhow!("PERPLEXICA_API_URL environment variable must be set"))?;

        let base_url = api_url.trim_end_matches('/');

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .pool_max_idle_per_host(10)
            .build()?;

        let search_url = format!("{}/api/search", base_url);
        let providers_url = format!("{}/api/providers", base_url);

        Ok(Self {
            tool_router: Self::tool_router(),
            search_url,
            providers_url,
            client,
        })
    }

    fn resolve_provider_value(
        param_value: Option<String>,
        env_var: &str,
        field_name: &str,
    ) -> Result<String, McpError> {
        match param_value {
            Some(value) => Ok(value),
            None => std::env::var(env_var).map_err(|_| McpError {
                code: ErrorCode(-32602),
                message: Cow::from(format!(
                    "Missing {}. Set either the {} parameter or {} environment variable",
                    field_name, field_name, env_var
                )),
                data: None,
            }),
        }
    }

    #[tool(
        description = "Search using Perplexica API. Provider and model parameters are optional - the server will use configured defaults unless the user explicitly specifies otherwise."
    )]
    async fn perplexica_search(
        &self,
        Parameters(request): Parameters<PerplexicaSearchRequest>,
    ) -> Result<CallToolResult, McpError> {
        let provider_id = Self::resolve_provider_value(
            request.provider_id,
            "PERPLEXICA_PROVIDER_ID",
            "provider_id",
        )?;

        let chat_model_key = Self::resolve_provider_value(
            request.chat_model_key,
            "PERPLEXICA_CHAT_MODEL_KEY",
            "chat_model_key",
        )?;

        let embedding_model_key = Self::resolve_provider_value(
            request.embedding_model_key,
            "PERPLEXICA_EMBEDDING_MODEL_KEY",
            "embedding_model_key",
        )?;

        let api_request = PerplexicaApiRequest {
            chat_model: ChatModel {
                provider_id: provider_id.clone(),
                key: chat_model_key,
            },
            embedding_model: EmbeddingModel {
                provider_id,
                key: embedding_model_key,
            },
            optimization_mode: Cow::Borrowed("speed"),
            focus_mode: request.focus_mode,
            query: request.query,
            history: request.history,
            system_instructions: request.system_instructions,
            stream: request.stream,
        };

        let response = self
            .client
            .post(&self.search_url)
            .json(&api_request)
            .send()
            .await
            .map_err(|e| McpError {
                code: ErrorCode(-32603),
                message: Cow::from(format!("Search request failed: {}", e)),
                data: None,
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error response".into());

            return Err(McpError {
                code: ErrorCode(-32603),
                message: Cow::from(format!(
                    "Perplexica search API error (status {}): {}",
                    status, error_text
                )),
                data: None,
            });
        }

        let search_response: PerplexicaSearchResponse = match response.json().await {
            Ok(data) => data,
            Err(e) => {
                return Err(McpError {
                    code: ErrorCode(-32603),
                    message: Cow::from(format!("Failed to parse search response as JSON: {}", e)),
                    data: None,
                });
            }
        };

        let estimated_capacity = search_response.message.len()
            + search_response
                .sources
                .iter()
                .map(|s| s.metadata.title.len() + s.metadata.url.len() + 10)
                .sum::<usize>()
            + 100; // Header/footer overhead
        let mut markdown = String::with_capacity(estimated_capacity);

        if writeln!(
            markdown,
            "## Summary\n\n{}\n\n## Sources\n\n",
            search_response.message
        )
        .is_err()
        {
            return Err(McpError {
                code: ErrorCode(-32603),
                message: Cow::from("Failed to format markdown response"),
                data: None,
            });
        }

        if search_response.sources.is_empty() {
            markdown.push_str("No sources found.\n");
        } else {
            for source in &search_response.sources {
                if writeln!(
                    markdown,
                    "- {}\n  - {}\n",
                    source.metadata.title, source.metadata.url
                )
                .is_err()
                {
                    return Err(McpError {
                        code: ErrorCode(-32603),
                        message: Cow::from("Failed to format markdown response"),
                        data: None,
                    });
                }
            }
        }

        Ok(CallToolResult::success(vec![Content::text(markdown)]))
    }

    #[tool(description = "Retrieve available providers and their models from Perplexica API")]
    async fn perplexica_providers(&self) -> Result<CallToolResult, McpError> {
        let response = self
            .client
            .get(&self.providers_url)
            .send()
            .await
            .map_err(|e| McpError {
                code: ErrorCode(-32603),
                message: Cow::from(format!("Providers request failed: {}", e)),
                data: None,
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error response".into());

            return Err(McpError {
                code: ErrorCode(-32603),
                message: Cow::from(format!(
                    "Perplexica providers API error (status {}): {}",
                    status, error_text
                )),
                data: None,
            });
        }

        let providers_response: ProvidersResponse = match response.json().await {
            Ok(data) => data,
            Err(e) => {
                return Err(McpError {
                    code: ErrorCode(-32603),
                    message: Cow::from(format!(
                        "Failed to parse providers response as JSON: {}",
                        e
                    )),
                    data: None,
                });
            }
        };

        let mut response_content = Vec::new();

        response_content.push(Content::text(format!(
            "Found {} providers available:",
            providers_response.providers.len()
        )));

        for provider in &providers_response.providers {
            let provider_info = format!(
                "\n## {}\nID: {}\nChat Models: {}\nEmbedding Models: {}",
                provider.name,
                provider.id,
                provider.chat_models.len(),
                provider.embedding_models.len()
            );
            response_content.push(Content::text(provider_info));
        }

        let complete_response_json =
            serde_json::to_string_pretty(&providers_response).map_err(|e| McpError {
                code: ErrorCode(-32603),
                message: Cow::from(format!("Failed to serialize providers data: {}", e)),
                data: None,
            })?;

        response_content.push(Content::text(format!(
            "\n\n## Complete Response (JSON)\n\n```json\n{}\n```",
            complete_response_json
        )));

        Ok(CallToolResult::success(response_content))
    }
}

#[tool_handler]
impl ServerHandler for PerplexicaService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "A Perplexica API service that performs intelligent searches. Use perplexica_providers to discover available providers and models, and perplexica_search to query the Perplexica instance. Required environment variables: PERPLEXICA_API_URL. Optional environment variables for defaults: PERPLEXICA_PROVIDER_ID, PERPLEXICA_CHAT_MODEL_KEY, PERPLEXICA_EMBEDDING_MODEL_KEY.".to_string(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_search_response() {
        let json_data = r#"
        {
            "message": "This is a test search response with citations [1][2].",
            "sources": [
                {
                    "pageContent": "Test content 1",
                    "metadata": {
                        "title": "Test Title 1",
                        "url": "https://example.com/1"
                    }
                },
                {
                    "pageContent": "Test content 2",
                    "metadata": {
                        "title": "Test Title 2",
                        "url": "https://example.com/2"
                    }
                }
            ]
        }
        "#;

        let response: PerplexicaSearchResponse = serde_json::from_str(json_data).unwrap();

        assert_eq!(
            response.message,
            "This is a test search response with citations [1][2]."
        );
        assert_eq!(response.sources.len(), 2);
        assert_eq!(response.sources[0].metadata.title, "Test Title 1");
        assert_eq!(response.sources[1].metadata.url, "https://example.com/2");
    }

    #[test]
    fn test_deserialize_providers_response() {
        let json_data = r#"
        {
            "providers": [
                {
                    "id": "test-provider-1",
                    "name": "Test Provider 1",
                    "chatModels": [
                        {
                            "name": "GPT 4",
                            "key": "gpt-4"
                        }
                    ],
                    "embeddingModels": [
                        {
                            "name": "Text Embedding 3 Large",
                            "key": "text-embedding-3-large"
                        }
                    ]
                }
            ]
        }
        "#;

        let response: ProvidersResponse = serde_json::from_str(json_data).unwrap();

        assert_eq!(response.providers.len(), 1);
        assert_eq!(response.providers[0].id, "test-provider-1");
        assert_eq!(response.providers[0].name, "Test Provider 1");
        assert_eq!(response.providers[0].chat_models.len(), 1);
        assert_eq!(response.providers[0].chat_models[0].key, "gpt-4");
        assert_eq!(
            response.providers[0].embedding_models[0].name,
            "Text Embedding 3 Large"
        );
    }

    #[test]
    fn test_deserialize_search_request() {
        let json_data = r#"
        {
            "query": "What is AI?",
            "focus_mode": "webSearch",
            "stream": false,
            "history": [
                ["human", "Hi"],
                ["assistant", "Hello"]
            ],
            "system_instructions": "Be helpful",
            "provider_id": "test-provider",
            "chat_model_key": "gpt-4",
            "embedding_model_key": "text-embedding-3-large"
        }
        "#;

        let request: PerplexicaSearchRequest = serde_json::from_str(json_data).unwrap();

        assert_eq!(request.query, "What is AI?");
        assert_eq!(request.focus_mode, "webSearch");
        assert!(!request.stream);
        assert_eq!(request.history.unwrap().len(), 2);
        assert_eq!(request.system_instructions.unwrap(), "Be helpful");
        assert_eq!(request.provider_id.unwrap(), "test-provider");
        assert_eq!(request.chat_model_key.unwrap(), "gpt-4");
        assert_eq!(
            request.embedding_model_key.unwrap(),
            "text-embedding-3-large"
        );
    }

    #[test]
    fn test_default_values() {
        let json_data = r#"
        {
            "query": "Test query"
        }
        "#;

        let request: PerplexicaSearchRequest = serde_json::from_str(json_data).unwrap();

        assert_eq!(request.query, "Test query");
        assert_eq!(request.focus_mode, "webSearch");
        assert!(!request.stream);
        assert!(request.history.is_none());
        assert!(request.system_instructions.is_none());
        assert!(request.provider_id.is_none());
        assert!(request.chat_model_key.is_none());
        assert!(request.embedding_model_key.is_none());
    }

    #[test]
    fn test_markdown_formatting() {
        let search_response = PerplexicaSearchResponse {
            message: "This is a test search response with citations [1][2].".to_string(),
            sources: vec![
                Source {
                    page_content: "Test content 1".to_string(),
                    metadata: SourceMetadata {
                        title: "Test Title 1".to_string(),
                        url: "https://example.com/1".to_string(),
                    },
                },
                Source {
                    page_content: "Test content 2".to_string(),
                    metadata: SourceMetadata {
                        title: "Test Title 2".to_string(),
                        url: "https://example.com/2".to_string(),
                    },
                },
            ],
        };

        let mut markdown = String::new();

        markdown.push_str("## Summary\n\n");
        markdown.push_str(&search_response.message);
        markdown.push_str("\n\n");

        markdown.push_str("## Sources\n\n");

        if search_response.sources.is_empty() {
            markdown.push_str("No sources found.\n");
        } else {
            for source in &search_response.sources {
                markdown.push_str("- ");
                markdown.push_str(&source.metadata.title);
                markdown.push_str("\n  - ");
                markdown.push_str(&source.metadata.url);
                markdown.push('\n');
            }
        }

        let expected = r#"## Summary

This is a test search response with citations [1][2].

## Sources

- Test Title 1
  - https://example.com/1
- Test Title 2
  - https://example.com/2
"#;

        assert_eq!(markdown, expected);
    }

    #[test]
    fn test_markdown_formatting_no_sources() {
        let search_response = PerplexicaSearchResponse {
            message: "No sources found for this query.".to_string(),
            sources: vec![],
        };

        let mut markdown = String::new();

        markdown.push_str("## Summary\n\n");
        markdown.push_str(&search_response.message);
        markdown.push_str("\n\n");

        markdown.push_str("## Sources\n\n");

        if search_response.sources.is_empty() {
            markdown.push_str("No sources found.\n");
        } else {
            for source in &search_response.sources {
                markdown.push_str("- ");
                markdown.push_str(&source.metadata.title);
                markdown.push_str("\n  - ");
                markdown.push_str(&source.metadata.url);
                markdown.push('\n');
            }
        }

        let expected = r#"## Summary

No sources found for this query.

## Sources

No sources found.
"#;

        assert_eq!(markdown, expected);
    }
}
