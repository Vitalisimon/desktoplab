use serde_json::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderMessageEnvelope {
    OllamaChat,
    OpenAiChatCompletions,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderCompatibilityProfile {
    envelope: ProviderMessageEnvelope,
    tool_choice: Option<&'static str>,
    reasoning_fields: &'static [&'static str],
    max_tool_calls_per_turn: usize,
}

impl ProviderCompatibilityProfile {
    #[must_use]
    pub fn ollama_chat() -> Self {
        Self {
            envelope: ProviderMessageEnvelope::OllamaChat,
            tool_choice: None,
            reasoning_fields: &["thinking"],
            max_tool_calls_per_turn: 1,
        }
    }

    #[must_use]
    pub fn openai_chat_completions() -> Self {
        Self {
            envelope: ProviderMessageEnvelope::OpenAiChatCompletions,
            tool_choice: Some("auto"),
            reasoning_fields: &["reasoning_content", "reasoning"],
            max_tool_calls_per_turn: 1,
        }
    }

    pub(crate) fn message<'a>(&self, response: &'a Value) -> &'a Value {
        match self.envelope {
            ProviderMessageEnvelope::OllamaChat => &response["message"],
            ProviderMessageEnvelope::OpenAiChatCompletions => &response["choices"][0]["message"],
        }
    }

    pub(crate) fn reasoning_text(&self, message: &Value) -> Option<String> {
        self.reasoning_fields
            .iter()
            .find_map(|field| message[*field].as_str())
            .map(ToString::to_string)
    }

    pub(crate) fn max_tool_calls_per_turn(&self) -> usize {
        self.max_tool_calls_per_turn
    }

    pub fn apply_tool_choice(&self, payload: &mut Value) {
        if let Some(choice) = self.tool_choice {
            payload["tool_choice"] = Value::String(choice.to_string());
        }
    }
}
