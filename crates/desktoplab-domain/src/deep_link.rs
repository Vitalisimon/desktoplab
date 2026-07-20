#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeepLinkAction {
    Setup,
    OpenRepository {
        path: Option<String>,
    },
    Thread {
        thread_id: String,
    },
    ProviderCallback {
        provider: String,
        state: String,
        code: Option<String>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesktopLabDeepLink {
    action: DeepLinkAction,
}

impl DesktopLabDeepLink {
    pub fn parse(input: &str) -> Result<Self, DeepLinkError> {
        let remainder = input
            .strip_prefix("desktoplab://")
            .ok_or(DeepLinkError::UnsupportedScheme)?;
        let (path_part, query) = split_once(remainder, '?');
        let segments = path_part
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect::<Vec<_>>();

        let action = match segments.as_slice() {
            ["setup"] => DeepLinkAction::Setup,
            ["open", "repository"] => {
                let path = query_value(query, "path");
                if let Some(candidate) = &path {
                    validate_repository_path(candidate)?;
                }
                DeepLinkAction::OpenRepository { path }
            }
            ["thread", thread_id] if !thread_id.trim().is_empty() => DeepLinkAction::Thread {
                thread_id: (*thread_id).to_string(),
            },
            ["provider", "callback"] => DeepLinkAction::ProviderCallback {
                provider: required_query_value(query, "provider")?,
                state: required_query_value(query, "state")?,
                code: query_value(query, "code"),
            },
            _ => return Err(DeepLinkError::UnknownAction),
        };

        Ok(Self { action })
    }

    #[must_use]
    pub fn action(&self) -> &DeepLinkAction {
        &self.action
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeepLinkError {
    UnsupportedScheme,
    UnknownAction,
    MissingRequiredQuery,
    UnsafeRepositoryPath,
}

fn split_once(input: &str, delimiter: char) -> (&str, Option<&str>) {
    match input.split_once(delimiter) {
        Some((left, right)) => (left, Some(right)),
        None => (input, None),
    }
}

fn query_value(query: Option<&str>, key: &str) -> Option<String> {
    query?
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(candidate, _)| *candidate == key)
        .map(|(_, value)| percent_decode(value))
        .filter(|value| !value.trim().is_empty())
}

fn required_query_value(query: Option<&str>, key: &str) -> Result<String, DeepLinkError> {
    query_value(query, key).ok_or(DeepLinkError::MissingRequiredQuery)
}

fn validate_repository_path(path: &str) -> Result<(), DeepLinkError> {
    let normalized = path.replace('\\', "/");
    if normalized
        .split('/')
        .any(|segment| segment == ".." || segment == "%2e%2e" || segment == "%2E%2E")
    {
        return Err(DeepLinkError::UnsafeRepositoryPath);
    }
    Ok(())
}

fn percent_decode(value: &str) -> String {
    let mut output = String::new();
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let Ok(decoded) = u8::from_str_radix(&value[index + 1..index + 3], 16) {
                output.push(decoded as char);
                index += 3;
                continue;
            }
        }
        output.push(bytes[index] as char);
        index += 1;
    }
    output
}
