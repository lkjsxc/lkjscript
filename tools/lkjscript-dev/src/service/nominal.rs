//! Raw-client acceptance for a separately authored concrete nominal session.
use crate::{
    error::DevError,
    process,
    raw_websocket::{RawMessage, RawWebSocket},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    time::{Duration, Instant},
};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Observation {
    pub messages: Vec<Value>,
    pub accept_matches: bool,
    pub close_code: u16,
    pub cleanup_complete: bool,
}

pub(crate) fn validate(value: &Observation) -> Result<(), DevError> {
    if value.messages
        != [
            json!({"first":1,"second":"a"}),
            json!({"first":2,"second":"ab"}),
        ]
        || !value.accept_matches
        || value.close_code != 1000
        || !value.cleanup_complete
    {
        return Err(DevError::corrupt(
            "nominal session omitted exact ordered state messages or clean close",
        ));
    }
    Ok(())
}

pub(crate) fn state_probe_command(
    mut arguments: impl Iterator<Item = std::ffi::OsString>,
) -> Result<u8, DevError> {
    let descriptor = crate::next_utf8(&mut arguments, "deployment")?.ok_or_else(|| {
        DevError::usage("session probe requires deployment, source container and digest")
    })?;
    let source = crate::next_utf8(&mut arguments, "source")?
        .ok_or_else(|| DevError::usage("session probe requires source container"))?;
    let transport = crate::next_utf8(&mut arguments, "transport")?
        .ok_or_else(|| DevError::usage("session probe requires exact transport digest"))?;
    if arguments.next().is_some() {
        return Err(DevError::usage("session probe takes three arguments"));
    }
    let source = process::read_bounded(Path::new(&source), 268_435_456)?;
    let observed = lkjscript::platform::contributor::nominal_session_state_probe(
        Path::new(&descriptor),
        &source,
        &transport,
    )
    .map_err(|error| DevError::corrupt(error.to_string()))?;
    println!("{}", serde_json::to_string(&observed)?);
    Ok(0)
}

pub(crate) fn probe(
    binary: &Path,
    standalone: &Path,
    evidence: &Path,
) -> Result<(Observation, process::ProcessObservation), DevError> {
    let result = probe_script(
        binary,
        standalone,
        evidence,
        "nominal-session",
        1,
        &[
            (0, "a", json!({"first":1,"second":"a"})),
            (0, "b", json!({"first":2,"second":"ab"})),
        ],
    )?;
    validate(&result.0)?;
    Ok(result)
}

pub(crate) fn probe_script(
    binary: &Path,
    standalone: &Path,
    evidence: &Path,
    label: &str,
    session_count: usize,
    steps: &[(usize, &str, Value)],
) -> Result<(Observation, process::ProcessObservation), DevError> {
    let stdout = evidence.join(format!("{label}.stdout"));
    let spec = process::ProcessSpec {
        command: vec![
            binary.display().to_string(),
            "serve".into(),
            "--deployment".into(),
            standalone
                .join("session.deployment.json")
                .display()
                .to_string(),
        ],
        cwd: standalone.to_path_buf(),
        environment: BTreeMap::from([("LANG".into(), "C.UTF-8".into())]),
        timeout: Duration::from_secs(120),
        maximum_stdout_bytes: 4 * 1024 * 1024,
        maximum_stderr_bytes: 4 * 1024 * 1024,
        stdout_path: stdout.clone(),
        stderr_path: evidence.join(format!("{label}.stderr")),
        unavailable_exit_code: None,
    };
    let control = process::ProcessControl::default();
    let child = control.clone();
    let evidence_root = evidence.to_path_buf();
    let thread = std::thread::Builder::new()
        .name("nominal-session-observer".into())
        .spawn(move || process::run_controlled(&spec, &evidence_root, &child))?;
    let observed = (|| {
        let start = Instant::now();
        let ready: Value = loop {
            if start.elapsed() > Duration::from_secs(30) || thread.is_finished() {
                return Err(DevError::corrupt("nominal session failed before readiness"));
            }
            if stdout.exists() {
                let bytes = process::read_bounded(&stdout, 4 * 1024 * 1024)?;
                if let Some(end) = bytes.iter().position(|byte| *byte == b'\n') {
                    break serde_json::from_slice(&bytes[..end])?;
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        if ready["event"] != "ready" || ready["ok"] != true {
            return Err(DevError::corrupt("nominal session was not ready"));
        }
        let address = ready["local_address"]
            .as_str()
            .ok_or_else(|| DevError::corrupt("nominal listener omitted"))?
            .parse()
            .map_err(|_| DevError::corrupt("nominal listener invalid"))?;
        let map = |error| DevError::corrupt(format!("nominal raw WebSocket: {error:?}"));
        let mut result = Observation {
            accept_matches: true,
            ..Default::default()
        };
        let mut clients = Vec::new();
        for _ in 0..session_count {
            let (client, handshake) =
                RawWebSocket::connect(address, "/", &[], Duration::from_secs(10)).map_err(map)?;
            result.accept_matches &= handshake.accept_matches;
            clients.push(client);
        }
        let mut trace = Vec::new();
        for (session, input, expected) in steps {
            let client = clients
                .get_mut(*session)
                .ok_or_else(|| DevError::corrupt("session script index invalid"))?;
            client.send_text(input).map_err(map)?;
            let RawMessage::Text(message) = client.read_message().map_err(map)? else {
                return Err(DevError::corrupt(
                    "nominal session returned a non-text message",
                ));
            };
            let actual: Value = serde_json::from_str(&message)?;
            trace.push(json!({"session":session,"sent":input,"received":actual}));
            fs::write(
                evidence.join(format!("{label}-wire.json")),
                crate::evidence::encode_json(&trace)?,
            )?;
            if &actual != expected {
                return Err(DevError::corrupt(
                    "session reply differs from independent full-state and traversal expectation",
                ));
            }
            result.messages.push(actual);
        }
        for mut client in clients {
            client.send_close(1000, "done").map_err(map)?;
            let RawMessage::Close { code, .. } = client.read_message().map_err(map)? else {
                return Err(DevError::corrupt(
                    "nominal session omitted close acknowledgement",
                ));
            };
            result.close_code = code.unwrap_or(0);
            if result.close_code != 1000 {
                return Err(DevError::corrupt("session close acknowledgement changed"));
            }
            client.disconnect().map_err(map)?;
        }
        Ok(result)
    })();
    control.interrupt();
    let terminal = thread
        .join()
        .map_err(|_| DevError::infrastructure("nominal session observer panicked"))?;
    let mut observed = observed?;
    observed.cleanup_complete = terminal.status == process::ProcessStatus::Passed;
    if !observed.accept_matches || !observed.cleanup_complete {
        return Err(DevError::corrupt(
            "session handshake or process cleanup incomplete",
        ));
    }
    fs::write(
        evidence.join(format!("{label}-messages.json")),
        crate::evidence::encode_json(&observed)?,
    )?;
    Ok((observed, terminal))
}
