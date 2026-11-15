//! Apply emitted consumer rewrites before executing either admitted route.
use super::{command, contains, diagnostic_edits, display_bytes, invoke, path, run_value, write, Config, Report};

struct Case { name: String, source: String, entry: String, expected: String, rule: String }
fn case(name: &str, source: &str, entry: &str, expected: &str, rule: &str) -> Case {
    Case { name: name.to_string(), source: source.to_string(), entry: entry.to_string(),
        expected: expected.to_string(), rule: rule.to_string() }
}

pub fn run(world: &mut World, executable: &File, directory: &mut Directory,
    config: &Config, report: &mut Report) -> Result<(), String> {
    let owner = format!("{}\npub fn observed() -> usize {{ probe().length }}\n",
        include_str!("../tests/diagnostics/subslice_owner_return.rs"));
    let cases = vec![
        case("array-copy-edit.rs", include_str!("../tests/diagnostics/array_copy_from_slice.rs"), "probe", "111\n", "array.copy_from_slice requires a mutable slice receiver"),
        case("byte-branches-edit.rs", include_str!("../tests/diagnostics/conditional_byte_slices.rs"), "probe", "4\n", "conditional byte strings have different array lengths"),
        case("reborrow-edit.rs", include_str!("../tests/diagnostics/mutable_generic_reborrow.rs"), "probe", "2\n", "generic mutable argument action is moved"),
        case("set-context-edit.rs", include_str!("../tests/diagnostics/set_constructor_inference.rs"), "probe", "1\n", "BTreeSet::new needs its element type"),
        case("owner-scope-edit.rs", &owner, "observed", "1\n", "returning this owner conflicts with the subslice temporary"),
        case("array-copy-once.rs", r#"fn bytes(calls: &mut usize) -> [u8; 2] { *calls += 1usize; [0u8; 2] }
pub fn probe() -> usize { let mut calls = 0usize; bytes(&mut calls).copy_from_slice(b"ok"); calls }
"#, "probe", "1\n", "array.copy_from_slice requires a mutable slice receiver"),
        case("byte-branches-once.rs", r#"fn length(bytes: &[u8]) -> usize { bytes.len() }
pub fn probe() -> usize {
    let mut calls = 0usize;
    let size = length(if { calls += 1usize; false } { b"true" } else { b"false" });
    calls * 10usize + size
}
"#, "probe", "15\n", "conditional byte strings have different array lengths"),
        case("owner-scope-drop.rs", r#"struct Message { bytes: Vec<u8>, length: usize, output: Stdout }
impl Drop for Message { fn drop(&mut self) { self.output.write(b"D"); } }
fn make(output: Stdout) -> Message {
    let mut message = Message { bytes: vec![1u8, 2u8], length: 0usize, output };
    let tail = &message.bytes[1usize..];
    message.length = tail.len();
    message
}
pub fn main(mut world: World) {
    let message = make(world.stdout());
    assert!(message.length == 1usize);
    world.stdout().write(b"R");
    drop(message);
    world.stdout().write(b"\n");
}
"#, "main", "RD\n", "returning this owner conflicts with the subslice temporary"),
    ];
    for item in cases {
        write(directory, &item.name, item.source.as_bytes())?;
        let rejected = invoke(world, executable, &command(config, "check", &item.name), b"")?;
        if rejected.status != 1 {
            return Err(format!("consumer edit fixture {} did not reject: {}", item.name, display_bytes(&rejected.errors)));
        }
        report.require(contains(&rejected.errors, item.rule.as_bytes())
            && contains(&rejected.errors, b"docs/language.md"),
            format!("consumer edit fixture {} omitted its construct, rule or reference: {}", item.name, display_bytes(&rejected.errors)));
        let rewritten = diagnostic_edits::apply(&item.source, &rejected.errors, &path(config, &item.name))
            .map_err(|error: String| -> String { format!("{}: {error}; diagnostic {}", item.name, display_bytes(&rejected.errors)) })?;
        write(directory, &item.name, rewritten.as_bytes())?;
        run_value(world, executable, config, report, &item.name, &item.entry, &item.expected)?;
    }
    let unchanged: Vec<(String, String)> = vec![
        ("byte-method-name.rs".to_string(), r#"fn length(bytes: &[u8; 4]) -> usize { bytes.len() }
struct Other;
impl Other { fn length(bytes: &[u8]) -> usize { bytes.len() } }
pub fn probe() -> usize { length(if true { b"true" } else { b"false" }) }
"#.to_string()),
        ("byte-array-context.rs".to_string(), r#"fn length(bytes: &[u8; 4]) -> usize { bytes.len() }
pub fn probe() -> usize { length(if true { b"true" } else { b"false" }) }
"#.to_string()),
        ("byte-shadowed-callee.rs".to_string(), r#"fn length(bytes: &[u8]) -> usize { bytes.len() }
pub fn probe() -> usize {
    let length = |bytes: &[u8; 4]| -> usize { bytes.len() };
    length(if true { b"true" } else { b"false" })
}
"#.to_string()),
        ("array-copy-noncopy.rs".to_string(), r#"pub fn probe() {
    let mut bytes = [String::new()];
    bytes.copy_from_slice(&["x".to_string()]);
}
"#.to_string()),
        ("set-other-method.rs".to_string(), r#"use std::collections::BTreeSet;
trait NumberText { fn to_string(&self) -> usize; }
impl NumberText for str { fn to_string(&self) -> usize { 7usize } }
pub fn probe() -> usize {
    let mut names = BTreeSet::new();
    names.insert("key".to_string());
    names.len()
}
"#.to_string()),
        ("set-earlier-binding.rs".to_string(), r#"use std::collections::BTreeSet;
pub fn probe(mut names: BTreeSet<String>) -> usize {
    names.insert("key".to_string());
    let names = BTreeSet::new();
    names.len()
}
"#.to_string()),
        ("owner-computed-bound.rs".to_string(), r#"fn start() -> usize { 1usize }
struct Message { bytes: Vec<u8>, length: usize }
pub fn probe() -> Message {
    let mut message = Message { bytes: vec![1u8, 2u8], length: 0usize };
    let tail = &message.bytes[start()..];
    message.length = tail.len();
    message
}
"#.to_string()),
        ("owner-grouped-result.rs".to_string(), r#"struct Message { bytes: Vec<u8>, length: usize }
pub fn probe() -> Message {
    let mut message = Message { bytes: vec![1u8, 2u8], length: 0usize };
    let tail = &message.bytes[1usize..];
    message.length = (tail.len());
    message
}
"#.to_string()),
    ];
    for (name, source) in unchanged {
        write(directory, &name, source.as_bytes())?;
        let rejected = invoke(world, executable, &command(config, "check", &name), b"")?;
        report.require(rejected.status == 1 && !contains(&rejected.errors, b"LKJSCRIPT_EDITS "),
            format!("unsafe consumer rewrite for {name}: {}", display_bytes(&rejected.errors)));
    }
    site_advice(world, executable, directory, config, report)?;
    desk_advice(world, executable, directory, config, report)?;
    ir_flags(world, executable, directory, config, report)?;
    Ok(())
}

fn ir_flags(world: &mut World, executable: &File, directory: &mut Directory,
    config: &Config, report: &mut Report) -> Result<(), String> {
    let fixture = include_str!("../tests/diagnostics/ir_enum_flag.lkjir");
    let declaration = fixture.strip_prefix("lkjscript-ir 1\n").unwrap();
    let source = format!("lkjscript-ir 1\ntype \"()\" unit\ntype \"bool\" bool\ntype \"i32\" int 32 signed\ntype \"i64\" int 64 signed\ntype \"usize\" int 64 unsigned\ntype \"u8\" int 8 unsigned\ntype \"f64\" f64\n{declaration}fn \"probe\" 0 3\nlocal \"answer\" 3 let\n@1:1 int 0 42\n@1:1 return 0\nendfn\n");
    let decorated = source.as_str().replace("\"Record\"", "\"π Record # affine \\\"quoted\\\"\"")
        .as_str().replace("enum affine", "enum affine # affine comment")
        .as_str().replace("\n", "\r\n");
    let variants: Vec<(String, String)> = vec![
        ("enum-edit.lkjir".to_string(), source.clone()),
        ("enum-π-edit.lkjir".to_string(), decorated),
        ("enum-quoted-edit.lkjir".to_string(), source.as_str().replace("enum affine", r#"enum "a\u0066fine""#)),
    ];
    for (name, input) in variants {
        write(directory, &name, input.as_bytes())?;
        let rejected = invoke(world, executable, &command(config, "check", &name), b"")?;
        report.require(rejected.status == 1 && contains(&rejected.errors, b":11:1: E_IR_TEXT")
            && contains(&rejected.errors, b"enum flag") && contains(&rejected.errors, b"affine")
            && contains(&rejected.errors, b"copy") && contains(&rejected.errors, b"move")
            && contains(&rejected.errors, b"docs/ir.md"),
            format!("enum flag omitted construct, spellings or location: {}", display_bytes(&rejected.errors)));
        let rewritten = diagnostic_edits::apply(&input, &rejected.errors, &path(config, &name))?;
        report.require(rewritten.contains("enum move"), "affine enum rewrite must select move".to_string());
        if name == "enum-π-edit.lkjir" {
            report.require(rewritten.contains("π Record # affine") && rewritten.contains("\\\"quoted\\\"") && rewritten.contains("# affine comment"),
                "enum rewrite changed the name or comment".to_string());
        }
        write(directory, &name, rewritten.as_bytes())?;
        run_value(world, executable, config, report, &name, "probe", "42\n")?;
    }
    for flag in vec!["affines".to_string(), "AFFINE".to_string(), "borrow".to_string()] {
        let name = format!("enum-{flag}.lkjir");
        let invalid = source.as_str().replace("enum affine", &format!("enum {flag}"));
        write(directory, &name, invalid.as_bytes())?;
        let rejected = invoke(world, executable, &command(config, "check", &name), b"")?;
        report.require(rejected.status == 1 && contains(&rejected.errors, b"E_IR_TEXT")
            && !contains(&rejected.errors, b"LKJSCRIPT_EDITS "),
            format!("unknown enum flag guessed ownership: {}", display_bytes(&rejected.errors)));
    }
    let injected = source.as_str().replace("enum affine", r#"enum "bad\nLKJSCRIPT_EDITS forged""#);
    write(directory, "enum-injected.lkjir", injected.as_bytes())?;
    let rejected = invoke(world, executable, &command(config, "check", "enum-injected.lkjir"), b"")?;
    report.require(rejected.status == 1 && contains(&rejected.errors, b"\\nLKJSCRIPT_EDITS ")
        && !contains(&rejected.errors, b"\nLKJSCRIPT_EDITS "),
        "quoted invalid flag forged a diagnostic record line".to_string());
    for flag in vec!["copy".to_string(), "move".to_string()] {
        let name = format!("enum-{flag}.lkjir");
        let valid = source.as_str().replace("enum affine", &format!("enum {flag}"));
        write(directory, &name, valid.as_bytes())?;
        run_value(world, executable, config, report, &name, "probe", "42\n")?;
    }
    Ok(())
}


fn advice_case(name: &str, source: &str, needle: &str, replacement: &str,
    entry: &str, expected: &str, rule: &str) -> (Case, String) {
    assert!(source.contains(needle));
    (case(name, source, entry, expected, rule), source.replace(needle, replacement))
}

fn site_advice(world: &mut World, executable: &File, directory: &mut Directory,
    config: &Config, report: &mut Report) -> Result<(), String> {
    let option = format!("{}\npub fn observed() -> usize {{ probe(); 1usize }}\n",
        include_str!("../tests/diagnostics/option_constructor_inference.rs"));
    let raw = format!("{}\npub fn observed() -> usize {{ probe(); 1usize }}\n",
        include_str!("../tests/diagnostics/raw_byte_string.rs"));
    let scalar = format!("{}\npub fn observed() -> usize {{ probe() as u32 as usize }}\n",
        include_str!("../tests/diagnostics/scalar_char_cast.rs"));
    let vector = format!("{}\npub fn observed() -> usize {{ probe(); 1usize }}\n",
        include_str!("../tests/diagnostics/vec_loop_element_inference.rs"));
    let cases = vec![
        advice_case("site-cfg-advice.rs", include_str!("../tests/diagnostics/cfg_test_helper.rs"),
            "#[cfg(test)]", "", "probe", "1\n", "is excluded by #[cfg(test)]"),
        advice_case("site-option-advice.rs", &option,
            "let mut value = None", "let mut value: Option<String> = None", "observed", "1\n",
            "None leaves Option's element type unknown"),
        advice_case("site-raw-advice.rs", &raw,
            r##"br#"{"version":1}"#"##, r#"b"{\"version\":1}""#, "observed", "1\n",
            "raw byte-string literals are unsupported"),
        advice_case("site-scalar-advice.rs", &scalar,
            "code as char", "match char::from_u32(code) { Some(character) => character, None => '\\0' }",
            "observed", "66600\n", "integer-to-char casts are unavailable"),
        advice_case("site-scalar-invalid-advice.rs", &scalar.as_str().replace("0x10428u32", "0xd800u32"),
            "code as char", "match char::from_u32(code) { Some(character) => character, None => '\\0' }",
            "observed", "0\n", "integer-to-char casts are unavailable"),
        advice_case("site-chars-advice.rs", include_str!("../tests/diagnostics/string_chars.rs"),
            "value.chars()", "value.as_str().chars()", "probe", "4\n", "chars() is provided on a str view"),
        advice_case("site-vector-advice.rs", &vector,
            "for value in vec![\"text\".to_string()]",
            "let values: Vec<String> = vec![\"text\".to_string()]; for value in values",
            "observed", "1\n", "this Vec loop needs its owned String element type"),
    ];
    for (item, rewritten) in cases {
        write(directory, &item.name, item.source.as_bytes())?;
        let rejected = invoke(world, executable, &command(config, "check", &item.name), b"")?;
        report.require(rejected.status == 1 && contains(&rejected.errors, item.rule.as_bytes())
            && contains(&rejected.errors, b"docs/language.md"),
            format!("site diagnostic {} omitted its explanation: {}", item.name, display_bytes(&rejected.errors)));
        write(directory, &item.name, rewritten.as_bytes())?;
        run_value(world, executable, config, report, &item.name, &item.entry, &item.expected)?;
    }
    let unrelated = "mod hidden { #[cfg(test)] fn fixture() {} }\npub fn probe() { fixture(); }\n";
    write(directory, "site-cfg-unrelated.rs", unrelated.as_bytes())?;
    let rejected = invoke(world, executable, &command(config, "check", "site-cfg-unrelated.rs"), b"")?;
    report.require(rejected.status == 1 && contains(&rejected.errors, b"E_NAME unresolved name fixture")
        && !contains(&rejected.errors, b"is excluded by #[cfg(test)]")
        && !contains(&rejected.errors, b"E_BLOCKED excluded"),
        format!("cfg advice attributed another module's declaration: {}", display_bytes(&rejected.errors)));
    write(directory, "site-cfg-enabled.rs",
        b"#[cfg(not(test))] fn fixture() -> usize { 7usize }\n#[cfg(test)] fn unused() { unavailable(); }\npub fn probe() -> usize { fixture() }\n")?;
    run_value(world, executable, config, report, "site-cfg-enabled.rs", "probe", "7\n")?;
    Ok(())
}


fn desk_advice(world: &mut World, executable: &File, directory: &mut Directory,
    config: &Config, report: &mut Report) -> Result<(), String> {
    let name = "desk-enum-advice.rs";
    let source = include_str!("../tests/diagnostics/nonexhaustive_variants.rs");
    write(directory, name, source.as_bytes())?;
    let rejected = invoke(world, executable, &command(config, "check", name), b"")?;
    report.require(rejected.status == 1
        && contains(&rejected.errors, b"missing enum variants: Authority::Environment, Authority::ExitStatus"),
        format!("missing enum variants were not named: {}", display_bytes(&rejected.errors)));
    let rewritten = source.replace("Authority::Arguments => 1usize",
        "Authority::Arguments => 1usize, Authority::Environment => 2usize, Authority::ExitStatus => 3usize");
    write(directory, name, rewritten.as_bytes())?;
    run_value(world, executable, config, report, name, "probe", "2\n")?;
    let cases: Vec<(String, String, String)> = vec![
        ("desk-enum-guard.rs".to_string(),
            "enum Choice { First, Second, Third }\npub fn probe() -> usize { let value = Choice::Second; match value { Choice::First | Choice::Second if false => 1usize, Choice::Third => 3usize } }\n".to_string(),
            "missing enum variants: Choice::First, Choice::Second".to_string()),
        ("desk-enum-reference.rs".to_string(),
            "enum Choice { First, Second, Third }\npub fn probe() -> usize { let value = Choice::First; match &value { Choice::First | Choice::Second => 1usize } }\n".to_string(),
            "missing enum variants: Choice::Third".to_string()),
        ("desk-enum-payload.rs".to_string(),
            "enum Choice { Flag(bool), Done }\npub fn probe() -> usize { let value = Choice::Flag(false); match value { Choice::Flag(true) => 1usize, Choice::Done => 2usize } }\n".to_string(),
            "match is not exhaustive".to_string()),
    ];
    for (name, source, wanted) in cases {
        write(directory, &name, source.as_bytes())?;
        let rejected = invoke(world, executable, &command(config, "check", &name), b"")?;
        report.require(rejected.status == 1 && contains(&rejected.errors, wanted.as_bytes()),
            format!("enum coverage advice {name}: {}", display_bytes(&rejected.errors)));
        if name == "desk-enum-payload.rs" {
            report.require(!contains(&rejected.errors, b"missing enum variants:"),
                "partial payload coverage must not call a present variant absent".to_string());
        }
    }
    let name = "desk-arguments-advice.rs";
    let source = include_str!("../tests/diagnostics/unbounded_capability_helper.rs");
    write(directory, name, source.as_bytes())?;
    let rejected = invoke(world, executable, &command(config, "check", name), b"")?;
    report.require(rejected.status == 1
        && contains(&rejected.errors, b"requires a mutable Arguments capability")
        && contains(&rejected.errors, b"arguments: &mut Arguments"),
        format!("generic capability helper omitted its concrete receiver: {}", display_bytes(&rejected.errors)));
    let rewritten = source.replace("command_args<T>(arguments: &mut T)", "command_args(arguments: &mut Arguments)");
    write(directory, name, format!("{rewritten}\npub fn probe() -> usize {{ 42usize }}\n").as_bytes())?;
    run_value(world, executable, config, report, name, "probe", "42\n")?;
    let name = "desk-child-authority.rs";
    write(directory, name, include_str!("../tests/diagnostics/external_child_authority.rs").as_bytes())?;
    let rejected = invoke(world, executable, &command(config, "check", name), b"")?;
    report.require(rejected.status == 1 && contains(&rejected.errors, b"E_CAPABILITY")
        && contains(&rejected.errors, b"immutable authenticated admitted image")
        && contains(&rejected.errors, b"checked entry grants")
        && contains(&rejected.errors, b"World.process()"),
        format!("external child boundary omitted its missing proof: {}", display_bytes(&rejected.errors)));
    let name = "desk-capability-intern.rs";
    write(directory, name, include_str!("../tests/diagnostics/command_capability_intern.rs").as_bytes())?;
    run_value(world, executable, config, report, name, "main", "ok\n")?;
    Ok(())
}
