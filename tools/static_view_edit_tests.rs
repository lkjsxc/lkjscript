//! Apply the private static-label ownership edit, then observe both routes.
use super::{command, contains, diagnostic_edits, display_bytes, invoke, path, run_value, write, Config, Report};

fn applied(world: &mut World, executable: &File, directory: &mut Directory,
    config: &Config, report: &mut Report, name: &str, source: &str, expected: &str) -> Result<(), String> {
    write(directory, name, source.as_bytes())?;
    let rejected = invoke(world, executable, &command(config, "check", name), b"")?;
    report.require(rejected.status == 1 && contains(&rejected.errors, b"private static label")
        && contains(&rejected.errors, b"docs/language.md"),
        format!("static label construct/rule missing for {name}: {}", display_bytes(&rejected.errors)));
    let rewritten = diagnostic_edits::apply(source, &rejected.errors, &path(config, name))?;
    report.require(rewritten.contains("kind: std::String") && !rewritten.contains("kind: &'static str"),
        format!("static label ownership/type edit missing for {name}"));
    // Original eligibility intentionally excludes reads of the migrating field.
    // Add an independent post-edit assertion to observe its owned contents.
    let observed = rewritten.as_str().replace("assert!(value.text == \"hello\");",
        "assert!(value.kind.as_str() == \"text/plain\"); assert!(value.text == \"hello\");");
    report.require(observed != rewritten, format!("static label content oracle missing for {name}"));
    write(directory, name, observed.as_bytes())?;
    run_value(world, executable, config, report, name, "main", expected)?;
    Ok(())
}

pub fn run(world: &mut World, executable: &File, directory: &mut Directory,
    config: &Config, report: &mut Report) -> Result<(), String> {
    let original = include_str!("../tests/diagnostics/static_view_owned_aggregate.rs");
    applied(world, executable, directory, config, report, "static-private-label.rs", original, "")?;
    let explicit = original.replace("Reply { kind, text }", "Reply { kind: kind, text: text }");
    applied(world, executable, directory, config, report, "static-explicit-label.rs", &explicit, "")?;
    let utf8 = format!("// π before every edit\n{original}").as_str().replace("\n", "\r\n");
    applied(world, executable, directory, config, report, "static-utf8-label.rs", &utf8, "")?;
    let literal = r#"struct Reply { kind: &'static str, text: String }
fn reply(text: String) -> Reply { Reply { kind: "text/plain", text } }
fn main() { let value = reply("hello".to_string()); assert!(value.text == "hello"); }
"#;
    applied(world, executable, directory, config, report, "static-literal-label.rs", literal, "")?;
    let order = r#"struct Reply { kind: &'static str, text: String }
fn reply(kind: &'static str, text: String) -> Reply { Reply { kind, text } }
struct Guard { output: Stdout }
impl Drop for Guard { fn drop(&mut self) { self.output.write(b"D"); } }
fn text(output: &mut Stdout) -> String { output.write(b"A"); "hello".to_string() }
pub fn main(mut world: World) {
    let guard = Guard { output: world.stdout() };
    let mut output = world.stdout();
    let value = reply("text/plain", text(&mut output));
    output.write(b"B");
    assert!(value.text == "hello");
    drop(value);
    output.write(b"C");
    drop(guard);
    output.write(b"\n");
}
"#;
    applied(world, executable, directory, config, report, "static-label-order.rs", order, "ABCD\n")?;

    let negatives: Vec<(String, String)> = vec![
        ("static-public-type.rs".to_string(), original.replace("struct Reply", "pub struct Reply")),
        ("static-public-constructor.rs".to_string(), original.replace("fn reply(", "pub fn reply(")),
        ("static-public-api.rs".to_string(), format!("{original}\npub fn exposed() -> Reply {{ reply(\"text/plain\", \"hello\".to_string()) }}\n")),
        ("static-used-field.rs".to_string(), original.replace("assert!(value.text", "assert!(value.kind == \"text/plain\"); assert!(value.text")),
        ("static-pattern-field.rs".to_string(), r#"struct Reply { kind: &'static str, text: String }
fn reply(kind: &'static str, text: String) -> Reply { Reply { kind, text } }
fn main() { let Reply { kind: _, text } = reply("text/plain", "hello".to_string()); assert!(text == "hello"); }
"#.to_string()),
        ("static-computed-label.rs".to_string(), format!("{}\nfn label() -> &'static str {{ \"text/plain\" }}\n", original.replace("reply(\"text/plain\",", "reply(label(),"))),
        ("static-other-constructor.rs".to_string(), format!("{original}\nfn another(kind: &'static str, text: String) -> Reply {{ Reply {{ kind, text }} }}\n")),
        ("static-reordered-fields.rs".to_string(), original.replace("Reply { kind, text }", "Reply { text, kind }")),
        ("static-mutable-input.rs".to_string(), original.replace("fn reply(kind:", "fn reply(mut kind:")),
        ("static-computed-field.rs".to_string(), original.replace("Reply { kind, text }", "Reply { kind, text: kind.to_string() }")),
        ("static-function-value.rs".to_string(), original.replace("fn main() {", "fn main() { let function = reply;")),
        ("static-custom-method.rs".to_string(), format!("{original}\ntrait OtherText {{ fn to_string(&self) -> String; }}\nimpl OtherText for str {{ fn to_string(&self) -> String {{ String::new() }} }}\n")),
        ("static-string-drop.rs".to_string(), format!("{original}\nimpl Drop for std::String {{ fn drop(&mut self) {{ assert!(self.as_str() == \"hello\"); }} }}\n")),
        ("static-string-alias-drop.rs".to_string(), format!("{original}\ntype OwnedText = std::String;\nimpl Drop for OwnedText {{ fn drop(&mut self) {{ assert!(self.as_str() == \"hello\"); }} }}\n")),
        ("static-shadowed-string.rs".to_string(), format!("{original}\nstruct String;\n")),
        ("static-shadowed-std.rs".to_string(), format!("{original}\nmod std {{ pub struct String; }}\n")),
    ];
    for (name, source) in negatives {
        write(directory, &name, source.as_bytes())?;
        let rejected = invoke(world, executable, &command(config, "check", &name), b"")?;
        report.require(rejected.status == 1 && !contains(&rejected.errors, b"LKJSCRIPT_EDITS "),
            format!("unsafe static label rewrite for {name}: {}", display_bytes(&rejected.errors)));
    }
    Ok(())
}
