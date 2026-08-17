#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use lkjscript::error::ErrorCode;
use lkjscript::machine::active_machine_schema_digest;
use lkjscript::protocol::Response;
use lkjscript::schema::NodeKind;
use lkjscript::transaction::{TransactionMode, TransactionReceipt};
use lkjscript::workbench::{ContextPacket, decode_context_packet, parse_edit_document};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

fn cli(arguments: &[String], input: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_lkjscript"))
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn CLI");
    child
        .stdin
        .take()
        .expect("CLI stdin")
        .write_all(input)
        .expect("write CLI input");
    child.wait_with_output().expect("CLI output")
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn packet_command(state: &Path, workspace: &str, revision: u64) -> Vec<String> {
    vec![
        "agent".into(),
        "context".into(),
        "--state".into(),
        state.display().to_string(),
        "--workspace".into(),
        workspace.into(),
        "--revision".into(),
        revision.to_string(),
        "--purpose".into(),
        "orient".into(),
    ]
}

fn transaction_receipt(output: &Output) -> TransactionReceipt {
    assert!(
        output.status.success(),
        "workbench action failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    match serde_json::from_slice::<Response>(&output.stdout).expect("logical response") {
        Response::TransactionReceipt(receipt) => receipt,
        response => panic!("unexpected response: {response:?}"),
    }
}

#[test]
fn public_workbench_packet_view_document_and_stale_rejection_are_exact() {
    let temporary = tempfile::tempdir().expect("state directory");
    let create_arguments = vec![
        "agent".into(),
        "create".into(),
        "--state".into(),
        temporary.path().display().to_string(),
    ];
    let created = cli(&create_arguments, &[]);
    assert!(created.status.success());
    let Response::WorkspaceCreated(created) =
        serde_json::from_slice::<Response>(&created.stdout).expect("workspace response")
    else {
        panic!("workspace response");
    };
    let workspace = created.workspace.to_string();

    let context_arguments = packet_command(temporary.path(), &workspace, 0);
    let first_packet_output = cli(&context_arguments, &[]);
    let second_packet_output = cli(&context_arguments, &[]);
    assert!(first_packet_output.status.success());
    assert_eq!(first_packet_output.stdout, second_packet_output.stdout);
    let first_packet = decode_context_packet(&first_packet_output.stdout).expect("context packet");
    assert_eq!(first_packet.payload.nodes.len(), 1);
    assert_eq!(
        first_packet.payload.aliases[0].kind,
        NodeKind::WorkspaceRoot
    );

    let packet_zero_path = temporary.path().join("revision-zero.packet.json");
    fs::write(&packet_zero_path, &first_packet_output.stdout).expect("write packet");
    let view_arguments = vec![
        "agent".into(),
        "view".into(),
        "--packet".into(),
        packet_zero_path.display().to_string(),
        "--ids".into(),
    ];
    let first_view = cli(&view_arguments, &[]);
    let second_view = cli(&view_arguments, &[]);
    assert!(first_view.status.success());
    assert_eq!(first_view.stdout, second_view.stdout);
    let rendered = String::from_utf8(first_view.stdout).expect("UTF-8 view");
    assert!(rendered.contains("lkjscript semantic review v2"));
    assert!(rendered.contains("purpose orient"));
    assert!(rendered.contains("@n1="));
    assert!(!rendered.contains('\u{1b}'));

    let schema = active_machine_schema_digest().expect("active schema");
    let create_document = format!(
        "document {{ version 1 schema {schema} workspace {workspace} base_revision 0 scope (workspace) edits [ \
         (create_package {{ symbol draft_1 name demo }}) \
         (create_module {{ symbol draft_2 package (draft draft_1) name focused }}) \
         (create_module {{ symbol draft_3 package (draft draft_1) name unrelated }}) \
         (create_function {{ symbol draft_4 module (draft draft_2) name score parameters [] result i64 \
           body {{ operations [ {{ symbol initial operation (const_i64 7) }} ] \
                  return_value (operation_result {{ operation (draft initial) output 0 }}) }} }}) \
         (set_entry_function {{ package (draft draft_1) function (draft draft_4) }}) \
         ] return_symbols [ draft_1 draft_2 draft_3 draft_4 ] }}"
    );
    let validate_arguments = vec![
        "agent".into(),
        "validate".into(),
        "--state".into(),
        temporary.path().display().to_string(),
    ];
    let apply_arguments = vec![
        "agent".into(),
        "apply".into(),
        "--state".into(),
        temporary.path().display().to_string(),
    ];
    let predicted = transaction_receipt(&cli(&validate_arguments, create_document.as_bytes()));
    assert!(!predicted.published);
    let committed = transaction_receipt(&cli(&apply_arguments, create_document.as_bytes()));
    assert!(committed.published);
    let mut expected = predicted.clone();
    expected.published = true;
    assert_eq!(committed, expected);
    let bindings: std::collections::BTreeMap<_, _> = committed
        .returned_bindings
        .iter()
        .map(|(symbol, node)| (symbol.to_string(), *node))
        .collect();
    let focused_module = bindings["draft_2"];
    let unrelated_module = bindings["draft_3"];
    let function = bindings["draft_4"];

    let focused_arguments = vec![
        "agent".into(),
        "context".into(),
        "--state".into(),
        temporary.path().display().to_string(),
        "--workspace".into(),
        workspace.clone(),
        "--revision".into(),
        "1".into(),
        "--purpose".into(),
        "refactor".into(),
        "--target".into(),
        focused_module.to_string(),
    ];
    let focused_output = cli(&focused_arguments, &[]);
    assert!(focused_output.status.success());
    let focused_packet =
        decode_context_packet(&focused_output.stdout).expect("focused context packet");
    let focused_nodes: std::collections::BTreeSet<_> = focused_packet
        .payload
        .nodes
        .iter()
        .map(|view| view.summary.node)
        .collect();
    assert!(focused_nodes.contains(&focused_module));
    assert!(
        !focused_nodes.contains(&unrelated_module),
        "owner traversal must not pull unrelated siblings into target context"
    );

    let context_one_output = cli(&packet_command(temporary.path(), &workspace, 1), &[]);
    assert!(context_one_output.status.success());
    let context_one: ContextPacket =
        decode_context_packet(&context_one_output.stdout).expect("revision-one packet");
    let mut unchanged_arguments = packet_command(temporary.path(), &workspace, 1);
    unchanged_arguments.extend(["--known-digest".to_owned(), context_one.digest.to_string()]);
    let unchanged = cli(&unchanged_arguments, &[]);
    assert!(unchanged.status.success());
    let unchanged: serde_json::Value =
        serde_json::from_slice(&unchanged.stdout).expect("unchanged context response");
    assert_eq!(unchanged["version"], 2);
    assert_eq!(unchanged["digest"], context_one.digest.to_string());
    assert_eq!(unchanged["unchanged"], true);
    assert!(
        serde_json::to_vec(&unchanged)
            .expect("unchanged JSON")
            .len()
            < context_one_output.stdout.len()
    );
    let package_alias = context_one
        .payload
        .aliases
        .iter()
        .find(|alias| alias.kind == NodeKind::Package)
        .expect("package alias")
        .alias
        .clone();
    let packet_one_path = temporary.path().join("revision-one.packet.json");
    fs::write(&packet_one_path, &context_one_output.stdout).expect("write revision-one packet");
    let rename_document = format!(
        "document {{ version 1 schema {schema} packet {} workspace {workspace} base_revision 1 scope (workspace) edits [ \
         (rename_node {{ node (existing @{package_alias}) name demo_renamed }}) ] return_symbols [] }}",
        context_one.digest
    );
    let packet_validate_arguments = vec![
        "agent".into(),
        "validate".into(),
        "--state".into(),
        temporary.path().display().to_string(),
        "--packet".into(),
        packet_one_path.display().to_string(),
    ];
    let packet_apply_arguments = vec![
        "agent".into(),
        "apply".into(),
        "--state".into(),
        temporary.path().display().to_string(),
        "--packet".into(),
        packet_one_path.display().to_string(),
    ];
    let predicted_rename =
        transaction_receipt(&cli(&packet_validate_arguments, rename_document.as_bytes()));
    let committed_rename =
        transaction_receipt(&cli(&packet_apply_arguments, rename_document.as_bytes()));
    let mut expected_rename = predicted_rename;
    expected_rename.published = true;
    assert_eq!(committed_rename, expected_rename);

    let stale = cli(&packet_apply_arguments, rename_document.as_bytes());
    assert!(
        stale.status.success(),
        "semantic rejection is a valid response"
    );
    match serde_json::from_slice::<Response>(&stale.stdout).expect("stale response") {
        Response::Error(error) => assert_eq!(error.code, ErrorCode::RevisionConflict),
        response => panic!("expected stale-base rejection, received {response:?}"),
    }

    let review_arguments = vec![
        "agent".into(),
        "context".into(),
        "--state".into(),
        temporary.path().display().to_string(),
        "--workspace".into(),
        workspace.clone(),
        "--revision".into(),
        "2".into(),
        "--purpose".into(),
        "review".into(),
        "--from-revision".into(),
        "1".into(),
    ];
    let review = cli(&review_arguments, &[]);
    assert!(review.status.success());
    let review_path = temporary.path().join("review.packet.json");
    fs::write(&review_path, &review.stdout).expect("write review packet");
    let diff = cli(
        &[
            "agent".into(),
            "diff".into(),
            "--packet".into(),
            review_path.display().to_string(),
        ],
        &[],
    );
    assert!(diff.status.success());
    let diff = String::from_utf8(diff.stdout).expect("UTF-8 diff");
    assert!(diff.contains("renamed \"demo\" -> \"demo_renamed\""));

    let function_context_arguments = vec![
        "agent".into(),
        "context".into(),
        "--state".into(),
        temporary.path().display().to_string(),
        "--workspace".into(),
        workspace.clone(),
        "--revision".into(),
        "2".into(),
        "--purpose".into(),
        "refactor".into(),
        "--target".into(),
        function.to_string(),
    ];
    let function_packet_output = cli(&function_context_arguments, &[]);
    assert!(function_packet_output.status.success());
    let function_packet =
        decode_context_packet(&function_packet_output.stdout).expect("function context packet");
    let function_packet_path = temporary.path().join("function.packet.json");
    fs::write(&function_packet_path, &function_packet_output.stdout)
        .expect("write function packet");
    let rendered_document = cli(
        &[
            "agent".into(),
            "document".into(),
            "--packet".into(),
            function_packet_path.display().to_string(),
        ],
        &[],
    );
    assert!(
        rendered_document.status.success(),
        "document render failed: {}",
        String::from_utf8_lossy(&rendered_document.stderr)
    );
    assert!(rendered_document.stdout.starts_with(b"document {"));
    parse_edit_document(
        &rendered_document.stdout,
        TransactionMode::ValidateOnly,
        Some(&function_packet),
    )
    .expect("rendered function document parses");
    let no_op = cli(
        &[
            "agent".into(),
            "validate".into(),
            "--state".into(),
            temporary.path().display().to_string(),
            "--packet".into(),
            function_packet_path.display().to_string(),
        ],
        &rendered_document.stdout,
    );
    assert!(no_op.status.success());
    match serde_json::from_slice::<Response>(&no_op.stdout).expect("no-op response") {
        Response::Error(error) => assert_eq!(error.code, ErrorCode::NoChange),
        response => panic!("expected rendered no-op rejection, received {response:?}"),
    }
    let edited_document = String::from_utf8(rendered_document.stdout)
        .expect("UTF-8 document")
        .replacen("(const_i64 7)", "(const_i64 8)", 1);
    assert!(edited_document.contains("(const_i64 8)"));
    let edited = transaction_receipt(&cli(
        &[
            "agent".into(),
            "apply".into(),
            "--state".into(),
            temporary.path().display().to_string(),
            "--packet".into(),
            function_packet_path.display().to_string(),
        ],
        edited_document.as_bytes(),
    ));
    assert_eq!(edited.revision.get(), 3);
    assert_eq!(edited.created_count, 0);

    let malformed = cli(
        &strings(&[
            "agent",
            "validate",
            "--state",
            temporary.path().to_str().expect("UTF-8 state"),
        ]),
        b"document { workspace }",
    );
    assert_eq!(malformed.status.code(), Some(2));
    let malformed: serde_json::Value =
        serde_json::from_slice(&malformed.stdout).expect("document error JSON");
    assert_eq!(malformed["kind"], "document_error");
    assert_eq!(malformed["error"]["line"], 1);
}

#[test]
fn editable_document_performs_an_independent_variant_replacement() {
    let temporary = tempfile::tempdir().expect("state directory");
    let state = temporary.path().display().to_string();
    let created = cli(&strings(&["agent", "create", "--state", &state]), &[]);
    let Response::WorkspaceCreated(created) =
        serde_json::from_slice::<Response>(&created.stdout).expect("workspace response")
    else {
        panic!("workspace response");
    };
    let workspace = created.workspace.to_string();
    let schema = active_machine_schema_digest().expect("active schema");
    let apply = strings(&["agent", "apply", "--state", &state]);
    let first = format!(
        "document {{ version 1 schema {schema} workspace {workspace} base_revision 0 \
         scope (workspace) edits [ \
         (create_package {{ symbol package name migration }}) \
         (create_module {{ symbol module package (draft package) name api }}) \
         (create_sum_type {{ symbol old_choice module (draft module) name Choice variants [ \
           {{ symbol old_first name first }} {{ symbol old_second name second }} ] }}) \
         (create_function {{ symbol old_entry module (draft module) name choose parameters [] \
           result {{ nominal (draft old_choice) }} body {{ operations [ \
             {{ symbol selected operation (construct_variant {{ variant (draft old_first) }}) }} \
           ] return_value (operation_result {{ operation (draft selected) output 0 }}) }} }}) \
         (set_entry_function {{ package (draft package) function (draft old_entry) }}) \
         ] return_symbols [ package module old_choice old_first old_second old_entry ] }}"
    );
    let first = transaction_receipt(&cli(&apply, first.as_bytes()));
    let old: std::collections::BTreeMap<_, _> = first
        .returned_bindings
        .iter()
        .map(|(symbol, node)| (symbol.to_string(), *node))
        .collect();

    let blocked = format!(
        "document {{ version 1 schema {schema} workspace {workspace} base_revision 1 \
         scope (workspace) edits [ (delete_owned_subtree {{ root (existing \"{}\") }}) ] \
         return_symbols [] }}",
        old["old_choice"]
    );
    let blocked = cli(&apply, blocked.as_bytes());
    assert!(blocked.status.success());
    match serde_json::from_slice::<Response>(&blocked.stdout).expect("blocked response") {
        Response::Error(error) => assert_eq!(error.code, ErrorCode::DeleteBlocked),
        response => panic!("expected blocked variant deletion, received {response:?}"),
    }

    let replacement = format!(
        "document {{ version 1 schema {schema} workspace {workspace} base_revision 1 \
         scope (workspace) edits [ \
         (create_sum_type {{ symbol new_choice module (existing \"{}\") name Decision variants [ \
           {{ symbol new_second name fallback }} {{ symbol new_first name selected }} \
           {{ symbol new_third name unavailable }} ] }}) \
         (create_function {{ symbol new_entry module (existing \"{}\") name choose parameters [] \
           result {{ nominal (draft new_choice) }} body {{ operations [ \
             {{ symbol selected operation (construct_variant {{ variant (draft new_first) }}) }} \
           ] return_value (operation_result {{ operation (draft selected) output 0 }}) }} }}) \
         (set_entry_function {{ package (existing \"{}\") function (draft new_entry) }}) \
         (delete_owned_subtree {{ root (existing \"{}\") }}) \
         (delete_owned_subtree {{ root (existing \"{}\") }}) \
         ] return_symbols [ new_choice new_second new_first new_third new_entry ] }}",
        old["module"], old["module"], old["package"], old["old_entry"], old["old_choice"]
    );
    eprintln!("variant_migration_document_bytes={}", replacement.len());
    assert!(replacement.len() < 2_000);
    let second = transaction_receipt(&cli(&apply, replacement.as_bytes()));
    assert_eq!(second.revision.get(), 2);
    let new: std::collections::BTreeMap<_, _> = second
        .returned_bindings
        .iter()
        .map(|(symbol, node)| (symbol.to_string(), *node))
        .collect();
    assert_ne!(old["old_choice"], new["new_choice"]);
    assert_ne!(old["old_first"], new["new_first"]);

    let run = format!(
        "run {{ workspace {workspace} revision 2 entry \"{}\" arguments [] \
         policy {{ fuel 1000 maximum_frames 32 }} }}",
        new["new_entry"]
    );
    let output = cli(
        &strings(&["agent", "run", "--state", &state]),
        run.as_bytes(),
    );
    assert!(output.status.success());
    let Response::Run(result) =
        serde_json::from_slice::<Response>(&output.stdout).expect("run response")
    else {
        panic!("run response");
    };
    let lkjscript::interpret::RuntimeValue::Sum {
        ty,
        variant,
        payload,
    } = result.value
    else {
        panic!("variant result");
    };
    assert_eq!(ty, new["new_choice"]);
    assert_eq!(variant, new["new_first"]);
    assert!(payload.is_none());
}

fn mutate_boundary(source: &[u8], seed: u64, case: u64) -> Vec<u8> {
    let mut state = seed ^ case.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    state ^= state << 13;
    state ^= state >> 7;
    state ^= state << 17;
    let index = usize::try_from(state).unwrap_or(usize::MAX) % source.len().max(1);
    let mut mutation = source.to_vec();
    match case % 6 {
        0 if !mutation.is_empty() => mutation[index] ^= 1_u8 << (state % 8),
        1 if !mutation.is_empty() => {
            mutation.remove(index);
        }
        2 => mutation.insert(index.min(mutation.len()), b'!'),
        3 => mutation.truncate(index.min(mutation.len())),
        4 => mutation.extend_from_slice(b"{}"),
        _ if !mutation.is_empty() => mutation[index] = 0xff,
        _ => mutation.push(0xff),
    }
    mutation
}

fn run_workbench_boundary_mutation(seed: u64, cases: u64) {
    let temporary = tempfile::tempdir().expect("state directory");
    let created = cli(
        &strings(&[
            "agent",
            "create",
            "--state",
            temporary.path().to_str().expect("UTF-8 state"),
        ]),
        &[],
    );
    let Response::WorkspaceCreated(created) =
        serde_json::from_slice::<Response>(&created.stdout).expect("workspace response")
    else {
        panic!("workspace response");
    };
    let workspace = created.workspace.to_string();
    let packet_bytes = cli(&packet_command(temporary.path(), &workspace, 0), &[]).stdout;
    let packet = decode_context_packet(&packet_bytes).expect("context packet");
    let root_alias = &packet.payload.aliases[0].alias;
    let schema = active_machine_schema_digest().expect("active schema");
    let document = format!(
        "document {{ version 1 schema {schema} packet {} workspace {workspace} base_revision 0 scope (workspace) edits [ \
         (rename_node {{ node (existing @{root_alias}) name trial }}) ] return_symbols [] }}",
        packet.digest
    );
    parse_edit_document(
        document.as_bytes(),
        TransactionMode::ValidateOnly,
        Some(&packet),
    )
    .expect("valid mutation-document corpus");

    for case in 0..cases {
        if case % 32 == 0 {
            let mutation = mutate_boundary(&packet_bytes, seed, case);
            assert_eq!(
                decode_context_packet(&mutation),
                decode_context_packet(&mutation),
                "packet mutation {case} is nondeterministic"
            );
        } else {
            let mutation = mutate_boundary(document.as_bytes(), seed, case);
            assert_eq!(
                parse_edit_document(&mutation, TransactionMode::ValidateOnly, Some(&packet)),
                parse_edit_document(&mutation, TransactionMode::ValidateOnly, Some(&packet)),
                "document mutation {case} is nondeterministic"
            );
        }
    }
}

#[test]
fn deterministic_workbench_boundary_mutation_normal_smoke() {
    run_workbench_boundary_mutation(0x776f_726b_6265_6e63, 600);
}

/// Deterministic mutation smoke, not coverage-guided fuzzing.
///
/// Reproduce with:
/// `LKJSCRIPT_WORKBENCH_MUTATION_SEED=1 LKJSCRIPT_WORKBENCH_MUTATION_CASES=10000 cargo test --release --test workbench workbench_boundary_mutation_smoke -- --ignored --nocapture --test-threads=1`
#[test]
#[ignore = "bounded manual deterministic workbench mutation smoke"]
fn workbench_boundary_mutation_smoke() {
    let seed = std::env::var("LKJSCRIPT_WORKBENCH_MUTATION_SEED")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(1);
    let cases = std::env::var("LKJSCRIPT_WORKBENCH_MUTATION_CASES")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(10_000);
    eprintln!(
        "deterministic workbench mutation smoke (not coverage-guided): seed={seed} cases={cases}"
    );
    run_workbench_boundary_mutation(seed, cases);
}
