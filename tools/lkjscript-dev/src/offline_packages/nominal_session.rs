//! Copied-product authorship of a concrete pair session; raw transport belongs to service.
use super::*;
use crate::pure_tail_program::Request;
use serde_json::json;

pub(super) fn workflow(context: &mut Context, standard: &mut Package) -> Result<(), DevError> {
    for (name, kind) in [
        ("SessionEvent", "variant"),
        ("SessionDecisionKind", "variant"),
        ("SessionOutbound", "variant"),
        ("SessionMessageKind", "variant"),
        ("SessionReject", "record"),
        ("SessionClose", "record"),
        ("ByteStream", "interface"),
        ("option-some", "external"),
        ("option-none", "external"),
        ("option-get-or", "external"),
        ("text-concat", "external"),
        ("bytes-to-text", "external"),
        ("json-encode", "external"),
    ] {
        let records = context.cli(
            None,
            &[
                "package", "builtin", "query", "owners", "--kind", kind, "--name", name,
            ],
            true,
        )?;
        standard
            .symbols
            .insert(name.into(), field(&records, "owner", "reference")?);
        if matches!(kind, "variant" | "interface") {
            let parent = field(&records, "owner", "id")?;
            let members = context.cli(
                None,
                &[
                    "package",
                    "builtin",
                    "query",
                    "owners",
                    "--kind",
                    if kind == "interface" {
                        "operation"
                    } else {
                        "case"
                    },
                    "--parent",
                    &parent,
                ],
                true,
            )?;
            for member in members.iter().filter(|record| record.operation == "owner") {
                standard.symbols.insert(
                    format!("{name}.{}", record_field(member, "name")?),
                    record_field(member, "reference")?,
                );
            }
        }
    }
    let mut invalid = context.new_package("nominal-session-invalid")?;
    context.stage(&invalid, standard)?;
    context.apply(&mut invalid, &binding("add", standard))?;
    for (label, argument, repeated, code) in [
        (
            "mismatched-application",
            "i64",
            "text",
            "session_port_state_identity",
        ),
        (
            "phantom-secret",
            "secret",
            "secret",
            "session_state_live_type",
        ),
        (
            "phantom-callable",
            "@Callable",
            "@Callable",
            "session_state_live_type",
        ),
        (
            "nested-secret",
            "@Nested",
            "@Nested",
            "session_state_live_type",
        ),
    ] {
        let path = context
            .evidence
            .join(format!("session-negative-{label}.lkchg"));
        fs::write(
            &path,
            format!(
                "request base={}\n{}",
                invalid.revision,
                invalid_state_program(&standard.symbols, argument, repeated)
            ),
        )?;
        context.reject(
            &invalid,
            &[
                "change",
                "plan",
                "--input-file",
                &path.display().to_string(),
            ],
            code,
        )?;
        context
            .receipt
            .nominal
            .session_rejections
            .push(label.into());
    }
    fs::remove_dir_all(&invalid.path)?;
    let mut package = context.new_package("nominal-session")?;
    context.stage(&package, standard)?;
    context.apply(
        &mut package,
        &format!("{}{}", binding("add", standard), program(&standard.symbols)),
    )?;
    for (arguments, expected) in [
        (
            json!([{"first":0,"second":""},"a"]),
            json!({"first":1,"second":"a"}),
        ),
        (
            json!([{"first":1,"second":"a"},"b"]),
            json!({"first":2,"second":"ab"}),
        ),
    ] {
        let records = context.cli(
            Some(&package.path),
            &["run", "state-step", "--arguments", &arguments.to_string()],
            true,
        )?;
        let actual: serde_json::Value =
            serde_json::from_str(&field(&records, "execution", "value")?)?;
        require(
            actual == expected && field(&records, "execution", "differential")? == "equal",
            "source-bound pair state transition differs",
        )?;
        context.receipt.nominal.state_steps.push(actual);
    }
    context.export(&mut package)?;
    let source = context.evidence.join(format!(
        "transport-{}.lkjp",
        context.receipt.inventories.len()
    ));
    let standalone = context.root.join("nominal-session-standalone");
    fs::create_dir(&standalone)?;
    let artifact = standalone.join("session.lkja");
    context.cli(
        Some(&package.path),
        &["build", "--output", &artifact.display().to_string()],
        true,
    )?;
    let descriptor = json!({"artifact":"session.lkja","target":"live","listen":"127.0.0.1:0",
        "runtime":{"maximum_concurrent_tasks":4,"maximum_queued_tasks":8,"request_deadline_milliseconds":30000,"shutdown_grace_milliseconds":3000,"cancellation_grace_milliseconds":1000},
        "execution":{"instruction_fuel":1000000,"maximum_call_depth":256,"maximum_value_stack":4096},"http":null,
        "session":{"maximum_active_sessions":2,"maximum_pending_handshakes":2,"maximum_message_bytes":4096,"maximum_frame_bytes":4096,"maximum_header_bytes":8192,"maximum_headers":32,"maximum_inbound_mailbox_items":2,"maximum_inbound_mailbox_bytes":8192,"maximum_outbound_mailbox_items":8,"maximum_outbound_mailbox_bytes":8192,"maximum_state_nodes":1024,"maximum_state_bytes":4096,"maximum_transition_messages":8,"maximum_transition_bytes":8192,"tick_interval_milliseconds":1000,"idle_timeout_milliseconds":60000,"maximum_lifetime_milliseconds":86400000,"close_grace_milliseconds":1000,"cancellation_grace_milliseconds":1000,"maximum_process_buffer_bytes":1048576},
        "worker":null,"streams":{"maximum_chunk_bytes":4096,"maximum_buffered_chunks":4,"maximum_total_bytes":65536,"maximum_live_streams":16},"configuration":{},"secrets":[],
        "grants":[{"requirement":"streams","sharing_domain":"nominal-session-streams","authority_revision":"9999999999999999999999999999999999999999999999999999999999999999","adapter":{"kind":"byte_stream"}}]});
    fs::write(
        standalone.join("session.deployment.json"),
        evidence::encode_json(&descriptor)?,
    )?;
    fs::copy(&artifact, context.evidence.join("nominal-session.lkja"))?;
    fs::write(
        context.evidence.join("nominal-session.deployment.json"),
        evidence::encode_json(&descriptor)?,
    )?;
    fs::remove_dir_all(&package.path)?;
    fs::remove_file(&package.container)?;
    let (observation, process) =
        crate::service::nominal::probe(&context.binary, &standalone, &context.evidence)?;
    context.receipt.nominal.session = observation;
    context.receipt.runners.push(process);
    let spec = process::ProcessSpec {
        command: vec![
            std::env::current_exe()?.display().to_string(),
            "nominal-session-state-probe".into(),
            standalone
                .join("session.deployment.json")
                .display()
                .to_string(),
            source.display().to_string(),
            package.transport,
        ],
        cwd: standalone.clone(),
        environment: BTreeMap::from([("LANG".into(), "C.UTF-8".into())]),
        timeout: Duration::from_secs(120),
        maximum_stdout_bytes: MAXIMUM_OUTPUT_BYTES,
        maximum_stderr_bytes: MAXIMUM_OUTPUT_BYTES,
        stdout_path: context.evidence.join("nominal-session-state.stdout"),
        stderr_path: context.evidence.join("nominal-session-state.stderr"),
        unavailable_exit_code: None,
    };
    let terminal = process::run(&spec, &context.evidence);
    fs::remove_dir_all(&standalone)?;
    require(
        terminal.status == process::ProcessStatus::Passed,
        "source-bound nominal session state probe failed",
    )?;
    let states: serde_json::Value = serde_json::from_slice(&process::read_bounded(
        &spec.stdout_path,
        MAXIMUM_OUTPUT_BYTES,
    )?)?;
    require(
        states["effects_replayed"] == false && states["cleanup"]["remaining_tasks"] == 0,
        "source-bound session cleanup or effect isolation failed",
    )?;
    context.receipt.nominal.retained_states = serde_json::from_value(states["states"].clone())?;
    context.receipt.runners.push(terminal);
    Ok(())
}

fn invalid_state_program(
    standard: &BTreeMap<String, String>,
    argument: &str,
    repeated: &str,
) -> String {
    let mut r = Request::default();
    r.text.push_str("create.module as=$module name=invalid-state\ncreate.component as=$component module=$module name=Invalid visibility=private\ncreate.variant as=$Marker module=$module name=Marker visibility=private\nadd.type-parameter as=$T declaration=$Marker name=T\nadd.case as=$absent variant=$Marker name=absent\ntype.function as=@Callable result=unit\ntype.list as=@Nested item=secret\n");
    for (ty, name) in [
        ("event", "SessionEvent"),
        ("kind", "SessionDecisionKind"),
        ("outbound", "SessionOutbound"),
        ("reject", "SessionReject"),
        ("close", "SessionClose"),
    ] {
        r.text.push_str(&format!(
            "type.named as=@{ty} declaration={}\n",
            standard[name]
        ));
    }
    r.text.push_str(&format!("type.application as=@State declaration=$Marker\ntype.argument parent=@State index=0 type={argument}\ntype.application as=@Repeated declaration=$Marker\ntype.argument parent=@Repeated index=0 type={repeated}\ntype.option as=@StateOption item=@State\ntype.option as=@RepeatedOption item=@Repeated\ntype.list as=@messages item=@outbound\ntype.option as=@rejection item=@reject\ntype.option as=@closing item=@close\ntype.structural-record as=@Decision\ntype.field parent=@Decision index=0 name=closing type=@closing\ntype.field parent=@Decision index=1 name=kind type=@kind\ntype.field parent=@Decision index=2 name=messages type=@messages\ntype.field parent=@Decision index=3 name=rejection type=@rejection\ntype.field parent=@Decision index=4 name=state type=@RepeatedOption\n"));
    let closing = r.call(&standard["option-none"], &["@close"], &[]);
    let rejection = r.call(&standard["option-none"], &["@reject"], &[]);
    let state = r.call(&standard["option-none"], &["@Repeated"], &[]);
    let kind = r.expression(
        "variant",
        &format!("case={}", standard["SessionDecisionKind.finish"]),
    );
    let messages = r.expression("list", "item=@outbound");
    let body = r.expression("record", "");
    for (index, (name, value)) in [
        ("closing", closing),
        ("kind", kind),
        ("messages", messages),
        ("rejection", rejection),
        ("state", state),
    ]
    .into_iter()
    .enumerate()
    {
        r.text.push_str(&format!(
            "expression.record-field parent={body} index={index} name={name} value={value}\n"
        ));
    }
    r.function(
        "handler",
        "@Decision",
        &body,
        &[("state", "@StateOption"), ("event", "@event")],
    );
    r.text.push_str("type.function as=@Handler result=@Decision\ntype.argument parent=@Handler index=0 type=@StateOption\ntype.argument parent=@Handler index=1 type=@event\nadd.port as=$port component=$component name=live type=@Handler function=$handler\ncreate.target as=$target name=invalid component=$component port=$port runner=interactive\n");
    r.text
}

fn program(standard: &BTreeMap<String, String>) -> String {
    session_program(standard, None)
}

pub(super) fn session_program(
    standard: &BTreeMap<String, String>,
    recursive: Option<&BTreeMap<String, String>>,
) -> String {
    let mut r = Request::default();
    r.text.push_str("create.module as=$module name=session\ncreate.component as=$component module=$module name=Live visibility=private\n");
    for (ty, name) in [
        ("event", "SessionEvent"),
        ("kind", "SessionDecisionKind"),
        ("outbound", "SessionOutbound"),
        ("message-kind", "SessionMessageKind"),
        ("reject", "SessionReject"),
        ("close", "SessionClose"),
    ] {
        r.text.push_str(&format!(
            "type.named as=@{ty} declaration={}\n",
            standard[name]
        ));
    }
    if let Some(library) = recursive {
        r.text.push_str(&format!("type.application as=@State declaration={}\ntype.argument parent=@State index=0 type=text\n",library["tree"]));
    } else {
        r.text.push_str(&format!("type.application as=@State declaration={}\ntype.argument parent=@State index=0 type=i64\ntype.argument parent=@State index=1 type=text\n",standard["pair"]));
    }
    r.text.push_str(&format!("add.requirement as=$streams component=$component name=streams interface={}\nrequirement.operation parent=$streams index=0 operation={}\nrequirement.limit parent=$streams index=0 name=maximum_calls maximum=64 unit=calls\n",standard["ByteStream"],standard["ByteStream.read-all"]));
    r.text.push_str(
        r#"type.option as=@StateOption item=@State
type.list as=@messages item=@outbound
type.option as=@rejection item=@reject
type.option as=@closing item=@close
type.structural-record as=@Decision
type.field parent=@Decision index=0 name=closing type=@closing
type.field parent=@Decision index=1 name=kind type=@kind
type.field parent=@Decision index=2 name=messages type=@messages
type.field parent=@Decision index=3 name=rejection type=@rejection
type.field parent=@Decision index=4 name=state type=@StateOption
type.structural-record as=@Header
type.field parent=@Header index=0 name=name type=text
type.field parent=@Header index=1 name=value type=bytes
type.list as=@Headers item=@Header
type.structural-record as=@Open
type.field parent=@Open index=0 name=headers type=@Headers
type.field parent=@Open index=1 name=path type=text
type.field parent=@Open index=2 name=query type=text
type.stream as=@Stream item=bytes
type.structural-record as=@Message
type.field parent=@Message index=0 name=body type=@Stream
type.field parent=@Message index=1 name=kind type=@message-kind
type.option as=@Code item=i64
type.structural-record as=@Peer
type.field parent=@Peer index=0 name=code type=@Code
type.field parent=@Peer index=1 name=reason type=text
"#,
    );
    let next = if let Some(library) = recursive {
        let state = r.local("$state-step_state");
        let input = r.local("$state-step_input");
        let leaf = r.expression(
            "variant",
            &format!("case={} payload={input}", library["leaf"]),
        );
        r.types(&leaf, &["text"]);
        let children = r.expression("list", "item=@State");
        r.arguments(&children, &[state, leaf]);
        let next = r.expression(
            "variant",
            &format!("case={} payload={children}", library["branch"]),
        );
        r.types(&next, &["text"]);
        let input = r.local("$state-step_input");
        let empty = r.expression("text", "value=\"\"");
        let reset = r.call(&standard["text-equal"], &[], &[input, empty]);
        let initial = r.call("$initial", &[], &[]);
        r.expression(
            "if",
            &format!("condition={reset} when-true={initial} when-false={next}"),
        )
    } else {
        let state = r.local("$state-step_state");
        let first = r.call(&standard["pair-first"], &["i64", "text"], &[state]);
        let one = r.integer(1);
        let first = r.call(&standard["add"], &[], &[first, one]);
        let state = r.local("$state-step_state");
        let second = r.call(&standard["pair-second"], &["i64", "text"], &[state]);
        let input = r.local("$state-step_input");
        let second = r.call(&standard["text-concat"], &[], &[second, input]);
        r.call(&standard["pair-new"], &["i64", "text"], &[first, second])
    };
    r.function(
        "state-step",
        "@State",
        &next,
        &[("state", "@State"), ("input", "text")],
    );
    r.text.push_str("create.component as=$pure module=$module name=PureSteps visibility=private\ntype.function as=@StateStep result=@State\ntype.argument parent=@StateStep index=0 type=@State\ntype.argument parent=@StateStep index=1 type=text\nadd.port as=$step-port component=$pure name=step type=@StateStep function=$state-step\ncreate.target as=$step-target name=state-step component=$pure port=$step-port runner=command\n");
    let closing = r.call(&standard["option-none"], &["@close"], &[]);
    let rejection = r.call(&standard["option-none"], &["@reject"], &[]);
    let kind = r.local("$decision_kind");
    let messages = r.local("$decision_messages");
    let state = r.local("$decision_state");
    let decision = r.expression("record", "");
    for (index, (name, value)) in [
        ("closing", closing),
        ("kind", kind),
        ("messages", messages),
        ("rejection", rejection),
        ("state", state),
    ]
    .iter()
    .enumerate()
    {
        r.text.push_str(&format!(
            "expression.record-field parent={decision} index={index} name={name} value={value}\n"
        ));
    }
    r.function(
        "decision",
        "@Decision",
        &decision,
        &[
            ("kind", "@kind"),
            ("messages", "@messages"),
            ("state", "@StateOption"),
        ],
    );
    let initial = if let Some(library) = recursive {
        let empty = r.expression("list", "item=@State");
        let result = r.expression(
            "variant",
            &format!("case={} payload={empty}", library["branch"]),
        );
        r.types(&result, &["text"]);
        result
    } else {
        let zero = r.integer(0);
        let empty = r.expression("text", "value=\"\"");
        r.call(&standard["pair-new"], &["i64", "text"], &[zero, empty])
    };
    r.function("initial", "@State", &initial, &[]);
    let initial = r.call("$initial", &[], &[]);
    let option_initial = r.call(&standard["option-some"], &["@State"], &[initial]);
    let no_messages = r.expression("list", "item=@outbound");
    let accept = r.expression(
        "variant",
        &format!("case={}", standard["SessionDecisionKind.accept"]),
    );
    let opened = r.call("$decision", &[], &[accept, no_messages, option_initial]);
    let none = r.call(&standard["option-none"], &["@State"], &[]);
    let finish = r.expression(
        "variant",
        &format!("case={}", standard["SessionDecisionKind.finish"]),
    );
    let no_messages = r.expression("list", "item=@outbound");
    let finished = r.call("$decision", &[], &[finish, no_messages, none]);
    r.function("finished", "@Decision", &finished, &[]);
    let state = r.local("$state");
    let continuing = r.expression(
        "variant",
        &format!("case={}", standard["SessionDecisionKind.continue"]),
    );
    let no_messages = r.expression("list", "item=@outbound");
    let tick = r.call("$decision", &[], &[continuing, no_messages, state]);
    let message = r.local("$message");
    let stream = r.field(&message, "body");
    let max = r.integer(4096);
    let bytes = r.capability("$streams", &standard["ByteStream.read-all"], &[stream, max]);
    let text = r.call(&standard["bytes-to-text"], &[], &[bytes]);
    let state = r.local("$state");
    let initial = r.call("$initial", &[], &[]);
    let current = r.call(&standard["option-get-or"], &["@State"], &[state, initial]);
    let next = r.call("$state-step", &[], &[current, text]);
    let next_value = r.local("$next");
    let encoded = if let Some(library) = recursive {
        r.text.push_str("type.structural-record as=@Reply\ntype.field parent=@Reply index=0 name=contents type=text\ntype.field parent=@Reply index=1 name=state type=@State\n");
        let callback = r.function_value(&standard["text-concat"]);
        let empty = r.expression("text", "value=\"\"");
        let contents = r.call(
            &library["tree-fold"],
            &["text", "text"],
            &[next_value, callback, empty],
        );
        let state = r.local("$next");
        let reply = r.expression("record", "");
        r.text.push_str(&format!("expression.record-field parent={reply} index=0 name=contents value={contents}\nexpression.record-field parent={reply} index=1 name=state value={state}\n"));
        r.call(&standard["json-encode"], &["@Reply"], &[reply])
    } else {
        r.call(&standard["json-encode"], &["@State"], &[next_value])
    };
    let encoded = r.call(&standard["bytes-to-text"], &[], &[encoded]);
    let output = r.expression(
        "variant",
        &format!(
            "case={} payload={encoded}",
            standard["SessionOutbound.text"]
        ),
    );
    let messages = r.expression("list", "item=@outbound");
    r.arguments(&messages, &[output]);
    let next_value = r.local("$next");
    let next_state = r.call(&standard["option-some"], &["@State"], &[next_value]);
    let kind = r.expression(
        "variant",
        &format!("case={}", standard["SessionDecisionKind.continue"]),
    );
    let continuing = r.call("$decision", &[], &[kind, messages, next_state]);
    let message_body = r.expression("let", &format!("body={continuing}"));
    r.text.push_str(&format!("expression.binding parent={message_body} index=0 as=$next name=next value={next} type=@State\n"));
    let event = r.local("$event");
    let body = r.expression("match", &format!("value={event}"));
    let peer_finished = r.call("$finished", &[], &[]);
    let shutdown_finished = r.call("$finished", &[], &[]);
    for (index, (name, payload, branch)) in [
        ("open", Some(("$opened", "@Open")), opened),
        ("message", Some(("$message", "@Message")), message_body),
        ("tick", None, tick),
        ("peer-close", Some(("$peer", "@Peer")), peer_finished),
        ("shutdown", None, shutdown_finished),
    ]
    .iter()
    .enumerate()
    {
        r.text.push_str(&format!(
            "expression.match-arm parent={body} index={index} case={} body={branch}",
            standard[&format!("SessionEvent.{name}")]
        ));
        if let Some((binding, ty)) = payload {
            r.text
                .push_str(&format!(" as={binding} name=payload type={ty}"));
        }
        r.text.push('\n');
    }
    r.text.push_str(&format!("create.function as=$handler module=$module name=handler visibility=private result=@Decision effect=task body={body}\nadd.parameter as=$state function=$handler name=state type=@StateOption\nadd.parameter as=$event function=$handler name=event type=@event\neffect.requirement parent=$handler index=0 requirement=$streams\ntype.function as=@Handler result=@Decision\ntype.argument parent=@Handler index=0 type=@StateOption\ntype.argument parent=@Handler index=1 type=@event\nadd.port as=$live component=$component name=live type=@Handler function=$handler\ncreate.target as=$target name=live component=$component port=$live runner=interactive\n"));
    r.text
}
