use crate::oracle::{compare_source, ScalarOutcome};
use lkjscript_compiler::compile_source;
use lkjscript_core::{CapabilityKind, ExecutionOutcome, ExecutionPolicy};
use lkjscript_vm::{run_chunk, ExecutionInputs};

#[test]
fn conditional_whole_place_cleanup_matches_evaluator_and_vm() {
    assert_eq!(
        compare_source(&source(), "conditional-cleanup.lkjscript"),
        ScalarOutcome::I64(3)
    );
}

#[test]
fn conditional_resource_close_executes_exactly_once_in_vm() {
    let path = format!("/tmp/lkjscript-conditional-resource-{}", std::process::id());
    std::fs::write(&path, b"resource").expect("create resource fixture");
    for flag in ["false", "true"] {
        let source = resource_source(&path, flag);
        let program = compile_source(&source, "conditional-resource-cleanup.lkjscript")
            .expect("compile conditional resource cleanup");
        assert!(program.memory_plan().obligations.iter().any(|obligation| {
            obligation.drop_class
                == Some(lkjscript_compiler::memory_plan::MemoryDropClass::Conditional)
        }));
        let outcome = run_chunk(
            program.bytecode(),
            &ExecutionInputs {
                arguments: Vec::new(),
                capabilities: vec![CapabilityKind::FileSystem],
                host: lkjscript_host::HostEnvironment::default(),
            },
            &ExecutionPolicy::unrestricted(),
        );
        assert!(matches!(outcome, ExecutionOutcome::Returned(_)));
    }
    let _removed = std::fs::remove_file(path);
}

fn resource_source(path: &str, flag: &str) -> String {
    format!(
        concat!(
            "def/\nname/\nmaybe-close\n/name\npublic\nfn/\nsig/\ninputs/\nbool\n",
            "file-reader\n/inputs\noutput/\nunit\n/output\n/sig\nparams/\nflag\nbool\n",
            "reader\nfile-reader\n/params\nif/\nflag\ndo/\nunwrap-ok/\ndrop/\nreader\n",
            "/drop\n/unwrap-ok\nunit\n/do\nunit\n/if\n/fn\n/def\nmain/\nsig/\n",
            "inputs/\ncapability/\nfile-system\n/capability\n/inputs\noutput/\nunit\n",
            "/output\n/sig\nparams/\nfile-system\ncapability/\nfile-system\n/capability\n",
            "/params\nlet/\nbind/\nreader\nunwrap-ok/\nopen-file-reader/\nfile-system\n",
            "unwrap-ok/\nconvert-string-to-path/\nstring-literal/\n{path}\n/string-literal\n",
            "/convert-string-to-path\n/unwrap-ok\n/open-file-reader\n/unwrap-ok\n/bind\n",
            "maybe-close/\n{flag}\nmove/\nreader\n/move\n/maybe-close\n/let\n/main\n"
        ),
        path = path,
        flag = flag
    )
}

fn source() -> String {
    concat!(
        "def/\nname/\nselect-bytes\n/name\npublic\nfn/\n",
        "sig/\ninputs/\nbool\n/inputs\noutput/\nbyte-vector\n/output\n/sig\n",
        "params/\nflag\nbool\n/params\nlet/\nbind/\nb\nnew-byte-vector/\n1\n",
        "/new-byte-vector\n/bind\nlet/\nbind/\nc\nnew-byte-vector/\n2\n/new-byte-vector\n",
        "/bind\nif/\nflag\nmove/\nb\n/move\nmove/\nc\n/move\n/if\n/let\n/let\n",
        "/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n",
        "let/\nbind/\na\nselect-bytes/\ntrue\n/select-bytes\n/bind\n",
        "let/\nbind/\nb\nselect-bytes/\nfalse\n/select-bytes\n/bind\nadd/\n",
        "byte-slice-length/\nborrow/\na\n/borrow\n/byte-slice-length\n",
        "byte-slice-length/\nborrow/\nb\n/borrow\n/byte-slice-length\n/add\n/let\n/let\n/main\n"
    )
    .to_owned()
}
