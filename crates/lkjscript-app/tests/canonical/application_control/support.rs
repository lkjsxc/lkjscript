use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::Duration;

pub(super) struct Daemon {
    child: Option<Child>,
    endpoint: PathBuf,
}

impl Daemon {
    pub(super) fn start(state: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let endpoint = state.join("control.sock");
        if endpoint.exists() {
            std::fs::remove_file(&endpoint)?;
        }
        let child = Command::new(env!("CARGO_BIN_EXE_lkjscriptd"))
            .args([
                "--foreground",
                "--state-dir",
                state.to_str().ok_or("state path UTF-8")?,
                "--principal",
                &current_user()?.to_string(),
                "--coordinator",
                "8801",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        let mut daemon = Self {
            child: Some(child),
            endpoint,
        };
        for _ in 0..6_000 {
            if daemon.endpoint.exists() {
                return Ok(daemon);
            }
            if daemon
                .child
                .as_mut()
                .ok_or("daemon child missing")?
                .try_wait()?
                .is_some()
            {
                return Err("daemon exited before control endpoint".into());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        Err("daemon control endpoint timeout".into())
    }

    pub(super) fn kill(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut child = self.child.take().ok_or("daemon child missing")?;
        child.kill()?;
        child.wait()?;
        Ok(())
    }

    pub(super) fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let output = cli(&[
            "system".into(),
            "stop".into(),
            "--endpoint".into(),
            self.endpoint.to_string_lossy().into_owned(),
        ])?;
        assert!(output.status.success());
        let status = self.child.take().ok_or("daemon child missing")?.wait()?;
        assert!(status.success());
        Ok(())
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

pub(super) fn lifecycle(
    endpoint: &str,
    operation: &str,
) -> Result<Output, Box<dyn std::error::Error>> {
    cli(&[
        "system".into(),
        "app".into(),
        operation.into(),
        "--endpoint".into(),
        endpoint.into(),
        "--application".into(),
        "1".into(),
    ])
}

pub(super) fn invoke(endpoint: &str) -> Result<Output, Box<dyn std::error::Error>> {
    cli(&[
        "system".into(),
        "app".into(),
        "invoke".into(),
        "--endpoint".into(),
        endpoint.into(),
        "--application".into(),
        "1".into(),
        "--".into(),
    ])
}

pub(super) fn cli(arguments: &[String]) -> Result<Output, Box<dyn std::error::Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_lkjscript"))
        .args(arguments)
        .output()?)
}

fn current_user() -> Result<u32, Box<dyn std::error::Error>> {
    let output = Command::new("id").arg("-u").output()?;
    if !output.status.success() {
        return Err("id -u failed".into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().parse()?)
}
