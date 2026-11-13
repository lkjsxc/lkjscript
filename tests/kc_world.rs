//! Exercise the exact World driver through the existing Rust capability adapter.
//! Native self-compilation remains a separate observation after resource work.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
use std::fs;
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

struct Workspace(PathBuf);
impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn successful(output: &Output) {
    assert!(
        output.status.success(),
        "status {:?}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
}

fn rejected(output: &Output, message: &str) {
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty(), "failure emitted artifact bytes");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(message),
        "missing {message:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn program(main: bool) -> String {
    let mut text = String::from(
        "type \"()\" unit\ntype \"bool\" bool\ntype \"i32\" int 32 signed\n\
         type \"i64\" int 64 signed\ntype \"usize\" int 64 unsigned\n\
         type \"u8\" int 8 unsigned\ntype \"f64\" f64\n\
         fn \"probe\" 0 3\nlocal \"result\" 3 let\n@1 int 0 42\n@2 return 0\nendfn\n",
    );
    if main {
        text.push_str("fn \"main\" 0 3\nlocal \"result\" 3 let\n@3 int 0 7\n@4 return 0\nendfn\n");
    }
    text
}

#[test]
fn world_kc_driver_checks_declared_inputs_and_emits_identical_bytes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let directory = root
        .join("target/kc-world")
        .join(std::process::id().to_string());
    fs::create_dir_all(&directory).unwrap();
    let workspace = Workspace(directory);
    let harness = workspace.0.join("host.rs");
    let binary = workspace.0.join("host");
    // Only the capability adapter is added. include! compiles the unchanged
    // driver and its production module declarations, without source rewriting.
    fs::write(
        &harness,
        format!(
            "#![allow(dead_code, unused_imports)]\n\
             #[path = {adapter:?}] mod host;\n\
             mod driver {{\n\
                 use crate::host::{{Directory, World}};\n\
                 include!({driver:?});\n\
             }}\n\
             pub(crate) use driver::facts;\n\
             pub use driver::{{check, diagnostic, k, k_text, kc, lower, native, runtime}};\n\
             fn main() {{ host::enter(driver::main); }}\n",
            adapter = root.join("bridge/output.rs"),
            driver = root.join("src/kc_main.rs"),
        ),
    )
    .unwrap();
    let compiled = Command::new("rustc")
        .args([
            "--edition=2024",
            "-C",
            "overflow-checks=yes",
            "-C",
            "opt-level=1",
        ])
        .arg(&harness)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    successful(&compiled);
    let run = |arguments: &[&str]| {
        Command::new(&binary)
            .current_dir(&workspace.0)
            .args(arguments)
            .output()
            .unwrap()
    };

    let runtime = Command::new(env!("CARGO_BIN_EXE_lkjscript"))
        .arg("runtime-k")
        .output()
        .unwrap();
    successful(&runtime);
    fs::write(workspace.0.join("runtime.k"), runtime.stdout).unwrap();
    fs::write(workspace.0.join("probe.k"), program(false)).unwrap();
    fs::write(workspace.0.join("both.k"), program(true)).unwrap();
    fs::write(
        workspace.0.join("uninitialized.k"),
        program(false).replace("@1 int 0 42\n", ""),
    )
    .unwrap();
    fs::write(workspace.0.join("malformed.k"), "not K\n").unwrap();
    fs::write(workspace.0.join("invalid-utf8.k"), [255u8]).unwrap();

    let checked = run(&["probe.k", "--check"]);
    successful(&checked);
    assert!(checked.stdout.is_empty());
    rejected(&run(&["uninitialized.k", "--check"]), "uninitialized.k:");
    rejected(
        &run(&["malformed.k", "--check"]),
        "malformed.k:1:1: E_K_TEXT",
    );
    rejected(&run(&["probe.k"]), "--runtime is required");
    rejected(&run(&["missing.k", "--check"]), "missing.k: open failed");
    rejected(
        &run(&["invalid-utf8.k", "--check"]),
        "invalid-utf8.k: K text is not valid UTF-8",
    );
    rejected(&run(&[]), "usage: kc");
    rejected(&run(&["probe.k", "--unknown"]), "unknown option --unknown");
    rejected(&run(&["probe.k", "--runtime"]), "--runtime needs a value");
    rejected(
        &run(&["probe.k", "--runtime", "malformed.k"]),
        "malformed.k:1:1: E_K_TEXT",
    );
    rejected(
        &run(&["probe.k", "--runtime", "probe.k"]),
        "probe.k:0:0: E_RUNTIME",
    );
    rejected(
        &run(&["probe.k", "--runtime", "runtime.k", "--entry", "absent"]),
        "probe.k:0:0: E_ENTRY unknown function absent",
    );
    let invalid_argument = Command::new(&binary)
        .current_dir(&workspace.0)
        .arg(std::ffi::OsString::from_vec(vec![255]))
        .output()
        .unwrap();
    rejected(&invalid_argument, "arguments must be valid UTF-8");

    for (program, entry) in [
        ("probe.k", None),
        ("both.k", None),
        ("both.k", Some("probe")),
    ] {
        let mut arguments = vec![program, "--runtime", "runtime.k"];
        if let Some(entry) = entry {
            arguments.extend(["--entry", entry]);
        }
        let actual = run(&arguments);
        successful(&actual);
        assert!(actual.stdout.starts_with(b"\x7fELF"));
        let expected = Command::new(env!("CARGO_BIN_EXE_kc"))
            .current_dir(&workspace.0)
            .args(&arguments)
            .args(["-o", "expected.elf"])
            .output()
            .unwrap();
        successful(&expected);
        assert_eq!(
            actual.stdout,
            fs::read(workspace.0.join("expected.elf")).unwrap()
        );
    }
}
