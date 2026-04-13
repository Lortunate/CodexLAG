const REDACTED: &str = "[redacted]";

pub fn redact_sensitive_value(value: &str) -> String {
    let value = redact_sensitive_query_params(value);
    let value = redact_named_secret_assignments(&value);
    let value = redact_prefixed_token(&value, "ck_local_");
    redact_bearer_token(&value)
}

fn redact_sensitive_query_params(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let bytes = value.as_bytes();
    let mut cursor = 0usize;

    while cursor < bytes.len() {
        let current = bytes[cursor];
        if current != b'?' && current != b'&' {
            let chunk_start = cursor;
            while cursor < bytes.len() && bytes[cursor] != b'?' && bytes[cursor] != b'&' {
                cursor += 1;
            }
            output.push_str(&value[chunk_start..cursor]);
            continue;
        }

        output.push(current as char);
        cursor += 1;

        let key_start = cursor;
        while cursor < bytes.len() {
            let byte = bytes[cursor];
            if byte == b'=' || byte == b'&' || byte == b'#' || byte.is_ascii_whitespace() {
                break;
            }
            cursor += 1;
        }
        let key = &value[key_start..cursor];
        output.push_str(key);

        if cursor >= bytes.len() || bytes[cursor] != b'=' {
            continue;
        }

        output.push('=');
        cursor += 1;

        let parameter_value_start = cursor;
        while cursor < bytes.len() {
            let byte = bytes[cursor];
            if byte == b'&' || byte == b'#' || byte.is_ascii_whitespace() {
                break;
            }
            cursor += 1;
        }
        if is_sensitive_parameter_key(key) && cursor > parameter_value_start {
            output.push_str(REDACTED);
        } else {
            output.push_str(&value[parameter_value_start..cursor]);
        }
    }

    output
}

fn redact_named_secret_assignments(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let bytes = value.as_bytes();
    let mut cursor = 0usize;

    while cursor < bytes.len() {
        let Some(relative_sep) = bytes[cursor..]
            .iter()
            .position(|byte| *byte == b'=' || *byte == b':')
        else {
            output.push_str(&value[cursor..]);
            break;
        };
        let separator_index = cursor + relative_sep;

        let mut key_start = separator_index;
        while key_start > cursor {
            let byte = bytes[key_start - 1];
            if byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-' {
                key_start -= 1;
            } else {
                break;
            }
        }

        output.push_str(&value[cursor..=separator_index]);
        let key = &value[key_start..separator_index];

        let mut value_start = separator_index + 1;
        while value_start < bytes.len() && bytes[value_start].is_ascii_whitespace() {
            output.push(bytes[value_start] as char);
            value_start += 1;
        }

        let mut value_end = value_start;
        while value_end < bytes.len() {
            let byte = bytes[value_end];
            if byte.is_ascii_whitespace()
                || byte == b'&'
                || byte == b','
                || byte == b';'
                || byte == b'"'
                || byte == b'\''
            {
                break;
            }
            value_end += 1;
        }

        if is_sensitive_parameter_key(key) && value_end > value_start {
            output.push_str(REDACTED);
        } else {
            output.push_str(&value[value_start..value_end]);
        }

        cursor = value_end;
    }

    output
}

fn is_sensitive_parameter_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase();
    if matches!(
        normalized.as_str(),
        "apikey" | "api_key" | "api-key" | "sessionid" | "session_id" | "session-id"
    ) {
        return true;
    }

    normalized
        .split(|character: char| !character.is_ascii_alphanumeric())
        .any(|segment| matches!(segment, "key" | "token" | "session" | "apikey"))
}

fn redact_prefixed_token(value: &str, prefix: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut cursor = 0usize;

    while let Some(relative_start) = value[cursor..].find(prefix) {
        let start = cursor + relative_start;
        output.push_str(&value[cursor..start]);
        output.push_str(prefix);
        output.push_str(REDACTED);

        let mut token_end = start + prefix.len();
        for ch in value[token_end..].chars() {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                token_end += ch.len_utf8();
            } else {
                break;
            }
        }

        cursor = token_end;
    }

    output.push_str(&value[cursor..]);
    output
}

fn redact_bearer_token(value: &str) -> String {
    let marker = "bearer ";
    let lowercase = value.to_ascii_lowercase();
    let mut output = String::with_capacity(value.len());
    let mut cursor = 0usize;

    while let Some(relative_start) = lowercase[cursor..].find(marker) {
        let start = cursor + relative_start;
        output.push_str(&value[cursor..start]);
        output.push_str(marker);
        output.push_str(REDACTED);

        let mut token_end = start + marker.len();
        for ch in value[token_end..].chars() {
            if ch.is_whitespace() || ch == '"' || ch == '\'' || ch == ',' {
                break;
            }
            token_end += ch.len_utf8();
        }
        cursor = token_end;
    }

    output.push_str(&value[cursor..]);
    output
}
