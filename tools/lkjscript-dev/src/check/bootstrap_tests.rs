#[test]
fn shared_bootstrap_admits_only_source_bound_cold_artifacts() {
    let output = std::process::Command::new("bash")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../bootstrap-verifier.test.sh"
        ))
        .output()
        .expect("execute the real bootstrap in an owned fixture");
    assert!(
        output.status.success(),
        "bootstrap fixture failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
