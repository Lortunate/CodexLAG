use crate::logging::redaction::redact_sensitive_value_for_key;

pub fn redact_secret_value(value: &str) -> String {
    if value.is_empty() {
        return String::new();
    }

    let char_count = value.chars().count();
    if char_count <= 4 {
        return "*".repeat(char_count);
    }

    let prefix: String = value.chars().take(4).collect();
    let masked = "*".repeat(char_count - 3);
    format!("{prefix}{masked}")
}

pub fn format_event_fields(fields: &[(&str, &str)]) -> String {
    let mut line = String::new();
    for (key, value) in fields {
        append_event_field(&mut line, key, value);
    }
    line
}

pub fn format_runtime_event_fields(
    component: &str,
    event: &str,
    request_id: &str,
    attempt_id: Option<&str>,
    endpoint_id: Option<&str>,
    latency_ms: Option<u64>,
    error_code: Option<&str>,
    extra_fields: &[(&str, &str)],
) -> String {
    let mut line = String::new();
    append_event_field(&mut line, "component", component);
    append_event_field(&mut line, "event", event);
    append_event_field(&mut line, "request_id", request_id);
    append_event_field(&mut line, "attempt_id", attempt_id.unwrap_or("none"));
    append_event_field(&mut line, "endpoint_id", endpoint_id.unwrap_or("none"));

    let latency_value = latency_ms
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_string());
    append_event_field(&mut line, "latency_ms", latency_value.as_str());
    let error_value = error_code.unwrap_or("none");
    append_event_field(&mut line, "error_code", error_value);
    append_event_field(&mut line, "error", error_value);

    for (key, value) in extra_fields {
        append_event_field(&mut line, key, value);
    }

    line
}

pub fn build_attempt_id(request_id: &str, attempt_index: usize) -> String {
    format!("{request_id}:{attempt_index}")
}

fn encode_field_value(value: &str) -> String {
    if !requires_quoting(value) {
        return value.to_string();
    }

    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            c if c.is_control() => escaped.push_str(&format!("\\u{:04x}", c as u32)),
            c => escaped.push(c),
        }
    }
    escaped.push('"');
    escaped
}

fn append_event_field(line: &mut String, key: &str, value: &str) {
    if !line.is_empty() {
        line.push(' ');
    }
    let redacted_value = redact_sensitive_value_for_key(key, value);
    line.push_str(key);
    line.push('=');
    line.push_str(&encode_field_value(redacted_value.as_str()));
}

fn requires_quoting(value: &str) -> bool {
    value.is_empty()
        || value
            .chars()
            .any(|c| c.is_whitespace() || c == '=' || c == '"' || c == '\\' || c.is_control())
}
