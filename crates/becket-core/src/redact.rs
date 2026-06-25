//! Basic secret redaction before content is sent to MCP sampling.

use std::sync::OnceLock;

use regex::Regex;

fn patterns() -> &'static Vec<Regex> {
    static PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        [
            r"(?i)(api[_-]?key|secret|token|password|passwd)\s*[:=]\s*\S+",
            r"(?i)authorization\s*:\s*bearer\s+\S+",
            r"(?i)bearer\s+\S+",
            r"\bsk-[A-Za-z0-9]{16,}\b",
            r"\bAKIA[0-9A-Z]{16}\b",
            r"(?i)-----BEGIN [A-Z ]+ PRIVATE KEY-----[\s\S]*?-----END [A-Z ]+ PRIVATE KEY-----",
        ]
        .into_iter()
        .filter_map(|pattern| Regex::new(pattern).ok())
        .collect()
    })
}

/// Redacts likely secrets from text before LLM sampling.
pub fn redact_secrets(input: &str) -> String {
    let mut output = input.to_string();
    for pattern in patterns() {
        output = pattern.replace_all(&output, "[REDACTED]").into_owned();
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_api_key_assignment() {
        let text = "config api_key=supersecret123 value";
        assert!(!redact_secrets(text).contains("supersecret123"));
        assert!(redact_secrets(text).contains("[REDACTED]"));
    }

    #[test]
    fn redacts_bearer_token() {
        let text = "Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.abc";
        assert!(!redact_secrets(text).contains("eyJhbGci"));
    }

    #[test]
    fn leaves_benign_code_untouched() {
        let text = "fn process_payment(amount: u64) -> Result<()>;";
        assert_eq!(redact_secrets(text), text);
    }
}
