use std::io;

use regex::{Regex, RegexBuilder};

pub(crate) enum WorkspaceSearchPattern {
    Literal {
        needle: String,
        case_sensitive: bool,
    },
    Regex(Regex),
}

impl WorkspaceSearchPattern {
    pub(crate) fn new(query: &str, regex: bool, case_sensitive: bool) -> io::Result<Self> {
        if regex {
            return RegexBuilder::new(query)
                .case_insensitive(!case_sensitive)
                .build()
                .map(Self::Regex)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error));
        }
        Ok(Self::Literal {
            needle: if case_sensitive {
                query.to_string()
            } else {
                query.to_ascii_lowercase()
            },
            case_sensitive,
        })
    }

    pub(crate) fn matches(&self, value: &str) -> bool {
        match self {
            Self::Literal {
                needle,
                case_sensitive: true,
            } => value.contains(needle),
            Self::Literal { needle, .. } => value.to_ascii_lowercase().contains(needle),
            Self::Regex(regex) => regex.is_match(value),
        }
    }
}
