use crate::codex::{parse_input, reasoning, user_data};
use crate::common::*;
use crate::{Error, EventData, ParsedEvent, normalize_file_modification};
use serde_json::{Value, json};
pub(crate) fn parse_response_item(p: &Value, ts: i64) -> Result<Vec<ParsedEvent>, Error> {
    let mut events = vec![];
    match s(p, "type") {
        "function_call" | "custom_tool_call" => {
            let name = s(p, "name");
            let mut params = if !s(p, "arguments").is_empty() {
                parse_input(&p["arguments"])
            } else {
                parse_input(&p["input"])
            };
            if let Some(m) = &mut params {
                let keys = match name {
                    "exec_command" => vec![("cmd", "command")],
                    "spawn_agent" => {
                        vec![("message", "prompt"), ("agent_type", "description")]
                    }
                    _ => vec![],
                };
                for (from, to) in keys {
                    if m.get(to).is_none_or(Value::is_null)
                        && let Some(v) = m.get(from).cloned()
                    {
                        m.insert(to.into(), v);
                    }
                }
            }
            let is_question = matches!(
                name,
                "AskUserQuestion"
                    | "ask_user"
                    | "ask_question"
                    | "request_user_input"
                    | "request_user_input_async"
            );
            events.push(event(
                if is_question {
                    "question_prompt"
                } else {
                    "tool_call"
                },
                ts,
                "",
                "",
                EventData {
                    role: Some("assistant".into()),
                    tool_name: Some(name.into()),
                    tool_call_id: opt(s(p, "call_id")),
                    question: is_question.then(|| {
                        question(&Value::Object(params.clone().unwrap_or_default()), true)
                    }),
                    file_modification: normalize_file_modification(name, params.as_ref()),
                    tool_input: params,
                    ..Default::default()
                },
            ))
        }
        "function_call_output" | "custom_tool_call_output" | "tool_search_output" => {
            let output = match &p["output"] {
                Value::String(s) => s.clone(),
                Value::Array(_) => p["output"].to_string(),
                Value::Null => String::new(),
                _ => {
                    return Err(Error::Invalid(
                        "tool output must be text or content array".into(),
                    ));
                }
            };
            let lower = output.to_lowercase();
            events.push(event(
                "tool_result",
                ts,
                "",
                "",
                EventData {
                    role: Some("toolResult".into()),
                    tool_call_id: opt(s(p, "call_id")),
                    tool_output: Some(output),
                    is_error: (s(p, "type") != "tool_search_output").then(|| {
                        lower.contains("exit code: 1")
                            || lower.contains("error:")
                            || lower.contains("command failed")
                    }),
                    ..Default::default()
                },
            ))
        }
        "message" => {
            let text = content(&p["content"]);
            if s(p, "role") == "user" {
                if let Some(data) = user_data(&text) {
                    events.push(event("turn_start", ts, "", "", data))
                }
            } else if s(p, "role") == "assistant" && !text.is_empty() {
                events.push(event(
                    "agent_message",
                    ts,
                    "",
                    "",
                    EventData {
                        role: Some("assistant".into()),
                        content: Some(text),
                        ..Default::default()
                    },
                ))
            }
        }
        "reasoning" => {
            if let Some(data) = reasoning(p) {
                events.push(event("thinking", ts, "", "", data))
            }
        }
        "tool_search_call" => events.push(event(
            "tool_call",
            ts,
            "",
            "",
            EventData {
                role: Some("assistant".into()),
                tool_name: Some("tool_search".into()),
                tool_call_id: opt(s(p, "call_id")),
                tool_input: json!({"query":s(p,"query")}).as_object().cloned(),
                ..Default::default()
            },
        )),
        _ => {}
    };
    Ok(events)
}
