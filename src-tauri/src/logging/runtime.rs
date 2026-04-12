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
    fields
        .iter()
        .map(|(key, value)| format!("{key}={}", encode_field_value(value)))
        .collect::<Vec<_>>()
        .join(" ")
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

fn requires_quoting(value: &str) -> bool {
    value.is_empty()
        || value
            .chars()
            .any(|c| c.is_whitespace() || c == '=' || c == '"' || c == '\\' || c.is_control())
}
