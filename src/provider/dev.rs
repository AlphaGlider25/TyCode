use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::provider::{Message, Provider, StopReason, StreamEvent, TokenUsage};
use crate::tools::ToolSchema;

use super::ProviderResponse;

pub struct DevProvider;

#[async_trait]
impl Provider for DevProvider {
    async fn chat(
        &self,
        messages: &[Message],
        _tools: &[ToolSchema],
        delta_tx: Option<mpsc::UnboundedSender<StreamEvent>>,
    ) -> Result<ProviderResponse> {
        let response_text =
            "This is a test response to allow developers to test functionality without acting on actions.";

        // Stream the response if delta_tx is provided
        if let Some(tx) = delta_tx {
            let _ = tx.send(StreamEvent::TextDelta(response_text.to_string()));
            let _ = tx.send(StreamEvent::Done);
        }

        // Calculate token usage
        let input_tokens = messages.iter().map(|m| m.content.len() / 4).sum::<usize>() as u32;
        let output_tokens = (response_text.len() / 4) as u32;

        Ok(ProviderResponse {
            text: response_text.to_string(),
            tool_calls: vec![],
            stop_reason: StopReason::EndTurn,
            usage: TokenUsage { input: input_tokens, output: output_tokens },
        })
    }

    async fn available_models(&self) -> Vec<String> {
        vec!["dev-v4-pro".to_string()]
    }

    fn name(&self) -> &str {
        "dev"
    }
}
