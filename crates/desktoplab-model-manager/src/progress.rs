#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelPullProgress {
    stage: String,
    percent: Option<u8>,
    downloaded_bytes: Option<u64>,
    total_bytes: Option<u64>,
    evidence: String,
}

impl ModelPullProgress {
    #[must_use]
    pub fn parse(line: &str) -> Self {
        let evidence = line.trim().to_string();
        let percent = parse_percent(&evidence);
        let (downloaded_bytes, total_bytes) = parse_size_pair(&evidence).unwrap_or((None, None));
        Self {
            stage: parse_stage(&evidence),
            percent,
            downloaded_bytes,
            total_bytes,
            evidence,
        }
    }

    #[must_use]
    pub fn stage(&self) -> &str {
        &self.stage
    }

    #[must_use]
    pub fn percent(&self) -> Option<u8> {
        self.percent
    }

    #[must_use]
    pub fn downloaded_bytes(&self) -> Option<u64> {
        self.downloaded_bytes
    }

    #[must_use]
    pub fn total_bytes(&self) -> Option<u64> {
        self.total_bytes
    }

    #[must_use]
    pub fn evidence(&self) -> &str {
        &self.evidence
    }
}

fn parse_stage(line: &str) -> String {
    line.split_whitespace()
        .next()
        .unwrap_or("unknown")
        .trim_end_matches(':')
        .to_string()
}

fn parse_percent(line: &str) -> Option<u8> {
    line.split_whitespace().find_map(|part| {
        let raw = part.trim_end_matches('%');
        if raw.len() == part.len() {
            return None;
        }
        raw.parse::<u8>().ok().filter(|value| *value <= 100)
    })
}

fn parse_size_pair(line: &str) -> Option<(Option<u64>, Option<u64>)> {
    let parts = line.split_whitespace().collect::<Vec<_>>();
    for window in parts.windows(3) {
        let Some(left) = window[0].parse::<f64>().ok() else {
            continue;
        };
        let Some((left_unit, right_value)) = window[1].split_once('/') else {
            continue;
        };
        let Some(right) = right_value.parse::<f64>().ok() else {
            continue;
        };
        let right_unit = window[2];
        if let (Some(downloaded), Some(total)) = (bytes(left, left_unit), bytes(right, right_unit))
        {
            return Some((Some(downloaded), Some(total)));
        }
    }
    None
}

fn bytes(value: f64, unit: &str) -> Option<u64> {
    let multiplier = match unit.to_ascii_uppercase().as_str() {
        "KB" => 1_000_f64,
        "MB" => 1_000_000_f64,
        "GB" => 1_000_000_000_f64,
        _ => return None,
    };
    Some((value * multiplier).round() as u64)
}
