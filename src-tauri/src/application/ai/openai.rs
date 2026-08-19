use serde_json::{json, Value};

use super::provider::{
    AiError, AiProvider, AiProviderRequest, AiProviderResponse, AiToolCall, AiToolOutput,
};

const RESPONSES_URL: &str = "https://api.openai.com/v1/responses";
const DEFAULT_MODEL: &str = "gpt-5.4-mini";

pub struct OpenAiProvider {
    client: reqwest::blocking::Client,
    model: String,
}

impl Default for OpenAiProvider {
    fn default() -> Self {
        Self::new(DEFAULT_MODEL)
    }
}

impl OpenAiProvider {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            client: reqwest::blocking::Client::new(),
            model: model.into(),
        }
    }

    fn send(&self, api_key: &str, payload: Value) -> Result<AiProviderResponse, AiError> {
        let response = self
            .client
            .post(RESPONSES_URL)
            .bearer_auth(api_key)
            .json(&payload)
            .send()
            .map_err(|error| AiError::Transport(format!("OpenAI request failed: {error}")))?;
        let status = response.status();
        let body: Value = response.json().map_err(|error| {
            AiError::InvalidResponse(format!("OpenAI returned unreadable JSON: {error}"))
        })?;
        if !status.is_success() {
            let message = body
                .get("error")
                .and_then(|value| value.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("OpenAI rejected the request.");
            return Err(AiError::Provider(format!(
                "OpenAI request failed ({status}): {message}"
            )));
        }
        parse_response(body)
    }
}

impl AiProvider for OpenAiProvider {
    fn create_response(
        &self,
        api_key: &str,
        request: &AiProviderRequest,
    ) -> Result<AiProviderResponse, AiError> {
        let mut payload = json!({
            "model": self.model,
            "store": false,
            "instructions": request.instructions,
            "input": request.input.iter().map(|message| json!({
                "role": message.role,
                "content": [{"type": "input_text", "text": message.content}],
            })).collect::<Vec<_>>(),
            "tools": request.tools.iter().map(|tool| json!({
                "type": "function",
                "name": tool.name,
                "description": tool.description,
                "parameters": tool.parameters,
                "strict": true,
            })).collect::<Vec<_>>(),
        });
        if let Some(output) = &request.structured_output {
            payload["text"] = json!({"format": {
                "type": "json_schema",
                "name": output.name,
                "strict": true,
                "schema": output.schema,
            }});
        }
        self.send(api_key, payload)
    }

    fn continue_with_tools(
        &self,
        api_key: &str,
        previous_response_id: &str,
        outputs: &[AiToolOutput],
    ) -> Result<AiProviderResponse, AiError> {
        self.send(
            api_key,
            json!({
                "model": self.model,
                "store": false,
                "previous_response_id": previous_response_id,
                "input": outputs.iter().map(|output| json!({
                    "type": "function_call_output",
                    "call_id": output.call_id,
                    "output": output.output.to_string(),
                })).collect::<Vec<_>>(),
            }),
        )
    }
}

pub fn parse_response(value: Value) -> Result<AiProviderResponse, AiError> {
    let response_id = value
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| AiError::InvalidResponse("OpenAI response is missing an id.".to_owned()))?
        .to_owned();
    let mut text = value
        .get("output_text")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let mut tool_calls = Vec::new();
    if let Some(output) = value.get("output").and_then(Value::as_array) {
        for item in output {
            match item.get("type").and_then(Value::as_str) {
                Some("function_call") => {
                    let call_id = item.get("call_id").and_then(Value::as_str).ok_or_else(|| {
                        AiError::InvalidResponse("Function call is missing call_id.".to_owned())
                    })?;
                    let name = item.get("name").and_then(Value::as_str).ok_or_else(|| {
                        AiError::InvalidResponse("Function call is missing name.".to_owned())
                    })?;
                    let arguments =
                        item.get("arguments")
                            .and_then(Value::as_str)
                            .ok_or_else(|| {
                                AiError::InvalidResponse(
                                    "Function call is missing arguments.".to_owned(),
                                )
                            })?;
                    tool_calls.push(AiToolCall {
                        call_id: call_id.to_owned(),
                        name: name.to_owned(),
                        arguments: serde_json::from_str(arguments).map_err(|error| {
                            AiError::InvalidResponse(format!(
                                "Function call arguments are invalid JSON: {error}"
                            ))
                        })?,
                    });
                }
                Some("message") if text.is_empty() => {
                    if let Some(content) = item.get("content").and_then(Value::as_array) {
                        text = content
                            .iter()
                            .filter_map(|part| part.get("text").and_then(Value::as_str))
                            .collect::<Vec<_>>()
                            .join("\n");
                    }
                }
                _ => {}
            }
        }
    }
    Ok(AiProviderResponse {
        response_id,
        text,
        tool_calls,
    })
}

#[cfg(test)]
mod tests {
    use super::parse_response;
    use serde_json::json;

    #[test]
    fn parses_text_and_function_calls_from_responses_api_output() {
        let response = parse_response(json!({"id":"resp_1","output":[
            {"type":"function_call","call_id":"call_1","name":"get_active_goals","arguments":"{}"},
            {"type":"message","content":[{"type":"output_text","text":"I checked your goals."}]}
        ]}))
        .expect("valid response should parse");
        assert_eq!(response.text, "I checked your goals.");
        assert_eq!(response.tool_calls[0].name, "get_active_goals");
    }

    #[test]
    fn rejects_invalid_function_arguments() {
        assert!(parse_response(json!({"id":"resp_1","output":[
            {"type":"function_call","call_id":"call_1","name":"get_active_goals","arguments":"not json"}
        ]})).is_err());
    }
}
