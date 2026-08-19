use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiInputMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiToolCall {
    pub call_id: String,
    pub name: String,
    pub arguments: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiToolOutput {
    pub call_id: String,
    pub output: Value,
}

#[derive(Debug, Clone)]
pub struct AiStructuredOutput {
    pub name: String,
    pub schema: Value,
}

#[derive(Debug, Clone)]
pub struct AiProviderRequest {
    pub instructions: String,
    pub input: Vec<AiInputMessage>,
    pub tools: Vec<AiToolDefinition>,
    pub structured_output: Option<AiStructuredOutput>,
}

#[derive(Debug, Clone)]
pub struct AiProviderResponse {
    pub response_id: String,
    pub text: String,
    pub tool_calls: Vec<AiToolCall>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AiError {
    #[cfg(test)]
    Unavailable(String),
    Transport(String),
    InvalidResponse(String),
    Provider(String),
}

impl Display for AiError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(message) | Self::InvalidResponse(message) | Self::Provider(message) => {
                formatter.write_str(message)
            }
            #[cfg(test)]
            Self::Unavailable(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for AiError {}

/// Provider boundary: application code speaks only these portable request and
/// response types. No domain or storage module depends on an OpenAI SDK/API.
pub trait AiProvider: Send + Sync {
    fn create_response(
        &self,
        api_key: &str,
        request: &AiProviderRequest,
    ) -> Result<AiProviderResponse, AiError>;

    fn continue_with_tools(
        &self,
        api_key: &str,
        previous_response_id: &str,
        outputs: &[AiToolOutput],
    ) -> Result<AiProviderResponse, AiError>;
}

#[cfg(test)]
#[derive(Default)]
pub struct UnavailableAiProvider;

#[cfg(test)]
impl AiProvider for UnavailableAiProvider {
    fn create_response(
        &self,
        _api_key: &str,
        _request: &AiProviderRequest,
    ) -> Result<AiProviderResponse, AiError> {
        Err(AiError::Unavailable(
            "AI is unavailable in this application instance.".to_owned(),
        ))
    }

    fn continue_with_tools(
        &self,
        _api_key: &str,
        _previous_response_id: &str,
        _outputs: &[AiToolOutput],
    ) -> Result<AiProviderResponse, AiError> {
        Err(AiError::Unavailable(
            "AI is unavailable in this application instance.".to_owned(),
        ))
    }
}
