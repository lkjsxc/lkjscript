use crate::bindings::{Bindings, expect_bool, expect_i64, expect_text};
use lkjscript::application::ApplicationValue;
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub fn mutation_result(bindings: &Bindings, value: &ApplicationValue) -> Result<Value, String> {
    let (kind, payload) = bindings.expect_sum("mutation_response", value)?;
    let payload = payload.ok_or_else(|| "mutation response omitted its detail".to_owned())?;
    let fields = bindings.expect_product("response_detail", payload)?;
    Ok(json!({
        "kind": kind,
        "task": expect_i64(required(&fields, "task")?)?,
        "code": expect_text(required(&fields, "code")?)?,
    }))
}

pub fn query_result(bindings: &Bindings, value: &ApplicationValue) -> Result<Value, String> {
    let (kind, payload) = bindings.expect_sum("query_result", value)?;
    let payload = payload.ok_or_else(|| "query result omitted its payload".to_owned())?;
    match kind.as_str() {
        "task_view" => Ok(json!({"kind": "task", "task": task_view(bindings, payload)?})),
        "tasks" => {
            let mut page = task_page(bindings, payload)?;
            page["kind"] = Value::String("tasks".to_owned());
            Ok(page)
        }
        "summary" => {
            let fields = bindings.expect_product("project_summary", payload)?;
            Ok(json!({
                "kind": "summary",
                "planned": expect_i64(required(&fields, "planned")?)?,
                "active": expect_i64(required(&fields, "active")?)?,
                "done": expect_i64(required(&fields, "done")?)?,
                "cancelled": expect_i64(required(&fields, "cancelled")?)?,
                "actionable": expect_i64(required(&fields, "actionable")?)?,
                "archived": expect_i64(required(&fields, "archived")?)?,
            }))
        }
        "not_found" => {
            let fields = bindings.expect_product("response_detail", payload)?;
            Ok(json!({
                "kind": "not_found",
                "task": expect_i64(required(&fields, "task")?)?,
                "code": expect_text(required(&fields, "code")?)?,
            }))
        }
        "error" => {
            let fields = bindings.expect_product("response_detail", payload)?;
            Ok(json!({
                "kind": "error",
                "task": expect_i64(required(&fields, "task")?)?,
                "code": expect_text(required(&fields, "code")?)?,
            }))
        }
        "context" => {
            let fields = bindings.expect_product("context_result", payload)?;
            let mut page = task_page(bindings, required(&fields, "page")?)?;
            page["kind"] = Value::String("context".to_owned());
            page["notes_omitted"] = json!(expect_i64(required(&fields, "notes_omitted")?)?);
            page["dependencies_omitted"] =
                json!(expect_i64(required(&fields, "dependencies_omitted")?)?);
            page["text_truncated"] = json!(expect_bool(required(&fields, "text_truncated")?)?);
            Ok(page)
        }
        "activity" => {
            let fields = bindings.expect_product("activity_page", payload)?;
            let items = bindings
                .expect_sequence("activity_sequence", required(&fields, "items")?)?
                .iter()
                .map(|value| activity(bindings, value))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(json!({
                "kind": "activity",
                "items": items,
                "total": expect_i64(required(&fields, "total")?)?,
                "omitted": expect_i64(required(&fields, "omitted")?)?,
                "next_after": expect_i64(required(&fields, "next_after")?)?,
            }))
        }
        _ => Err(format!("unsupported lkjwork query result {kind}")),
    }
}

pub fn human_mutation(result: &Value, revision: u64, published: bool) -> String {
    let kind = result
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let task = result.get("task").and_then(Value::as_i64).unwrap_or(0);
    let code = result
        .get("code")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let target = if task > 0 {
        format!(" #{task}")
    } else {
        String::new()
    };
    if published {
        format!(
            "{kind}{target}: {} (revision {revision})",
            terminal_text(code)
        )
    } else {
        format!(
            "{kind}{target}: {} (no publication; revision {revision})",
            terminal_text(code)
        )
    }
}

pub fn human_query(result: &Value, revision: u64) -> String {
    match result.get("kind").and_then(Value::as_str) {
        Some("task") => result
            .get("task")
            .map(|task| human_task(task, revision))
            .unwrap_or_else(|| "invalid task result".to_owned()),
        Some("tasks") => {
            let mut lines = result
                .get("tasks")
                .and_then(Value::as_array)
                .map(|tasks| tasks.iter().map(human_task_row).collect::<Vec<_>>())
                .unwrap_or_default();
            if lines.is_empty() {
                lines.push("No tasks.".to_owned());
            }
            let omitted = result.get("omitted").and_then(Value::as_i64).unwrap_or(0);
            if omitted > 0 {
                lines.push(format!("{omitted} task(s) omitted"));
            }
            lines.push(format!("revision {revision}"));
            lines.join("\n")
        }
        Some("not_found") => {
            let task = result.get("task").and_then(Value::as_i64).unwrap_or(0);
            format!("Task #{task} was not found (revision {revision}).")
        }
        Some("summary") => format!(
            "planned {} | active {} | done {} | cancelled {} | actionable {} | archived {}\nrevision {}",
            number(result, "planned"),
            number(result, "active"),
            number(result, "done"),
            number(result, "cancelled"),
            number(result, "actionable"),
            number(result, "archived"),
            revision,
        ),
        Some("context") => {
            let mut lines = result
                .get("tasks")
                .and_then(Value::as_array)
                .map(|tasks| tasks.iter().map(human_task_row).collect::<Vec<_>>())
                .unwrap_or_default();
            if lines.is_empty() {
                lines.push("No active or actionable tasks.".to_owned());
            }
            lines.push(format!(
                "omitted tasks={} notes={} dependencies={} text_truncated={} | revision {}",
                number(result, "omitted"),
                number(result, "notes_omitted"),
                number(result, "dependencies_omitted"),
                result
                    .get("text_truncated")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                revision,
            ));
            lines.join("\n")
        }
        Some("error") => format!(
            "Query rejected: {} (revision {}).",
            terminal_text(
                result
                    .get("code")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
            ),
            revision,
        ),
        Some("activity") => {
            let mut lines = result
                .get("items")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .map(|item| {
                            format!(
                                "#{:<4} {:<20} {}",
                                number(item, "task"),
                                terminal_text(
                                    item.get("code").and_then(Value::as_str).unwrap_or("")
                                ),
                                terminal_text(
                                    item.get("actor").and_then(Value::as_str).unwrap_or("")
                                ),
                            )
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            if lines.is_empty() {
                lines.push("No activity.".to_owned());
            }
            lines.push(format!("revision {revision}"));
            lines.join("\n")
        }
        _ => "Unsupported query result.".to_owned(),
    }
}

fn task_page(bindings: &Bindings, value: &ApplicationValue) -> Result<Value, String> {
    let fields = bindings.expect_product("task_page", value)?;
    let tasks = bindings
        .expect_sequence("task_view_sequence", required(&fields, "tasks")?)?
        .iter()
        .map(|value| task_view(bindings, value))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({
        "tasks": tasks,
        "total": expect_i64(required(&fields, "total")?)?,
        "omitted": expect_i64(required(&fields, "omitted")?)?,
        "next_after": expect_i64(required(&fields, "next_after")?)?,
    }))
}

fn task_view(bindings: &Bindings, value: &ApplicationValue) -> Result<Value, String> {
    let fields = bindings.expect_product("task_view", value)?;
    let mut result = task(bindings, required(&fields, "task")?)?;
    result["ready"] = json!(expect_bool(required(&fields, "ready")?)?);
    let blockers = bindings
        .expect_sequence("id_sequence", required(&fields, "blockers")?)?
        .iter()
        .map(expect_i64)
        .collect::<Result<Vec<_>, _>>()?;
    result["blockers"] = json!(blockers);
    Ok(result)
}

fn task(bindings: &Bindings, value: &ApplicationValue) -> Result<Value, String> {
    let fields = bindings.expect_product("task", value)?;
    let (phase, _) = bindings.expect_sum("task_phase", required(&fields, "phase")?)?;
    let (hold_kind, hold_payload) = bindings.expect_sum("task_hold", required(&fields, "hold")?)?;
    let hold = match (hold_kind.as_str(), hold_payload) {
        ("none", None) => Value::Null,
        ("manual", Some(value)) => Value::String(expect_text(value)?.to_owned()),
        _ => return Err("task hold has an invalid payload".to_owned()),
    };
    let labels = bindings
        .expect_sequence("text_sequence", required(&fields, "labels")?)?
        .iter()
        .map(|value| expect_text(value).map(ToOwned::to_owned))
        .collect::<Result<Vec<_>, _>>()?;
    let dependencies = bindings
        .expect_sequence("id_sequence", required(&fields, "dependencies")?)?
        .iter()
        .map(expect_i64)
        .collect::<Result<Vec<_>, _>>()?;
    let notes = bindings
        .expect_sequence("note_sequence", required(&fields, "notes")?)?
        .iter()
        .map(|value| note(bindings, value))
        .collect::<Result<Vec<_>, _>>()?;
    let attachments = bindings
        .expect_sequence("attachment_sequence", required(&fields, "attachments")?)?
        .iter()
        .map(|value| attachment(bindings, value))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({
        "id": expect_i64(required(&fields, "id")?)?,
        "title": expect_text(required(&fields, "title")?)?,
        "description": expect_text(required(&fields, "description")?)?,
        "phase": phase,
        "hold": hold,
        "priority": expect_i64(required(&fields, "priority")?)?,
        "labels": labels,
        "dependencies": dependencies,
        "notes": notes,
        "attachments": attachments,
        "archived": expect_bool(required(&fields, "archived")?)?,
    }))
}

fn note(bindings: &Bindings, value: &ApplicationValue) -> Result<Value, String> {
    let fields = bindings.expect_product("note", value)?;
    Ok(json!({
        "id": expect_i64(required(&fields, "id")?)?,
        "actor": expect_text(required(&fields, "actor")?)?,
        "body": expect_text(required(&fields, "body")?)?,
    }))
}

fn attachment(bindings: &Bindings, value: &ApplicationValue) -> Result<Value, String> {
    let fields = bindings.expect_product("attachment", value)?;
    let ApplicationValue::Bytes(digest) = required(&fields, "digest")? else {
        return Err("attachment digest is not bytes".to_owned());
    };
    Ok(json!({
        "digest": hex(digest.as_slice()),
        "name": expect_text(required(&fields, "name")?)?,
        "length": expect_i64(required(&fields, "length")?)?,
        "actor": expect_text(required(&fields, "actor")?)?,
    }))
}

fn activity(bindings: &Bindings, value: &ApplicationValue) -> Result<Value, String> {
    let fields = bindings.expect_product("activity", value)?;
    Ok(json!({
        "task": expect_i64(required(&fields, "task")?)?,
        "actor": expect_text(required(&fields, "actor")?)?,
        "code": expect_text(required(&fields, "code")?)?,
    }))
}

fn required<'a>(
    fields: &BTreeMap<String, &'a ApplicationValue>,
    name: &str,
) -> Result<&'a ApplicationValue, String> {
    fields
        .get(name)
        .copied()
        .ok_or_else(|| format!("lkjwork application omitted field {name}"))
}

fn human_task(task: &Value, revision: u64) -> String {
    let mut lines = vec![human_task_row(task)];
    if let Some(description) = task.get("description").and_then(Value::as_str)
        && !description.is_empty()
    {
        lines.push(format!("  description: {}", terminal_text(description)));
    }
    if let Some(hold) = task.get("hold").and_then(Value::as_str) {
        lines.push(format!("  hold: {}", terminal_text(hold)));
    }
    if let Some(dependencies) = task.get("dependencies").and_then(Value::as_array)
        && !dependencies.is_empty()
    {
        let values = dependencies
            .iter()
            .filter_map(Value::as_i64)
            .map(|value| format!("#{value}"))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!("  prerequisites: {values}"));
    }
    lines.push(format!("revision {revision}"));
    lines.join("\n")
}

fn human_task_row(task: &Value) -> String {
    format!(
        "#{:<4} {:<9} {:<8} p={:<4} {}",
        number(task, "id"),
        task.get("phase")
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        if task.get("ready").and_then(Value::as_bool).unwrap_or(false) {
            "ready"
        } else {
            "blocked"
        },
        number(task, "priority"),
        terminal_text(task.get("title").and_then(Value::as_str).unwrap_or("")),
    )
}

fn number(value: &Value, field: &str) -> i64 {
    value.get(field).and_then(Value::as_i64).unwrap_or(0)
}

pub fn terminal_text(value: &str) -> String {
    value.chars().flat_map(char::escape_default).collect()
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(char::from(DIGITS[usize::from(byte >> 4)]));
        result.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    result
}
