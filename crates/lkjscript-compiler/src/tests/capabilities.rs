use super::*;

const STDIO_MAIN: &str = concat!(
    "main/\nsig/\ninputs/\ncapability/\nstdio\n/capability\n/inputs\noutput/\nunit\n/output\n/sig\n",
    "params/\nstdio\ncapability/\nstdio\n/capability\n/params\n",
    "print/\nstdio\nstring-literal/\nhello\n/string-literal\n/print\n/main\n"
);

#[test]
fn exact_main_capability_reaches_ssa_and_bytecode() {
    let program = compile_source(STDIO_MAIN, "capability.lkjscript", &Limits::default())
        .expect("compile explicit capability main");
    assert_eq!(
        program.bytecode().required_capabilities(),
        [lkjscript_core::CapabilityKind::Stdio]
    );
    assert_eq!(program.bytecode().main().arity, 1);
    let main = &program.ssa().program().functions[program.ssa().program().main.index().unwrap()];
    assert_eq!(
        main.signature.parameters,
        [lkjscript_ir::SsaType::Capability(
            lkjscript_core::CapabilityKind::Stdio
        )]
    );
}

#[test]
fn ambient_and_wrong_capability_calls_are_rejected() {
    let ambient = unit_main("print/\nstring-literal/\nhello\n/string-literal\n/print");
    let error = compile_source(&ambient, "ambient.lkjscript", &Limits::default())
        .expect_err("ambient print")
        .to_string();
    assert!(error.contains("expected 2 args, got 1"));

    let wrong = concat!(
        "main/\nsig/\ninputs/\ncapability/\narguments\n/capability\n/inputs\noutput/\nunit\n/output\n/sig\n",
        "params/\narguments\ncapability/\narguments\n/capability\n/params\n",
        "print/\narguments\nstring-literal/\nhello\n/string-literal\n/print\n/main\n"
    );
    let error = compile_source(wrong, "wrong-capability.lkjscript", &Limits::default())
        .expect_err("wrong capability")
        .to_string();
    assert!(error.contains("not assignable"), "{error}");
}

#[test]
fn duplicate_unsorted_and_forged_capabilities_are_rejected() {
    let duplicate = STDIO_MAIN.replace(
        "inputs/\ncapability/\nstdio\n/capability\n/inputs\noutput/\nunit\n/output",
        "inputs/\ncapability/\nstdio\n/capability\ncapability/\nstdio\n/capability\n/inputs\noutput/\nunit\n/output",
    );
    assert!(compile_source(&duplicate, "duplicate.lkjscript", &Limits::default()).is_err());

    let unsorted = STDIO_MAIN
        .replace(
            "inputs/\ncapability/\nstdio\n/capability\n/inputs\noutput/\nunit\n/output",
            "inputs/\ncapability/\nstdio\n/capability\ncapability/\narguments\n/capability\n/inputs\noutput/\nunit\n/output",
        )
        .replace(
            "params/\nstdio\ncapability/\nstdio\n/capability",
            concat!(
                "params/\nstdio\ncapability/\nstdio\n/capability\n",
                "arguments\ncapability/\narguments\n/capability"
            ),
        );
    let error = compile_source(&unsorted, "unsorted.lkjscript", &Limits::default())
        .expect_err("unsorted capabilities")
        .to_string();
    assert!(error.contains("sorted and unique"));

    let forged = STDIO_MAIN.replace("print/\nstdio", "print/\n7");
    assert!(compile_source(&forged, "forged.lkjscript", &Limits::default()).is_err());
}
