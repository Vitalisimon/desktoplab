use std::io::{BufRead, BufReader};
use std::sync::atomic::{AtomicBool, Ordering};

use reqwest::blocking::Client;
use serde_json::{Value, json};

pub(crate) fn execute(
    client: &Client,
    url: &str,
    payload: Value,
    cancellation: &AtomicBool,
    on_delta: &mut impl FnMut(&str),
) -> Result<Value, String> {
    if cancellation.load(Ordering::SeqCst) {
        return Err("agent_cancelled".to_string());
    }
    let response = client
        .post(url)
        .json(&payload)
        .send()
        .map_err(|error| format!("ollama_request_failed:{error}"))?;
    if !response.status().is_success() {
        return Err(format!("ollama_http_status:{}", response.status()));
    }
    let mut content = String::new();
    let mut tool_calls = Vec::new();
    for line in BufReader::new(response).lines() {
        if cancellation.load(Ordering::SeqCst) {
            return Err("agent_cancelled".to_string());
        }
        let line = line.map_err(|error| format!("ollama_stream_read_failed:{error}"))?;
        if line.trim().is_empty() {
            continue;
        }
        let chunk: Value =
            serde_json::from_str(&line).map_err(|error| format!("ollama_stream_json:{error}"))?;
        if let Some(delta) = chunk["message"]["content"].as_str()
            && !delta.is_empty()
        {
            content.push_str(delta);
            on_delta(delta);
        }
        if let Some(calls) = chunk["message"]["tool_calls"].as_array() {
            tool_calls.extend(calls.iter().cloned());
        }
        if chunk["done"].as_bool() == Some(true) {
            break;
        }
    }
    if cancellation.load(Ordering::SeqCst) {
        return Err("agent_cancelled".to_string());
    }
    Ok(json!({
        "message":{
            "content":content,
            "tool_calls":tool_calls
        }
    }))
}
