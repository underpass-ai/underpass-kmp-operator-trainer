//! Single chat-completion message: a role + a content string. Mirrors
//! the wire shape every OpenAI-compatible LLM expects, but lives in
//! the application layer so ports can take typed messages instead of
//! free-form strings.
//!
//! Validation is intentionally minimal — both fields go straight to
//! the adapter, which serialises them and ships them to the LLM as
//! is. Role is held as a free-form string rather than a closed enum
//! because OpenAI-compatible servers accept extensions
//! (`tool`, `function`, custom system roles) and we do not want to
//! reject them here.

use operator_shared_domain::error::domain_result::DomainResult;
use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
    role: NonEmptyString,
    content: NonEmptyString,
}

impl ChatMessage {
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> DomainResult<Self> {
        Ok(Self {
            role: NonEmptyString::parse(role, "chat_message.role")?,
            content: NonEmptyString::parse(content, "chat_message.content")?,
        })
    }

    pub fn system(content: impl Into<String>) -> DomainResult<Self> {
        Self::new("system", content)
    }

    pub fn user(content: impl Into<String>) -> DomainResult<Self> {
        Self::new("user", content)
    }

    pub fn assistant(content: impl Into<String>) -> DomainResult<Self> {
        Self::new("assistant", content)
    }

    pub fn role(&self) -> &str {
        self.role.as_str()
    }

    pub fn content(&self) -> &str {
        self.content.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_user_assistant_constructors_set_expected_roles() {
        let system = ChatMessage::system("rule").unwrap();
        let user = ChatMessage::user("question").unwrap();
        let assistant = ChatMessage::assistant("answer").unwrap();
        assert_eq!(system.role(), "system");
        assert_eq!(user.role(), "user");
        assert_eq!(assistant.role(), "assistant");
        assert_eq!(system.content(), "rule");
        assert_eq!(user.content(), "question");
        assert_eq!(assistant.content(), "answer");
    }

    #[test]
    fn empty_role_is_rejected() {
        assert!(ChatMessage::new("", "content").is_err());
    }

    #[test]
    fn empty_content_is_rejected() {
        assert!(ChatMessage::new("user", "").is_err());
    }

    #[test]
    fn custom_role_is_accepted_for_openai_extensions() {
        // OpenAI-compat servers accept `tool`/`function` and custom
        // system roles; the value object must not reject them.
        let message = ChatMessage::new("tool", "stdout").unwrap();
        assert_eq!(message.role(), "tool");
    }
}
