use lkjscript_core::ExecutionOutcome;

pub fn outcome(outcome: &ExecutionOutcome) -> String {
    match outcome {
        ExecutionOutcome::Returned(value) => {
            let (kind, exact) = if value.is_unit() {
                ("unit", "unit".to_string())
            } else if value.is_empty_list() {
                ("empty-list", "empty-list".to_string())
            } else if value.enum_physical_tag() == Some(1) && value.enum_payload_len() == Some(0) {
                ("none", "none".to_string())
            } else if let Some(value) = value.as_bool() {
                ("bool", value.to_string())
            } else if let Some(value) = value.as_i64() {
                ("i64", value.to_string())
            } else if let Some(value) = value.as_f64() {
                ("f64-bits", format!("0x{:016x}", value.to_bits()))
            } else if let Some(value) = value.as_str() {
                ("str-or-symbol", value.to_string())
            } else if let Some(value) = value.as_resource() {
                ("resource", value.to_string())
            } else {
                ("owned-value", format!("{value:?}"))
            };
            format!(
                "{{\"kind\":\"returned\",\"value_kind\":{},\"exact\":{}}}",
                string(kind),
                string(&exact)
            )
        }
        ExecutionOutcome::Exited(code) => {
            format!("{{\"kind\":\"exited\",\"code\":{code}}}")
        }
        ExecutionOutcome::Trapped(trap) => format!(
            "{{\"kind\":\"trapped\",\"detail\":{}}}",
            string(trap.as_str())
        ),
        ExecutionOutcome::DeadlineExceeded => "{\"kind\":\"deadline-exceeded\"}".to_string(),
        ExecutionOutcome::ResourceLimitExceeded(kind) => format!(
            "{{\"kind\":\"resource-limit-exceeded\",\"resource\":{}}}",
            string(&format!("{kind:?}"))
        ),
        ExecutionOutcome::HostFailure(error) => format!(
            "{{\"kind\":\"host-failure\",\"detail\":{}}}",
            string(error.as_str())
        ),
        ExecutionOutcome::CleanupFailed { primary, failures } => format!(
            "{{\"kind\":\"cleanup-failed\",\"primary\":{},\"cleanup\":{}}}",
            self::outcome(primary),
            cleanup(failures)
        ),
    }
}

fn cleanup(failures: &lkjscript_core::CleanupFailures) -> String {
    let records = failures
        .retained()
        .iter()
        .map(|failure| {
            format!(
                concat!(
                    "{{\"phase\":{},\"subject\":{},\"detail\":{},",
                    "\"omitted_message_bytes\":{}}}"
                ),
                string(failure.phase().as_str()),
                string(failure.subject().as_str()),
                string(failure.message()),
                failure.omitted_message_bytes()
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        concat!(
            "{{\"retained\":[{}],\"retained_message_bytes\":{},",
            "\"omitted_message_bytes\":{},\"omitted_failures\":{}}}"
        ),
        records,
        failures.retained_message_bytes(),
        failures.omitted_message_bytes(),
        failures.omitted_failures()
    )
}

pub fn string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len().saturating_add(2));
    escaped.push('"');
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character.is_control() => {
                escaped.push_str(&format!("\\u{:04x}", u32::from(character)));
            }
            character => escaped.push(character),
        }
    }
    escaped.push('"');
    escaped
}
