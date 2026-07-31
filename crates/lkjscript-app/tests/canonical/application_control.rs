use std::path::Path;

use super::application_control_support::{cli, invoke, lifecycle, Daemon};

#[cfg(target_os = "linux")]
#[test]
fn daemon_persists_restarts_and_controls_runnable_application(
) -> Result<(), Box<dyn std::error::Error>> {
    let state = std::env::temp_dir().join(format!("lkjscript-app-control-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&state);
    std::fs::create_dir(&state)?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    let endpoint = state.join("control.sock").to_string_lossy().into_owned();
    let mut daemon = Daemon::start(&state)?;
    let install = cli(&[
        "system".into(),
        "app".into(),
        "install".into(),
        "--endpoint".into(),
        endpoint.clone(),
        "--name".into(),
        "persistent-hello".into(),
        "--package".into(),
        "5abaa965951efcfd87b27e57d776a8eddec68e25ae644cf167d62a7b04e47b80".into(),
        "--root".into(),
        root.to_string_lossy().into_owned(),
        "--entry".into(),
        "src/examples/hello/main.lkjscript".into(),
        "--capabilities".into(),
        "stdio".into(),
        "--concurrent".into(),
        "2".into(),
        "--total".into(),
        "8".into(),
    ])?;
    assert!(
        install.status.success(),
        "{}",
        String::from_utf8_lossy(&install.stderr)
    );
    assert!(String::from_utf8(install.stdout)?.contains("application: 1"));
    assert!(lifecycle(&endpoint, "start")?.status.success());
    assert!(lifecycle(&endpoint, "restart")?.status.success());
    let invoked = invoke(&endpoint)?;
    assert!(invoked.status.success());
    let invoked_stderr = String::from_utf8(invoked.stderr)?;
    assert_eq!(invoked.stdout, b"3628800", "{invoked_stderr}");
    assert!(
        invoked_stderr.contains("Returned(unit)"),
        "{invoked_stderr}"
    );
    daemon.kill()?;

    let mut recovered = Daemon::start(&state)?;
    let status = cli(&[
        "system".into(),
        "status".into(),
        "--endpoint".into(),
        endpoint.clone(),
    ])?;
    let recovered_status = String::from_utf8(status.stdout)?;
    assert!(
        recovered_status.contains("previous-clean-shutdown: false"),
        "{recovered_status}"
    );
    let listed = cli(&[
        "system".into(),
        "app".into(),
        "list".into(),
        "--endpoint".into(),
        endpoint.clone(),
    ])?;
    let listed = String::from_utf8(listed.stdout)?;
    assert!(listed.contains("application: 1"));
    assert!(listed.contains("desired: running"));
    assert!(listed.contains("state: running"));
    assert!(listed.contains("database: attached"));
    assert_eq!(invoke(&endpoint)?.stdout, b"3628800");
    assert!(lifecycle(&endpoint, "stop")?.status.success());
    let stopped = cli(&[
        "system".into(),
        "app".into(),
        "list".into(),
        "--endpoint".into(),
        endpoint.clone(),
    ])?;
    assert!(String::from_utf8(stopped.stdout)?.contains("database: detached"));
    assert!(lifecycle(&endpoint, "remove")?.status.success());
    recovered.stop()?;
    std::fs::remove_dir_all(state)?;
    Ok(())
}
