//! LLM client trait and implementation

use anyhow::{Context, Result};
use reqwest::Client;
use tokio_stream::StreamExt;
use tracing::{debug, info};

use super::types::*;
use super::config::{LlmConfig, ProviderType};

/// Callback type for streaming responses
pub type StreamCallback = Box<dyn Fn(&str) + Send + Sync>;

/// LLM Client trait
#[async_trait::async_trait]
pub trait LlmClient: Send + Sync {
    /// Call the LLM with a request (non-streaming)
    async fn call(&self, request: LlmRequest) -> Result<LlmResponse>;
    
    /// Call the LLM with streaming response
    async fn call_stream(
        &self,
        request: LlmRequest,
        on_chunk: StreamCallback,
    ) -> Result<String>;
}

/// HTTP-based LLM client
pub struct HttpLlmClient {
    config: LlmConfig,
    http_client: Client,
}

impl HttpLlmClient {
    /// Create a new HTTP LLM client
    pub fn new(config: LlmConfig) -> Self {
        Self {
            config,
            http_client: Client::builder()
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Get the provider type
    fn provider_type(&self) -> ProviderType {
        self.config.provider_type()
    }

    /// Call OpenAI-compatible API with streaming
    async fn call_openai_stream(
        &self,
        request: LlmRequest,
        base_url: &str,
        api_key: &str,
        on_chunk: StreamCallback,
    ) -> Result<String> {
        let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
        
        debug!("Calling OpenAI-compatible API with streaming: {}", url);
        
        // Build OpenAI request with streaming enabled
        let openai_request = OpenAIRequest {
            model: request.model.clone(),
            messages: request.messages.iter().map(|m| OpenAIMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            }).collect(),
            max_tokens: Some(request.max_tokens),
            temperature: request.temperature,
            stream: Some(true),
            tools: Vec::new(), // Simplified for streaming
        };

        let response = self.http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&openai_request)
            .send()
            .await
            .context("Failed to send request to LLM API")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("LLM API error: {} - {}", status, body);
        }

        // Process SSE stream
        let mut full_content = String::new();
        let mut stream = response.bytes_stream();
        let mut chunk_count = 0;
        let mut buffer = String::new(); // Buffer for incomplete lines
        
        info!("Starting to process SSE stream");
        
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.context("Failed to read stream chunk")?;
            let chunk_str = String::from_utf8_lossy(&chunk);
            chunk_count += 1;
            
            debug!("Received chunk {}: {} bytes", chunk_count, chunk.len());
            
            // Add to buffer
            buffer.push_str(&chunk_str);
            
            // Debug: log first few chunks to see raw data
            if chunk_count <= 3 {
                debug!("Raw chunk {} content: {}", chunk_count, &chunk_str.chars().take(500).collect::<String>());
            }
            
            // Process complete lines from buffer
            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].trim().to_string();
                buffer = buffer[newline_pos + 1..].to_string();
                
                if line.starts_with("data: ") {
                    let data = &line[6..];
                    if data == "[DONE]" {
                        info!("Stream completed with [DONE]");
                        continue;
                    }
                    
                    // Parse the JSON
                    match serde_json::from_str::<OpenAIStreamResponse>(data) {
                        Ok(stream_response) => {
                            if let Some(choice) = stream_response.choices.first() {
                                if let Some(delta) = &choice.delta {
                                    // Handle both content and reasoning_content
                                    // Some providers like Huawei/GLM use reasoning_content for chain-of-thought
                                    // Priority: non-empty content > non-empty reasoning_content
                                    let content_to_send = delta.content.as_ref()
                                        .filter(|c| !c.is_empty())
                                        .or_else(|| delta.reasoning_content.as_ref()
                                            .filter(|c| !c.is_empty()));
                                    
                                    if let Some(content) = content_to_send {
                                        debug!("Sending chunk: {} chars", content.len());
                                        on_chunk(content);
                                        full_content.push_str(content);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            debug!("Failed to parse stream response: {} - data: {}", e, data);
                        }
                    }
                }
            }
        }
        
        info!("Stream completed: {} chunks, {} total chars", chunk_count, full_content.len());

        if full_content.is_empty() {
            anyhow::bail!("No content received from streaming response");
        }

        Ok(full_content)
    }

    /// Call OpenAI-compatible API (non-streaming)
    async fn call_openai_compatible(
        &self,
        request: LlmRequest,
        base_url: &str,
        api_key: &str,
    ) -> Result<LlmResponse> {
        let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
        
        debug!("Calling OpenAI-compatible API: {}", url);
        
        // Build OpenAI request
        let openai_request = OpenAIRequest {
            model: request.model.clone(),
            messages: request.messages.iter().map(|m| OpenAIMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            }).collect(),
            max_tokens: Some(request.max_tokens),
            temperature: request.temperature,
            stream: None,
            tools: request.tools.iter().map(|t| OpenAITool {
                tool_type: "function".to_string(),
                function: OpenAIFunction {
                    name: t.name.clone(),
                    description: t.description.clone(),
                    parameters: t.parameters.clone(),
                },
            }).collect(),
        };

        let response = self.http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&openai_request)
            .send()
            .await
            .context("Failed to send request to LLM API")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("LLM API error: {} - {}", status, body);
        }

        let openai_response: OpenAIResponse = response
            .json()
            .await
            .context("Failed to parse LLM API response")?;

        // Extract response
        if let Some(choice) = openai_response.choices.first() {
            // Check for tool calls
            if !choice.message.tool_calls.is_empty() {
                let tool_call = &choice.message.tool_calls[0];
                let arguments: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
                    .unwrap_or(serde_json::Value::Object(Default::default()));
                
                return Ok(LlmResponse::ToolCall {
                    name: tool_call.function.name.clone(),
                    arguments,
                });
            }

            // Return text response
            let content = choice.message.content.clone().unwrap_or_default();
            Ok(LlmResponse::Text(content))
        } else {
            anyhow::bail!("No response choices from LLM API");
        }
    }

    /// Call Anthropic API with streaming
    async fn call_anthropic_stream(
        &self,
        request: LlmRequest,
        base_url: &str,
        api_key: &str,
        on_chunk: StreamCallback,
    ) -> Result<String> {
        let url = format!("{}/v1/messages", base_url.trim_end_matches('/'));
        
        debug!("Calling Anthropic API with streaming: {}", url);
        
        // Separate system message from other messages
        let mut system_prompt = None;
        let messages: Vec<AnthropicMessage> = request.messages.iter()
            .filter_map(|m| {
                if m.role == "system" {
                    system_prompt = Some(m.content.clone());
                    None
                } else {
                    Some(AnthropicMessage {
                        role: m.role.clone(),
                        content: m.content.clone(),
                    })
                }
            })
            .collect();

        let anthropic_request = AnthropicRequest {
            model: request.model.clone(),
            messages,
            max_tokens: Some(request.max_tokens),
            system: system_prompt,
            stream: Some(true),
        };

        let response = self.http_client
            .post(&url)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&anthropic_request)
            .send()
            .await
            .context("Failed to send request to Anthropic API")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Anthropic API error: {} - {}", status, body);
        }

        // Process SSE stream
        let mut full_content = String::new();
        let mut stream = response.bytes_stream();
        
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.context("Failed to read stream chunk")?;
            let chunk_str = String::from_utf8_lossy(&chunk);
            
            // Parse SSE data
            for line in chunk_str.lines() {
                if line.starts_with("data: ") {
                    let data = &line[6..];
                    
                    // Parse the JSON
                    if let Ok(event) = serde_json::from_str::<AnthropicStreamEvent>(data) {
                        if event.event_type == "content_block_delta" {
                            if let Some(delta) = event.delta {
                                if delta.delta_type == "text_delta" {
                                    if let Some(text) = delta.text {
                                        on_chunk(&text);
                                        full_content.push_str(&text);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if full_content.is_empty() {
            anyhow::bail!("No content received from streaming response");
        }

        Ok(full_content)
    }

    /// Call Anthropic API (non-streaming)
    async fn call_anthropic(
        &self,
        request: LlmRequest,
        base_url: &str,
        api_key: &str,
    ) -> Result<LlmResponse> {
        let url = format!("{}/v1/messages", base_url.trim_end_matches('/'));
        
        debug!("Calling Anthropic API: {}", url);
        
        // Separate system message from other messages
        let mut system_prompt = None;
        let messages: Vec<AnthropicMessage> = request.messages.iter()
            .filter_map(|m| {
                if m.role == "system" {
                    system_prompt = Some(m.content.clone());
                    None
                } else {
                    Some(AnthropicMessage {
                        role: m.role.clone(),
                        content: m.content.clone(),
                    })
                }
            })
            .collect();

        let anthropic_request = AnthropicRequest {
            model: request.model.clone(),
            messages,
            max_tokens: Some(request.max_tokens),
            system: system_prompt,
            stream: None,
        };

        let response = self.http_client
            .post(&url)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&anthropic_request)
            .send()
            .await
            .context("Failed to send request to Anthropic API")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Anthropic API error: {} - {}", status, body);
        }

        let anthropic_response: AnthropicResponse = response
            .json()
            .await
            .context("Failed to parse Anthropic API response")?;

        // Extract text from content blocks
        let text = anthropic_response.content.iter()
            .filter_map(|c| {
                if c.content_type == "text" {
                    c.text.clone()
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        if text.is_empty() {
            anyhow::bail!("No text content in Anthropic response");
        }

        Ok(LlmResponse::Text(text))
    }
}

#[async_trait::async_trait]
impl LlmClient for HttpLlmClient {
    async fn call(&self, request: LlmRequest) -> Result<LlmResponse> {
        let provider_type = self.provider_type();
        let base_url = self.config.current_base_url();
        let api_key = self.config.current_api_key();

        if api_key.is_empty() {
            anyhow::bail!("No API key configured for provider: {}", self.config.current_provider_name());
        }

        info!("Calling LLM: provider={}, model={}", self.config.current_provider_name(), request.model);

        match provider_type {
            ProviderType::Anthropic => {
                self.call_anthropic(request, &base_url, &api_key).await
            }
            ProviderType::OpenAI => {
                self.call_openai_compatible(request, &base_url, &api_key).await
            }
        }
    }

    async fn call_stream(
        &self,
        request: LlmRequest,
        on_chunk: StreamCallback,
    ) -> Result<String> {
        let provider_type = self.provider_type();
        let base_url = self.config.current_base_url();
        let api_key = self.config.current_api_key();

        if api_key.is_empty() {
            anyhow::bail!("No API key configured for provider: {}", self.config.current_provider_name());
        }

        info!("Calling LLM with streaming: provider={}, model={}", self.config.current_provider_name(), request.model);

        match provider_type {
            ProviderType::Anthropic => {
                self.call_anthropic_stream(request, &base_url, &api_key, on_chunk).await
            }
            ProviderType::OpenAI => {
                self.call_openai_stream(request, &base_url, &api_key, on_chunk).await
            }
        }
    }
}
