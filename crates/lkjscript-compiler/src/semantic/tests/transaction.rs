use super::*;
use crate::semantic::schema::{ResponseResult, SnapshotResult};

fn take_snapshot(root: &std::path::Path) -> (String, SnapshotResult) {
    let output = crate::semantic::execute(&request(root, "{\"kind\":\"snapshot\"}"))
        .expect("snapshot request");
    let decoded = response(&output);
    let revision = decoded
        .revision
        .unwrap_or_else(|| panic!("snapshot had no revision: {:?}", decoded.result));
    let ResponseResult::Snapshot { snapshot } = decoded.result else {
        panic!("expected snapshot response");
    };
    (revision, *snapshot)
}

fn preconditions(snapshot: &SnapshotResult) -> String {
    snapshot
        .source_units
        .iter()
        .map(|file| {
            format!(
                "{{\"path\":{},\"bytes\":{},\"sha256\":\"{}\"}}",
                serde_json::to_string(&file.path).expect("path JSON"),
                file.bytes,
                file.sha256
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

#[test]
fn cross_import_rename_and_expression_replacement_are_atomic() {
    let directory = case_dir("transactions");
    let root = directory.join("main.lkjscript");
    let library = directory.join("lib.lkjscript");
    let original_root = concat!(
        "imports/\nimport/\nmodule/\nlib.lkjscript\n/module\ndeclarations/\ninc\n/declarations\n/import\n/imports\n;; inc stays in a comment\n",
        "def/\nname/\ntext\n/name\nfn/\nsig/\ninputs/\n/inputs\noutput/\nstring\n/output\n/sig\n",
        "params/\n/params\nstring-literal/\ninc stays in a string\n/string-literal\n/fn\n/def\n",
        "def/\nname/\nshadow\n/name\nfn/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nparams/\n/params\n",
        "let/\nbind/\ninc\nunit\n/bind\ninc\n/let\n/fn\n/def\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\ninc/\n2\n/inc\n/main\n",
    );
    let original_library = concat!(
        "def/\nname/\ninc\n/name\npublic\nfn/\nsig/\ninputs/\ni64\n/inputs\noutput/\ni64\n/output\n/sig\n",
        "params/\nx\ni64\n/params\nadd/\nx\n1\n/add\n/fn\n/def\n",
    );
    std::fs::write(&root, original_root).expect("write root");
    std::fs::write(&library, original_library).expect("write library");
    let (revision, snapshot) = take_snapshot(&root);
    let declaration = snapshot
        .declarations
        .iter()
        .find(|declaration| declaration.name == "inc")
        .expect("inc declaration");
    let operation = format!(
        concat!(
            "{{\"kind\":\"apply-transaction\",\"mode\":\"preview\",",
            "\"base_revision\":\"{revision}\",\"file_preconditions\":[{}],",
            "\"operations\":[{{\"kind\":\"rename-declaration\",",
            "\"declaration_key\":\"{}\",\"entity_fingerprint\":\"{}\",",
            "\"new_name\":\"increment\"}}]}}",
        ),
        preconditions(&snapshot),
        declaration.key,
        declaration.fingerprint,
        revision = revision
    );
    let collision = operation.replace("new_name\":\"increment", "new_name\":\"text");
    let rejected = response(
        &crate::semantic::execute(&request(&root, &collision)).expect("collision response"),
    );
    assert!(matches!(rejected.result, ResponseResult::Error { .. }));
    assert_eq!(
        std::fs::read_to_string(&root).expect("root after collision"),
        original_root
    );
    assert_eq!(
        std::fs::read_to_string(&library).expect("library after collision"),
        original_library
    );
    let preview =
        response(&crate::semantic::execute(&request(&root, &operation)).expect("preview rename"));
    assert!(matches!(
        preview.result,
        ResponseResult::ApplyTransaction { .. }
    ));
    assert_eq!(
        std::fs::read_to_string(&root).expect("root unchanged"),
        original_root
    );
    let publish = operation.replace("\"mode\":\"preview\"", "\"mode\":\"publish\"");
    let published =
        response(&crate::semantic::execute(&request(&root, &publish)).expect("publish rename"));
    assert!(matches!(
        published.result,
        ResponseResult::ApplyTransaction { .. }
    ));
    let renamed_root = std::fs::read_to_string(&root).expect("renamed root");
    let renamed_library = std::fs::read_to_string(&library).expect("renamed library");
    assert!(renamed_root.contains("increment/\n2\n/increment"));
    assert!(renamed_root.contains(";; inc stays in a comment"));
    assert!(renamed_root.contains("inc stays in a string"));
    assert!(renamed_root.contains("bind/\ninc\nunit\n/bind\ninc\n/let"));
    assert!(renamed_library.contains("name/\nincrement\n/name"));

    let (revision, snapshot) = take_snapshot(&root);
    let main = snapshot
        .declarations
        .iter()
        .find(|declaration| declaration.name == "$main")
        .expect("main declaration");
    let call = snapshot
        .nodes
        .iter()
        .find(|node| {
            matches!(
                &node.value,
                Some(crate::semantic::schema::SemanticNodeValue::UserFunction { name })
                    if name == "increment"
            ) && node.declaration.as_deref() == Some(&main.key)
        })
        .expect("main call");
    let replace = |kind: &str, value: &str| {
        format!(
            concat!(
                "{{\"kind\":\"apply-transaction\",\"mode\":\"preview\",",
                "\"base_revision\":\"{revision}\",\"file_preconditions\":[{}],",
                "\"operations\":[{{\"kind\":\"replace-expression\",",
                "\"declaration_key\":\"{}\",\"entity_fingerprint\":\"{}\",",
                "\"node\":{},\"node_fingerprint\":\"{}\",",
                "\"expression\":{{\"kind\":\"{kind}\",\"value\":{value}}}}}]}}",
            ),
            preconditions(&snapshot),
            main.key,
            main.fingerprint,
            call.index,
            call.fingerprint,
            revision = revision,
            kind = kind,
            value = value
        )
    };
    let before_failure = std::fs::read(&root).expect("bytes before failed replacement");
    let failed = response(
        &crate::semantic::execute(&request(&root, &replace("string", "\"wrong\"")))
            .expect("typed replacement failure response"),
    );
    let ResponseResult::Error {
        diagnostic: Some(diagnostic),
        ..
    } = failed.result
    else {
        panic!("typed replacement failure must carry a diagnostic");
    };
    assert_eq!(
        diagnostic.code,
        crate::semantic::schema::DiagnosticCode::TypeMismatch
    );
    assert_eq!(
        std::fs::read(&root).expect("bytes after failure"),
        before_failure
    );
    let replaced = response(
        &crate::semantic::execute(&request(&root, &replace("i64", "5")))
            .expect("replacement preview"),
    );
    let transaction = match replaced.result {
        ResponseResult::ApplyTransaction { transaction } => transaction,
        other => panic!("expected replacement transaction, got {other:?}"),
    };
    assert_eq!(
        transaction.identities[0].relation,
        crate::semantic::schema::IdentityRelationKind::ReplacedExpression
    );
    assert_eq!(
        transaction.identities[0].old_node,
        transaction.identities[0].new_node
    );
}

#[test]
fn valid_transaction_crosses_the_removed_64_operation_admission_boundary() {
    let directory = case_dir("transaction-scale");
    let root = directory.join("main.lkjscript");
    let mut source = String::from("imports/\n");
    for file_index in 0..9 {
        source.push_str(&format!(
            "import/\nmodule/\nlib{file_index}.lkjscript\n/module\ndeclarations/\n"
        ));
        let mut library = String::new();
        let mut names = ((file_index * 8)..((file_index + 1) * 8).min(65))
            .map(|index| format!("f{index}"))
            .collect::<Vec<_>>();
        names.sort();
        for name in names {
            source.push_str(&format!("{name}\n"));
            library.push_str(&format!(
                "def/\nname/\n{name}\n/name\npublic\nfn/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nparams/\n/params\nunit\n/fn\n/def\n"
            ));
        }
        source.push_str("/declarations\n/import\n");
        std::fs::write(
            directory.join(format!("lib{file_index}.lkjscript")),
            library,
        )
        .expect("write scale library");
    }
    source.push_str(
        "/imports\nmain/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nunit\n/main\n",
    );
    std::fs::write(&root, &source).expect("write scale root");
    let (revision, snapshot) = take_snapshot(&root);
    let operations = (0..65)
        .map(|index| {
            let declaration = snapshot
                .declarations
                .iter()
                .find(|declaration| declaration.name == format!("f{index}"))
                .expect("generated declaration");
            format!(
                concat!(
                    "{{\"kind\":\"rename-declaration\",",
                    "\"declaration_key\":\"{}\",\"entity_fingerprint\":\"{}\",",
                    "\"new_name\":\"g{index}\"}}"
                ),
                declaration.key,
                declaration.fingerprint,
                index = index,
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let operation = format!(
        concat!(
            "{{\"kind\":\"apply-transaction\",\"mode\":\"preview\",",
            "\"base_revision\":\"{revision}\",\"file_preconditions\":[{}],",
            "\"operations\":[{operations}]}}"
        ),
        preconditions(&snapshot),
        revision = revision,
        operations = operations,
    );
    let preview = response(
        &crate::semantic::execute(&request(&root, &operation))
            .expect("valid transaction above the former operation boundary"),
    );
    assert!(matches!(
        preview.result,
        ResponseResult::ApplyTransaction { .. }
    ));
    assert_eq!(
        std::fs::read_to_string(&root).expect("preview source remains unchanged"),
        source
    );
}

#[test]
fn deep_expression_replacement_crosses_json_and_source_depth_boundaries() {
    let directory = case_dir("deep-expression-transaction");
    let root = directory.join("main.lkjscript");
    let original = "main/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nunit\n/main\n";
    std::fs::write(&root, original).expect("write deep transaction source");
    let (revision, snapshot) = take_snapshot(&root);
    let main = snapshot
        .declarations
        .iter()
        .find(|declaration| declaration.name == "$main")
        .expect("main declaration");
    let body = snapshot
        .nodes
        .iter()
        .rfind(|node| {
            node.kind == crate::semantic::schema::SemanticNodeKind::UnitLiteral
                && node.declaration.as_deref() == Some(&main.key)
        })
        .expect("main body");
    let depth = 192;
    let mut expression = "{\"kind\":\"unit\"}".to_string();
    for _ in 0..depth {
        expression = format!("{{\"kind\":\"do\",\"expressions\":[{expression}]}}");
    }
    let publish = format!(
        concat!(
            "{{\"kind\":\"apply-transaction\",\"mode\":\"publish\",",
            "\"base_revision\":\"{revision}\",\"file_preconditions\":[{}],",
            "\"operations\":[{{\"kind\":\"replace-expression\",",
            "\"declaration_key\":\"{}\",\"entity_fingerprint\":\"{}\",",
            "\"node\":{},\"node_fingerprint\":\"{}\",\"expression\":{expression}}}]}}"
        ),
        preconditions(&snapshot),
        main.key,
        main.fingerprint,
        body.index,
        body.fingerprint,
        revision = revision,
        expression = expression,
    );
    assert!(request(&root, &publish).len() < crate::semantic::MAX_REQUEST_BYTES);
    let published = response(
        &crate::semantic::execute(&request(&root, &publish)).expect("publish deep expression"),
    );
    assert!(matches!(
        published.result,
        ResponseResult::ApplyTransaction { .. }
    ));
    assert_eq!(
        std::fs::read_to_string(&root)
            .expect("read deep source")
            .matches("do/\n")
            .count(),
        depth
    );

    let (revision, snapshot) = take_snapshot(&root);
    let main = snapshot
        .declarations
        .iter()
        .find(|declaration| declaration.name == "$main")
        .expect("deep main declaration");
    let deepest = snapshot
        .nodes
        .iter()
        .rfind(|node| {
            node.kind == crate::semantic::schema::SemanticNodeKind::UnitLiteral
                && node.declaration.as_deref() == Some(&main.key)
        })
        .expect("deepest unit");
    let preview = format!(
        concat!(
            "{{\"kind\":\"apply-transaction\",\"mode\":\"preview\",",
            "\"base_revision\":\"{revision}\",\"file_preconditions\":[{}],",
            "\"operations\":[{{\"kind\":\"replace-expression\",",
            "\"declaration_key\":\"{}\",\"entity_fingerprint\":\"{}\",",
            "\"node\":{},\"node_fingerprint\":\"{}\",",
            "\"expression\":{{\"kind\":\"do\",\"expressions\":[{{\"kind\":\"unit\"}}]}}}}]}}"
        ),
        preconditions(&snapshot),
        main.key,
        main.fingerprint,
        deepest.index,
        deepest.fingerprint,
        revision = revision,
    );
    let previewed = response(
        &crate::semantic::execute(&request(&root, &preview)).expect("preview deepest replacement"),
    );
    assert!(
        matches!(previewed.result, ResponseResult::ApplyTransaction { .. }),
        "unexpected deep replacement response: {:?}",
        previewed.result
    );
}
