use super::*;

const STDIO_MAIN: &str = concat!(
    "main/\nsig/\nCapability/\nStdio\n/Capability\n->\nUnit\n/sig\n",
    "params/\nstdio\nCapability/\nStdio\n/Capability\n/params\n",
    "print/\nstdio\nstr/\nhello\n/str\n/print\n/main\n"
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
    let ambient = unit_main("print/\nstr/\nhello\n/str\n/print");
    let error = compile_source(&ambient, "ambient.lkjscript", &Limits::default())
        .expect_err("ambient print")
        .to_string();
    assert!(error.contains("expected 2 args, got 1"));

    let wrong = concat!(
        "main/\nsig/\nCapability/\nArguments\n/Capability\n->\nUnit\n/sig\n",
        "params/\narguments\nCapability/\nArguments\n/Capability\n/params\n",
        "print/\narguments\nstr/\nhello\n/str\n/print\n/main\n"
    );
    let error = compile_source(wrong, "wrong-capability.lkjscript", &Limits::default())
        .expect_err("wrong capability")
        .to_string();
    assert!(error.contains("not assignable"), "{error}");
}

#[test]
fn duplicate_unsorted_and_forged_capabilities_are_rejected() {
    let duplicate = STDIO_MAIN.replace(
        "Capability/\nStdio\n/Capability\n->",
        "Capability/\nStdio\n/Capability\nCapability/\nStdio\n/Capability\n->",
    );
    assert!(compile_source(&duplicate, "duplicate.lkjscript", &Limits::default()).is_err());

    let unsorted = STDIO_MAIN
        .replace(
            "Capability/\nStdio\n/Capability\n->",
            "Capability/\nStdio\n/Capability\nCapability/\nArguments\n/Capability\n->",
        )
        .replace(
            "params/\nstdio\nCapability/\nStdio\n/Capability",
            concat!(
                "params/\nstdio\nCapability/\nStdio\n/Capability\n",
                "arguments\nCapability/\nArguments\n/Capability"
            ),
        );
    let error = compile_source(&unsorted, "unsorted.lkjscript", &Limits::default())
        .expect_err("unsorted capabilities")
        .to_string();
    assert!(error.contains("sorted and unique"));

    let forged = STDIO_MAIN.replace("print/\nstdio", "print/\n7");
    assert!(compile_source(&forged, "forged.lkjscript", &Limits::default()).is_err());
}
