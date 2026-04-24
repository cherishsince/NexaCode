//! Agent Controller
//!
//! This module implements the core agent controller that manages the state machine
//! and orchestrates the interaction between the user, LLM, and tools.

use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, info, warn};

use crate::{Message, MessageRole, Store};
use crate::infra::llm::config::LlmConfig;
use crate::infra::llm::trait_def::{HttpLlmClient, LlmClient};
use crate::infra::llm::types::{LlmRequest, LlmResponse};
use crate::infra::llm::StreamCallback;
use super::context::{ContextManager, ContextConfig, MessagePriority};

// ============================================================================
// Agent State Machine
// ============================================================================

/// Agent state enum representing the different states of the agent
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentStateEnum {
    /// Agent is idle, waiting for user input
    Idle,
    /// Agent is thinking (processing with LLM)
    Thinking,
    /// Agent is executing a tool
    ExecutingTool,
    /// Agent is streaming response to the user
    StreamingResponse,
    /// Agent encountered an error
    Error,
}

impl Default for AgentStateEnum {
    fn default() -> Self {
        Self::Idle
    }
}

impl std::fmt::Display for AgentStateEnum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "Idle"),
            Self::Thinking => write!(f, "Thinking"),
            Self::ExecutingTool => write!(f, "Executing Tool"),
            Self::StreamingResponse => write!(f, "Streaming"),
            Self::Error => write!(f, "Error"),
        }
    }
}

// ============================================================================
// Agent Events
// ============================================================================

/// Events that can be emitted by the agent
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// State changed
    StateChanged(AgentStateEnum),
    /// Received user message
    UserMessage(String),
    /// LLM response chunk (streaming)
    ResponseChunk(String),
    /// Response completed
    ResponseComplete(String),
    /// Tool execution started
    ToolExecutionStarted { name: String, arguments: serde_json::Value },
    /// Tool execution completed
    ToolExecutionCompleted { name: String, result: serde_json::Value },
    /// Error occurred
    Error(String),
    /// Agent is ready for input
    Ready,
}

// ============================================================================
// Tool Definition
// ============================================================================

/// Tool definition for function calling
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Tool execution result
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub tool_name: String,
    pub result: serde_json::Value,
    pub is_error: bool,
}

// ============================================================================
// Agent Controller
// ============================================================================

/// Agent Controller - the main orchestrator
pub struct AgentController {
    /// Current state
    state: Arc<RwLock<AgentStateEnum>>,
    /// Context management (using new ContextManager)
    context: Arc<Mutex<ContextManager>>,
    /// Available tools
    tools: Vec<ToolDefinition>,
    /// LLM configuration
    config: LlmConfig,
    /// LLM client
    llm_client: Arc<dyn LlmClient>,
    /// Store reference for state management
    store: Option<Arc<Store>>,
    /// Event sender
    event_tx: Option<mpsc::Sender<AgentEvent>>,
    /// Tool executor callback
    tool_executor: Option<Arc<dyn ToolExecutor + Send + Sync>>,
}

/// Trait for tool execution (to be implemented by the tool system)
#[async_trait::async_trait]
pub trait ToolExecutor {
    async fn execute(&self, name: &str, arguments: serde_json::Value) -> ToolResult;
}

impl AgentController {
    /// Create a new agent controller
    pub fn new(config: LlmConfig) -> Self {
        let context_config = ContextConfig {
            max_tokens: 200_000,
            ..Default::default()
        };
        
        let llm_client = Arc::new(HttpLlmClient::new(config.clone()));
        
        Self {
            state: Arc::new(RwLock::new(AgentStateEnum::default())),
            context: Arc::new(Mutex::new(ContextManager::new(context_config))),
            tools: Vec::new(),
            config,
            llm_client,
            store: None,
            event_tx: None,
            tool_executor: None,
        }
    }

    /// Set the store reference
    pub fn with_store(mut self, store: Arc<Store>) -> Self {
        self.store = Some(store);
        self
    }

    /// Set the event channel
    pub fn with_event_channel(mut self, tx: mpsc::Sender<AgentEvent>) -> Self {
        self.event_tx = Some(tx);
        self
    }

    /// Set the tool executor
    pub fn with_tool_executor(mut self, executor: Arc<dyn ToolExecutor + Send + Sync>) -> Self {
        self.tool_executor = Some(executor);
        self
    }

    /// Set system prompt
    pub async fn set_system_prompt(&self, prompt: String) {
        let mut ctx = self.context.lock().await;
        ctx.set_system_prompt(prompt);
    }

    /// Add a tool
    pub fn add_tool(&mut self, tool: ToolDefinition) {
        self.tools.push(tool);
    }

    /// Get current state
    pub async fn get_state(&self) -> AgentStateEnum {
        *self.state.read().await
    }

    /// Get context statistics
    pub async fn get_context_stats(&self) -> super::context::ContextStats {
        let ctx = self.context.lock().await;
        ctx.stats()
    }

    /// Set state and emit event
    async fn set_state(&self, new_state: AgentStateEnum) {
        let mut state = self.state.write().await;
        if *state != new_state {
            debug!("Agent state changed: {:?} -> {:?}", *state, new_state);
            *state = new_state;
            
            // Emit state change event
            if let Some(tx) = &self.event_tx {
                let _ = tx.send(AgentEvent::StateChanged(new_state)).await;
            }
        }
    }

    /// Emit an event
    async fn emit(&self, event: AgentEvent) {
        if let Some(tx) = &self.event_tx {
            let _ = tx.send(event).await;
        }
    }

    /// Process a user message - the main entry point
    /// Returns the assistant's response text
    pub async fn process_user_message(&self, content: String) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        info!("Processing user message: {}", content);
        
        // Create user message
        let user_message = Message::new(MessageRole::User, content.clone());
        
        // Add to context with normal priority
        {
            let mut ctx = self.context.lock().await;
            ctx.add_message(user_message.clone());
        }
        
        // Emit user message event
        self.emit(AgentEvent::UserMessage(content.clone())).await;
        
        // Run the reasoning loop and return the response
        self.reasoning_loop().await
    }

    /// Process a user message with streaming callback
    /// Returns the assistant's response text
    pub async fn process_user_message_stream(
        &self, 
        content: String, 
        on_chunk: StreamCallback
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        info!("Processing user message with streaming: {}", content);
        
        // Create user message
        let user_message = Message::new(MessageRole::User, content.clone());
        
        // Add to context with normal priority
        {
            let mut ctx = self.context.lock().await;
            ctx.add_message(user_message.clone());
        }
        
        // Emit user message event
        self.emit(AgentEvent::UserMessage(content.clone())).await;
        
        // Run the reasoning loop with streaming and return the response
        self.reasoning_loop_stream(on_chunk).await
    }

    /// Core reasoning loop
    /// Returns the final assistant response
    async fn reasoning_loop(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        loop {
            // Check context budget - ContextManager handles pruning automatically
            {
                let ctx = self.context.lock().await;
                if !ctx.is_within_budget() {
                    warn!("Context budget exceeded, pruning should have occurred");
                }
            }

            // Set state to thinking
            self.set_state(AgentStateEnum::Thinking).await;
            
            // Call LLM
            let response = self.call_llm().await?;
            
            // Process the response
            match response {
                LlmResponse::Text(text) => {
                    // Stream the response
                    self.set_state(AgentStateEnum::StreamingResponse).await;
                    self.stream_response(&text).await;
                    
                    // Add assistant message to context
                    let assistant_message = Message::new(MessageRole::Assistant, text.clone());
                    {
                        let mut ctx = self.context.lock().await;
                        ctx.add_message(assistant_message);
                    }
                    
                    // Done, return to idle
                    self.set_state(AgentStateEnum::Idle).await;
                    return Ok(text);
                }
                
                LlmResponse::ToolCall { name, arguments } => {
                    // Execute the tool
                    self.set_state(AgentStateEnum::ExecutingTool).await;
                    
                    self.emit(AgentEvent::ToolExecutionStarted {
                        name: name.clone(),
                        arguments: arguments.clone(),
                    }).await;
                    
                    let result = self.execute_tool(&name, arguments).await;
                    
                    self.emit(AgentEvent::ToolExecutionCompleted {
                        name: name.clone(),
                        result: result.result.clone(),
                    }).await;
                    
                    // Add tool result to context with high priority
                    let tool_message = Message::new(MessageRole::Tool, result.result.to_string());
                    {
                        let mut ctx = self.context.lock().await;
                        ctx.add_message_with_priority(tool_message, MessagePriority::High);
                    }
                    
                    // Continue the loop for next turn
                    continue;
                }
                
                LlmResponse::Error(err) => {
                    self.set_state(AgentStateEnum::Error).await;
                    self.emit(AgentEvent::Error(err.clone())).await;
                    return Err(err.into());
                }
            }
        }
    }

    /// Core reasoning loop with streaming
    /// Returns the final assistant response
    async fn reasoning_loop_stream(&self, on_chunk: StreamCallback) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Check context budget
        {
            let ctx = self.context.lock().await;
            if !ctx.is_within_budget() {
                warn!("Context budget exceeded, pruning should have occurred");
            }
        }

        // Set state to thinking
        self.set_state(AgentStateEnum::Thinking).await;
        
        // Call LLM with streaming
        let text = self.call_llm_stream(on_chunk).await?;
        
        // Add assistant message to context
        let assistant_message = Message::new(MessageRole::Assistant, text.clone());
        {
            let mut ctx = self.context.lock().await;
            ctx.add_message(assistant_message);
        }
        
        // Done, return to idle
        self.set_state(AgentStateEnum::Idle).await;
        Ok(text)
    }

    /// Call the LLM
    async fn call_llm(&self) -> Result<LlmResponse, Box<dyn std::error::Error + Send + Sync>> {
        // Get messages for LLM from ContextManager
        let messages = {
            let ctx = self.context.lock().await;
            ctx.get_messages_for_llm()
        };
        
        // Build request based on provider
        let request = LlmRequest {
            provider_name: self.config.current_provider_name().to_string(),
            model: self.config.current_model(),
            messages,
            max_tokens: self.config.max_tokens,
            temperature: Some(self.config.temperature),
            tools: self.tools.clone(),
        };
        
        debug!("Calling {} LLM with model: {}", request.provider_name, request.model);
        
        // Use the LLM client to make the actual API call
        self.llm_client.call(request).await.map_err(|e| e.into())
    }

    /// Call the LLM with streaming
    async fn call_llm_stream(&self, on_chunk: StreamCallback) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Get messages for LLM from ContextManager
        let messages = {
            let ctx = self.context.lock().await;
            ctx.get_messages_for_llm()
        };
        
        // Build request based on provider
        let request = LlmRequest {
            provider_name: self.config.current_provider_name().to_string(),
            model: self.config.current_model(),
            messages,
            max_tokens: self.config.max_tokens,
            temperature: Some(self.config.temperature),
            tools: Vec::new(), // Simplified for streaming
        };
        
        debug!("Calling {} LLM with streaming, model: {}", request.provider_name, request.model);
        
        // Use the LLM client to make the streaming API call
        self.llm_client.call_stream(request, on_chunk).await.map_err(|e| e.into())
    }

    /// Stream response to the user
    async fn stream_response(&self, text: &str) {
        // Simulate streaming by chunks
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut accumulated = String::new();
        
        for word in words {
            accumulated.push_str(word);
            accumulated.push(' ');
            
            self.emit(AgentEvent::ResponseChunk(format!("{} ", word))).await;
            
            // Small delay for visual effect
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }
        
        self.emit(AgentEvent::ResponseComplete(accumulated.trim().to_string())).await;
    }

    /// Execute a tool
    async fn execute_tool(&self, name: &str, arguments: serde_json::Value) -> ToolResult {
        if let Some(executor) = &self.tool_executor {
            executor.execute(name, arguments).await
        } else {
            ToolResult {
                tool_name: name.to_string(),
                result: serde_json::json!({"error": "No tool executor configured"}),
                is_error: true,
            }
        }
    }

    /// Reset the agent
    pub async fn reset(&self) {
        self.set_state(AgentStateEnum::Idle).await;
        
        let mut ctx = self.context.lock().await;
        ctx.clear();
        
        self.emit(AgentEvent::Ready).await;
    }

    /// Stop the agent (cancel current operation)
    pub async fn stop(&self) {
        self.set_state(AgentStateEnum::Idle).await;
        self.emit(AgentEvent::Ready).await;
    }
}

// ============================================================================
// Stream Event for async iteration
// ============================================================================

/// Stream events from LLM
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// Text delta
    TextDelta(String),
    /// Tool call started
    ToolCallStart { name: String },
    /// Tool call arguments chunk
    ToolCallChunk(String),
    /// Stream completed
    Complete,
    /// Error
    Error(String),
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_state_display() {
        assert_eq!(AgentStateEnum::Idle.to_string(), "Idle");
        assert_eq!(AgentStateEnum::Thinking.to_string(), "Thinking");
        assert_eq!(AgentStateEnum::ExecutingTool.to_string(), "Executing Tool");
        assert_eq!(AgentStateEnum::StreamingResponse.to_string(), "Streaming");
        assert_eq!(AgentStateEnum::Error.to_string(), "Error");
    }

    #[tokio::test]
    async fn test_agent_controller_creation() {
        let config = LlmConfig::default();
        let agent = AgentController::new(config);
        
        assert_eq!(agent.get_state().await, AgentStateEnum::Idle);
    }

    #[tokio::test]
    async fn test_context_stats() {
        let config = LlmConfig::default();
        let agent = AgentController::new(config);
        
        let stats = agent.get_context_stats().await;
        assert_eq!(stats.message_count, 0);
        assert!(stats.remaining_tokens > 0);
    }
}
