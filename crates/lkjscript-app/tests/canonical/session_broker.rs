use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};

use super::application_control_support::{cli, Daemon};

#[cfg(target_os = "linux")]
#[test]
fn broker_registers_heartbeats_lists_and_unregisters_without_desktop_authority(
) -> Result<(), Box<dyn std::error::Error>> {
    let state = std::env::temp_dir().join(format!("lkjscript-session-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&state);
    std::fs::create_dir(&state)?;
    let endpoint = state.join("control.sock").to_string_lossy().into_owned();
    let mut daemon = Daemon::start(&state)?;
    let mut broker = Command::new(env!("CARGO_BIN_EXE_lkjscript-session"))
        .args([
            "--foreground",
            "--endpoint",
            &endpoint,
            "--backend",
            "none",
            "--heartbeat-limit",
            "1",
        ])
        .env_clear()
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut output = BufReader::new(broker.stdout.take().ok_or("broker stdout")?);
    let mut ready = String::new();
    output.read_line(&mut ready)?;
    assert!(
        ready.contains("lkjscript-session ready session=1"),
        "{ready}"
    );

    let listed = cli(&[
        "system".into(),
        "session".into(),
        "list".into(),
        "--endpoint".into(),
        endpoint.clone(),
    ])?;
    let listed = String::from_utf8(listed.stdout)?;
    assert!(listed.contains("session: 1"), "{listed}");
    assert!(
        listed.contains(&format!("process: {}", broker.id())),
        "{listed}"
    );
    assert!(listed.contains("backend: none"), "{listed}");

    let status = broker.wait()?;
    assert!(status.success());
    let mut remainder = String::new();
    output.read_to_string(&mut remainder)?;
    assert!(remainder.contains("lkjscript-session stopped session=1"));
    let after = cli(&[
        "system".into(),
        "session".into(),
        "list".into(),
        "--endpoint".into(),
        endpoint,
    ])?;
    assert!(after.status.success());
    assert!(after.stdout.is_empty());
    daemon.stop()?;
    std::fs::remove_dir_all(state)?;
    Ok(())
}
