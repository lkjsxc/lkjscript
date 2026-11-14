use std::path::Path;
use std::process::Command;

#[test]
fn host_entries_reject_view_results_before_running_any_route() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for engine in ["native", "interpreter"] {
        let output = Command::new(env!("CARGO_BIN_EXE_lkjscript"))
            .arg("run")
            .arg(root.join("tests/conformance/pass/inline_view_values.rs"))
            .args(["--entry", "text", "--engine", engine])
            .output()
            .expect("start the view-result entry probe");
        assert!(!output.status.success(), "{engine} accepted a view result");
        assert!(output.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("E_ENTRY host entry result must be unit or a scalar"),
            "{engine}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
