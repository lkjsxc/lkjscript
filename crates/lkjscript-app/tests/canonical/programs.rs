use lkjscript_compiler::compile_source;
use lkjscript_core::ExecutionConfig;
use lkjscript_jit::{execute_forced, JitConfig};

pub fn compile(source: &str, name: &str) -> lkjscript_compiler::ExecutableProgram {
    let marked;
    let source = if source.starts_with("") {
        source
    } else {
        marked = source.to_string();
        &marked
    };
    compile_source(source, name).expect("compile JIT source fixture")
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
        "inputs/",
        "f64",
        "i64",
        "/inputs",
        "output/",
        "f64",
        "/output",
        "/sig",
        "params/",
        "acc",
        "f64",
        "i",
        "i64",
        "/params",
        "add/",
        "acc",
        "divide/",
        "1.0",
        "add/",
        "multiply/",
        "2.0",
        "convert-i64-to-f64-rounded/",
        "i",
        "/convert-i64-to-f64-rounded",
        "/multiply",
        "1.0",
        "/add",
        "/divide",
        "/add",
        "/fn",
        "/def",
        "main/",
        "sig/",
        "inputs/",
        "/inputs",
        "output/",
        "f64",
        "/output",
        "/sig",
        "var/",
        "name/",
        "i",
        "/name",
        "type/",
        "i64",
        "/type",
        "0",
        "var/",
        "name/",
        "acc",
        "/name",
        "type/",
        "f64",
        "/type",
        "0.0",
        "do/",
        "while/",
        "less-than/",
        "i",
        "1000",
        "/less-than",
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
        "add/",
        "i",
        "1",
        "/add",
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
