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
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(" ")
}
