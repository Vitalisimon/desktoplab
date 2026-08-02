use std::collections::BTreeMap;
use std::io::{BufRead, BufReader};
use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::{Value, json};

#[derive(Default)]
struct ToolCallFragments {
    id: String,
    name: String,
    arguments: String,
}

pub(crate) fn execute(
    url: &str,
    payload: Value,
    prompt: &crate::BackendPrompt,
    cancellation: &AtomicBool,
    on_delta: &mut impl FnMut(&str),
) -> Result<Value, String> {
    if cancellation.load(Ordering::SeqCst) {
        return Err("agent_cancelled".to_string());
    }
    let response = crate::openai_compatible_http::request_client(prompt)?
        .post(url)
        .json(&payload)
        .send()
        .map_err(|error| format!("openai_compatible_stream_request_failed:{error}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "openai_compatible_stream_http_status:{}",
            response.status()
        ));
    }
    read_sse(response, cancellation, on_delta)
}

fn read_sse(
    response: impl std::io::Read,
    cancellation: &AtomicBool,
    on_delta: &mut impl FnMut(&str),
) -> Result<Value, String> {
    let mut content = String::new();
    let mut reasoning = String::new();
    let mut calls = BTreeMap::<usize, ToolCallFragments>::new();
    for line in BufReader::new(response).lines() {
        if cancellation.load(Ordering::SeqCst) {
            return Err("agent_cancelled".to_string());
        }
        let line = line.map_err(|error| format!("openai_compatible_stream_read_failed:{error}"))?;
        let Some(data) = line.strip_prefix("data:").map(str::trim) else {
            continue;
        };
        if data == "[DONE]" {
            break;
        }
        let chunk: Value = serde_json::from_str(data)
            .map_err(|error| format!("openai_compatible_stream_json:{error}"))?;
        let delta = &chunk["choices"][0]["delta"];
        append_text(delta, "content", &mut content, on_delta);
        append_reasoning(delta, &mut reasoning);
        append_tool_fragments(delta, &mut calls)?;
    }
    if cancellation.load(Ordering::SeqCst) {
        return Err("agent_cancelled".to_string());
    }
    Ok(response_envelope(content, reasoning, calls))
}

fn append_reasoning(delta: &Value, reasoning: &mut String) {
    let field = if delta["reasoning_content"].is_string() {
        "reasoning_content"
    } else {
        "reasoning"
    };
    append_text(delta, field, reasoning, &mut |_| {});
}

fn append_text(delta: &Value, field: &str, target: &mut String, on_delta: &mut impl FnMut(&str)) {
    if let Some(fragment) = delta[field].as_str()
        && !fragment.is_empty()
    {
        target.push_str(fragment);
        on_delta(fragment);
    }
}

fn append_tool_fragments(
    delta: &Value,
    calls: &mut BTreeMap<usize, ToolCallFragments>,
) -> Result<(), String> {
    let Some(fragments) = delta.get("tool_calls") else {
        return Ok(());
    };
    let fragments = fragments
        .as_array()
        .ok_or_else(|| "openai_compatible_stream_tool_calls_must_be_array".to_string())?;
    for fragment in fragments {
        let index = fragment["index"]
            .as_u64()
            .ok_or_else(|| "openai_compatible_stream_tool_call_missing_index".to_string())?
            as usize;
        let call = calls.entry(index).or_default();
        append_string(&mut call.id, &fragment["id"]);
        append_string(&mut call.name, &fragment["function"]["name"]);
        append_string(&mut call.arguments, &fragment["function"]["arguments"]);
    }
    Ok(())
}

fn append_string(target: &mut String, value: &Value) {
    if let Some(fragment) = value.as_str() {
        target.push_str(fragment);
    }
}

fn response_envelope(
    content: String,
    reasoning: String,
    calls: BTreeMap<usize, ToolCallFragments>,
) -> Value {
    let tool_calls = calls
        .into_values()
        .map(|call| {
            json!({
                "id":call.id,
                "type":"function",
                "function":{"name":call.name,"arguments":call.arguments}
            })
        })
        .collect::<Vec<_>>();
    json!({"choices":[{"message":{
        "content":content,
        "reasoning_content":reasoning,
        "tool_calls":tool_calls
    }}]})
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicBool;

    use super::read_sse;

    #[test]
    fn mlx_reasoning_alias_is_preserved_in_the_canonical_envelope() {
        let body = concat!(
            "data: {\"choices\":[{\"delta\":{\"reasoning\":\"inspect \"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"reasoning\":\"evidence\",\"content\":\"{}\"}}]}\n\n",
            "data: [DONE]\n\n"
        );
        let value = read_sse(body.as_bytes(), &AtomicBool::new(false), &mut |_| {}).unwrap();

        assert_eq!(
            value["choices"][0]["message"]["reasoning_content"],
            "inspect evidence"
        );
        assert_eq!(value["choices"][0]["message"]["content"], "{}");
    }
}
