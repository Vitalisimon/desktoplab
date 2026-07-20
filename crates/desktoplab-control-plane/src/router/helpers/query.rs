pub(crate) fn query_value(path: &str, key: &str) -> Option<String> {
    let encoded = path.split_once('?')?.1.split('&').find_map(|pair| {
        let (candidate, value) = pair.split_once('=')?;
        (candidate == key && !value.is_empty()).then_some(value)
    })?;
    percent_decode(encoded)
}

fn percent_decode(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' if index + 2 < bytes.len() => {
                decoded.push((hex_value(bytes[index + 1])? << 4) | hex_value(bytes[index + 2])?);
                index += 3;
            }
            b'+' => {
                decoded.push(b' ');
                index += 1;
            }
            byte => {
                decoded.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8(decoded).ok()
}

fn hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::query_value;

    #[test]
    fn decodes_percent_encoded_workspace_ids() {
        assert_eq!(
            query_value(
                "/v1/sessions?workspace_id=workspace.shared%20repo",
                "workspace_id"
            ),
            Some("workspace.shared repo".to_string())
        );
    }

    #[test]
    fn rejects_invalid_percent_encoding() {
        assert_eq!(
            query_value("/v1/sessions?workspace_id=workspace.%ZZ", "workspace_id"),
            None
        );
    }
}
