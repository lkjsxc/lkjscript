use lkjscript_compiler::compile_source;
use lkjscript_core::{ExecutionConfig, Limits};
use lkjscript_jit::{execute_forced, JitConfig};

pub fn compile(source: &str, name: &str) -> lkjscript_compiler::ExecutableProgram {
    let marked;
    let source = if source.starts_with("edition/\n2\n/edition\n") {
        source
    } else {
        marked = format!("edition/\n2\n/edition\n{source}");
        &marked
    };
    compile_source(source, name, &Limits::default()).expect("compile JIT source fixture")
}

pub fn forced(source: &str, name: &str) -> lkjscript_jit::JitExecution {
    let program = compile(source, name);
    execute_forced(
        program.ssa(),
        &ExecutionConfig::default(),
        JitConfig::default(),
    )
    .expect("forced native execution")
}

pub fn f64_loop() -> String {
    [
        "def/",
        "name/",
        "step",
        "/name",
        "fn/",
        "sig/",
        "F64",
        "I64",
        "->",
        "F64",
        "/sig",
        "params/",
        "acc",
        "F64",
        "i",
        "I64",
        "/params",
        "+/",
        "acc",
        "div/",
        "1.0",
        "+/",
        "*/",
        "2.0",
        "f64-from-i64-rounded/",
        "i",
        "/f64-from-i64-rounded",
        "/*",
        "1.0",
        "/+",
        "/div",
        "/+",
        "/fn",
        "/def",
        "main/",
        "sig/",
        "->",
        "F64",
        "/sig",
        "var/",
        "name/",
        "i",
        "/name",
        "type/",
        "I64",
        "/type",
        "0",
        "var/",
        "name/",
        "acc",
        "/name",
        "type/",
        "F64",
        "/type",
        "0.0",
        "do/",
        "while/",
        "lt/",
        "i",
        "1000",
        "/lt",
        "do/",
        "set/",
        "acc",
        "step/",
        "acc",
        "i",
        "/step",
        "/set",
        "set/",
        "i",
        "+/",
        "i",
        "1",
        "/+",
        "/set",
        "/do",
        "/while",
        "acc",
        "/do",
        "/var",
        "/var",
        "/main",
        "",
    ]
    .join("\n")
}
