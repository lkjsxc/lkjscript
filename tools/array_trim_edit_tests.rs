//! Apply array-view and proved-ASCII trim edits before either execution route.
use super::{command, contains, diagnostic_edits, display_bytes, invoke, path, run_value, write, Config, Report};

struct Case { name: String, source: String, entry: String, expected: String, rule: String }
fn case(name: &str, source: &str, entry: &str, expected: &str, rule: &str) -> Case {
    Case { name: name.to_string(), source: source.to_string(), entry: entry.to_string(),
        expected: expected.to_string(), rule: rule.to_string() }
}

pub fn run(world: &mut World, executable: &File, directory: &mut Directory,
    config: &Config, report: &mut Report) -> Result<(), String> {
    let array = include_str!("../tests/diagnostics/borrowed_array_iteration.rs");
    let trim = include_str!("../tests/diagnostics/str_trim.rs");
    let cases = vec![
        case("array-shared-edit.rs", array, "probe", "3\n", "borrowed-array iteration has no applicable IntoIterator implementation"),
        case("array-once-edit.rs", r#"fn values(calls: &mut usize) -> [u8; 3] { *calls += 1usize; [1u8, 2u8, 3u8] }
pub fn probe() -> usize {
    let mut calls = 0usize;
    let mut total = 0usize;
    for value in &values(&mut calls) {
        if *value == 1u8 { continue; }
        if *value == 3u8 { break; }
        total += *value as usize;
    }
    calls * 10usize + total
}
"#, "probe", "12\n", "borrowed-array iteration has no applicable IntoIterator implementation"),
        case("array-grouped-edit.rs", r#"pub fn probe() -> usize {
    let values = [1usize, 2usize];
    let mut total = 0usize;
    for value in &(values) { total += *value; }
    total
}
"#, "probe", "3\n", "borrowed-array iteration has no applicable IntoIterator implementation"),
        case("array-empty-edit.rs", r#"pub fn probe() -> usize {
    let values: [u8; 0] = [];
    let mut total = 0usize;
    for value in &values { total += *value as usize; }
    total
}
"#, "probe", "0\n", "borrowed-array iteration has no applicable IntoIterator implementation"),
        case("array-drop-edit.rs", r#"struct Item { output: Stdout, tag: u8 }
impl Drop for Item { fn drop(&mut self) { self.output.write(&[self.tag]); } }
pub fn main(mut world: World) {
    let values = [
        Item { output: world.stdout(), tag: b'1' },
        Item { output: world.stdout(), tag: b'2' }
    ];
    for value in &values { world.stdout().write(&[value.tag]); }
    world.stdout().write(b"R");
    drop(values);
    world.stdout().write(b"\n");
}
"#, "main", "12R12\n", "borrowed-array iteration has no applicable IntoIterator implementation"),
        case("trim-literal-edit.rs", trim, "probe", "4\n", "str.trim is unavailable"),
        case("trim-ascii-whitespace.rs", r#"pub fn probe() -> usize { "\t\r\n\u{b}\u{c} x \t\r\n".trim().len() }
"#, "probe", "1\n", "str.trim is unavailable"),
        case("trim-empty-edit.rs", r#"pub fn probe() -> usize { "".trim().len() }
"#, "probe", "0\n", "str.trim is unavailable"),
        case("trim-null-edit.rs", r#"pub fn probe() -> usize { " \0x\0 ".trim().len() }
"#, "probe", "3\n", "str.trim is unavailable"),
    ];
    for item in cases {
        write(directory, &item.name, item.source.as_bytes())?;
        let rejected = invoke(world, executable, &command(config, "check", &item.name), b"")?;
        report.require(rejected.status == 1 && contains(&rejected.errors, item.rule.as_bytes())
            && contains(&rejected.errors, b"docs/language.md"),
            format!("array/trim edit omitted its construct, rule or reference: {}", display_bytes(&rejected.errors)));
        let rewritten = diagnostic_edits::apply(&item.source, &rejected.errors, &path(config, &item.name))
            .map_err(|error: String| -> String { format!("{}: {error}; diagnostic {}", item.name, display_bytes(&rejected.errors)) })?;
        write(directory, &item.name, rewritten.as_bytes())?;
        run_value(world, executable, config, report, &item.name, &item.entry, &item.expected)?;
    }
    let negatives: Vec<(String, String, String)> = vec![
        ("array-custom-protocol.rs".to_string(),
            format!("{array}\nstruct Other;\nimpl Iterator for Other {{ type Item = u8; fn next(&mut self) -> Option<u8> {{ None }} }}\nimpl IntoIterator for Other {{ type IntoIter = Other; fn into_iter(self) -> Other {{ self }} }}\n"),
            "borrowed-array iteration".to_string()),
        ("array-custom-index.rs".to_string(),
            format!("{array}\ntrait Index {{ fn index(&self) -> &u8; }}\nstruct Other {{ value: u8 }}\nimpl Index for Other {{ fn index(&self) -> &u8 {{ &self.value }} }}\n"),
            "borrowed-array iteration".to_string()),
        ("trim-dynamic.rs".to_string(), r#"pub fn probe(value: &str) -> usize { value.trim().len() }
"#.to_string(), "str.trim is unavailable".to_string()),
        ("trim-nonbreaking.rs".to_string(), r#"pub fn probe() -> usize { "\u{a0} x \u{a0}".trim().len() }
"#.to_string(), "str.trim is unavailable".to_string()),
        ("trim-unicode.rs".to_string(), r#"pub fn probe() -> usize { "\u{2003}x\u{2003}".trim().len() }
"#.to_string(), "str.trim is unavailable".to_string()),
        ("trim-grouped.rs".to_string(), r#"pub fn probe() -> usize { (" x ").trim().len() }
"#.to_string(), "str.trim is unavailable".to_string()),
        ("trim-shadowed-path.rs".to_string(), r#"mod std { pub mod str { pub fn trim_ascii(value: &str) -> &str { &value[..0usize] } } }
pub fn probe() -> usize { " x ".trim().len() }
"#.to_string(), "str.trim is unavailable".to_string()),
    ];
    for (name, source, reason) in negatives {
        write(directory, &name, source.as_bytes())?;
        let rejected = invoke(world, executable, &command(config, "check", &name), b"")?;
        report.require(rejected.status == 1 && contains(&rejected.errors, reason.as_bytes())
            && !contains(&rejected.errors, b"LKJSCRIPT_EDITS "),
            format!("unsafe array/trim rewrite or unrelated rejection for {name}: {}", display_bytes(&rejected.errors)));
        if name.starts_with("trim-") {
            report.require(contains(&rejected.errors, b"bytes 9..13 and 32") && contains(&rejected.errors, b"Unicode spaces are preserved"),
                format!("ASCII-only trim distinction missing for {name}"));
        }
    }
    // Observe the deliberately different Unicode behavior of the proposed API.
    let explicit = r#"pub fn probe() -> usize {
    assert!(std::str::trim_ascii("\u{a0} x \u{a0}") == "\u{a0} x \u{a0}");
    assert!(std::str::trim_ascii("\u{2003}x\u{2003}") == "\u{2003}x\u{2003}");
    std::str::trim_ascii(" \t x \r\n").len()
}
"#;
    write(directory, "trim-explicit-ascii.rs", explicit.as_bytes())?;
    run_value(world, executable, config, report, "trim-explicit-ascii.rs", "probe", "1\n")?;
    Ok(())
}
