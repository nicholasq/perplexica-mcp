# Perplexica MCP Server

A Model Context Protocol (MCP) server that provides access to Perplexica's intelligent search capabilities.

## Features

- **Intelligent Search**: Query Perplexica's AI-powered search engine
- **Multiple Focus Modes**: Support for web search, academic search, and more
- **Chat History**: Maintain conversation context across searches
- **System Instructions**: Customize search behavior with custom instructions

## Installation

### From Source

```bash
git clone <repository-url>
cd perplexica-mcp
cargo install --path .
```

### Dependencies

- Rust 1.70+
- A running Perplexica instance

## Configuration

### Required Environment Variables

Set `PERPLEXICA_API_URL` environment variable to point to your Perplexica instance:

```bash
export PERPLEXICA_API_URL="http://localhost:3000"
# or
export PERPLEXICA_API_URL="https://your-perplexica-instance.com"
```

### Optional Environment Variables (Recommended)

You can set default provider and model values to avoid specifying them in each search request:

```bash
export PERPLEXICA_PROVIDER_ID="550e8400-e29b-41d4-a716-446655440000"
export PERPLEXICA_CHAT_MODEL_KEY="gpt-4o-mini"
export PERPLEXICA_EMBEDDING_MODEL_KEY="text-embedding-3-large"
```

**Note**: These are optional. If not set, the client will need to specify provider information in each search request.

## Usage

### With Zed Editor

Add to your `settings.json` file:

```json
{
  "context_servers": {
    "perplexica": {
      "source": "custom",
      "command": "perplexica-mcp",
      "env": {
        "PERPLEXICA_API_URL": "http://localhost:3000",
        "PERPLEXICA_CHAT_MODEL_KEY": "models/gemini-2.5-flash",
        "PERPLEXICA_PROVIDER_ID": "40c45540-9774-42c7-b38c-7934039f73f1",
        "PERPLEXICA_EMBEDDING_MODEL_KEY": "models/embedding-gemini-001"
      }
    }
  }
}
```

#### Development Mode in Zed

For development with Zed, you can run directly from the source:

```json
{
  "context_servers": {
    "perplexica-dev": {
      "source": "custom",
      "command": "cargo",
      "args": ["run"],
      "env": {
        "PERPLEXICA_API_URL": "http://localhost:3000",
        "PERPLEXICA_CHAT_MODEL_KEY": "models/gemini-2.5-flash",
        "PERPLEXICA_PROVIDER_ID": "40c45540-9774-42c7-b38c-7934039f73f1",
        "PERPLEXICA_EMBEDDING_MODEL_KEY": "models/embedding-gemini-001"
      }
    }
  }
}
```

### With Opencode

Add to your `opencode.json` file (usually located at `~/.config/opencode/opencode.json`):

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "perplexica": {
      "type": "local",
      "command": ["perplexica-mcp"],
      "enabled": true,
      "environment": {
        "PERPLEXICA_API_URL": "http://localhost:3000",
        "PERPLEXICA_CHAT_MODEL_KEY": "models/gemini-2.5-flash",
        "PERPLEXICA_PROVIDER_ID": "40c45540-9774-42c7-b38c-7934039f73f1",
        "PERPLEXICA_EMBEDDING_MODEL_KEY": "models/embedding-gemini-001"
      }
    }
  }
}
```

#### Development Mode in Opencode

For development with Opencode, you can run directly from the source:

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "perplexica-dev": {
      "type": "local",
      "command": ["cargo", "run"],
      "cwd": "/path/to/perplexica-mcp",
      "enabled": true,
      "environment": {
        "PERPLEXICA_API_URL": "http://localhost:3000",
        "PERPLEXICA_CHAT_MODEL_KEY": "models/gemini-2.5-flash",
        "PERPLEXICA_PROVIDER_ID": "40c45540-9774-42c7-b38c-7934039f73f1",
        "PERPLEXICA_EMBEDDING_MODEL_KEY": "models/embedding-gemini-001"
      }
    }
  }
}
```

## Available Tools

### `perplexica_providers`

Retrieve available providers and their models from [Perplexica API](https://github.com/ItzCrazyKns/Perplexica/blob/master/docs/API/SEARCH.md).

**Parameters:** None

**Usage:** Call this tool to discover available providers, their chat models, and embedding models. This helps the client understand what provider IDs and model keys are available for use in search requests.

**Response Format:** The tool returns a structured response with:
- A summary of available providers
- Detailed information about each provider (name, ID, model counts)
- Complete JSON response for programmatic access

### `perplexica_search`

Search using [Perplexica API](https://github.com/ItzCrazyKns/Perplexica/blob/master/docs/API/SEARCH.md). Provider and model parameters are optional - the server will use configured defaults unless the user explicitly specifies otherwise.

**Response Format:** The tool returns formatted markdown with:
- A summary section containing the main search response
- A sources section listing all referenced sources with their titles and URLs

**Markdown Output Format:**
```markdown
## Summary

<content of the message property>

## Sources

- <title property of source object>
  - <url property of source object>
```

**JSON Deserialization:** The server properly deserializes the Perplexica API JSON responses into structured Rust data types (`PerplexicaSearchResponse`, `ProvidersResponse`, etc.) internally before formatting them as markdown for MCP clients. This ensures type safety and proper error handling while providing a clean, readable output format.

**Parameters:**

- `query` (required): The search query to send to Perplexica
- `focus_mode` (optional): The focus mode for search (default: "webSearch")
- `stream` (optional): Whether to stream response (default: false)
- `history` (optional): Chat history as array of [role, message] pairs
- `system_instructions` (optional): System instructions for search
- `provider_id` (optional): Provider ID to use. DO NOT SET unless user explicitly specifies a provider. Will use default from environment variables if omitted.
- `chat_model_key` (optional): Chat model key to use. DO NOT SET unless user explicitly specifies a model. Will use default from environment variables if omitted.
- `embedding_model_key` (optional): Embedding model key to use. DO NOT SET unless user explicitly specifies an embedding model. Will use default from environment variables if omitted.

**Example Usage:**

```json
{
  "query": "What is artificial intelligence?",
  "focus_mode": "webSearch",
  "system_instructions": "Focus on technical details and recent developments"
}
```

**Example with explicit provider selection:**

```json
{
  "query": "What is artificial intelligence?",
  "provider_id": "550e8400-e29b-41d4-a716-446655440000",
  "chat_model_key": "gpt-4o",
  "embedding_model_key": "text-embedding-3-large"
}
```

## API Compatibility

This MCP server is compatible with Perplexica instances that support:

### `/api/providers` endpoint

Returns a list of all active providers with their available chat and embedding models.

### `/api/search` endpoint

Accepts search requests with the following format:

```json
{
  "chatModel": {
    "providerId": "uuid",
    "key": "model-name"
  },
  "embeddingModel": {
    "providerId": "uuid", 
    "key": "embedding-model"
  },
  "optimizationMode": "speed",
  "focusMode": "webSearch",
  "query": "search query",
  "history": [...],
  "systemInstructions": "...",
  "stream": false
}
```

## Development

### Building

```bash
cargo build
```

### Running Tests

```bash
cargo test
```

### Checking Code

```bash
cargo check
cargo clippy
```

## License

MIT License

## Troubleshooting

### Environment Variable Not Set

```
Error: PERPLEXICA_API_URL environment variable must be set
```

Solution: Set the environment variable before running the server.

### Missing Provider Information

```
Error: Missing provider_id. Set either provider_id parameter or PERPLEXICA_PROVIDER_ID environment variable
```

Solution: Either set the optional environment variables for defaults, or specify provider information in the search request.

### Connection Refused

Make sure your Perplexica instance is running and accessible at the specified URL.

### MCP Protocol Errors

Ensure your MCP client (Zed, OpenCode etc.) is properly configured to use stdio transport.
