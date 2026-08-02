use std::time::Duration;

use reqwest::blocking::Client;

use crate::BackendPrompt;

pub(crate) fn request_client(prompt: &BackendPrompt) -> Result<Client, String> {
    let mut builder = Client::builder();
    if let Some(seconds) = prompt.request_timeout_seconds() {
        builder = builder.timeout(Duration::from_secs(seconds));
    }
    builder
        .build()
        .map_err(|error| format!("openai_compatible_client_build_failed:{error}"))
}
