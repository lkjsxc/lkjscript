use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use lkjscript_runtime::{
    ControlOperation, ControlRequest, ControlSuccess, SessionBackend, UnixControlClient,
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("lkjscript-session: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let configuration = Configuration::parse(&std::env::args().skip(1).collect::<Vec<_>>())?;
    let instance = broker_instance(&configuration.endpoint)?;
    let mut broker = Broker {
        client: UnixControlClient::new(&configuration.endpoint),
        instance,
        request: 1,
    };
    let registered = broker.call(ControlOperation::SessionRegister {
        broker_instance: instance,
        backend: SessionBackend::None,
    })?;
    let ControlSuccess::Session(session) = registered else {
        return Err("coordinator returned a non-session registration".into());
    };
    println!(
        "lkjscript-session ready session={} process={} backend=none",
        session.session, session.process
    );
    std::io::stdout()
        .flush()
        .map_err(|error| format!("flush readiness: {error}"))?;
    let mut successful = 0_u32;
    let mut failures = 0_u8;
    loop {
        std::thread::sleep(Duration::from_secs(2));
        match broker.call(ControlOperation::SessionHeartbeat {
            session: session.session,
        }) {
            Ok(ControlSuccess::Session(_)) => {
                successful = successful.saturating_add(1);
                failures = 0;
                if configuration.heartbeat_limit == Some(successful) {
                    broker.call(ControlOperation::SessionUnregister {
                        session: session.session,
                    })?;
                    println!("lkjscript-session stopped session={}", session.session);
                    return Ok(());
                }
            }
            Ok(_) => return Err("coordinator returned a non-session heartbeat".into()),
            Err(error) => {
                failures = failures.saturating_add(1);
                if failures == 3 {
                    return Err(format!("three consecutive control failures: {error}"));
                }
            }
        }
    }
}

struct Broker {
    client: UnixControlClient,
    instance: [u8; 32],
    request: u64,
}

impl Broker {
    fn call(&mut self, operation: ControlOperation) -> Result<ControlSuccess, String> {
        let request_id = self.request;
        self.request = self
            .request
            .checked_add(1)
            .ok_or("broker request identity exhausted")?;
        let mut idempotency = Vec::new();
        idempotency.extend_from_slice(&self.instance);
        idempotency.extend_from_slice(&request_id.to_le_bytes());
        idempotency.push(operation.kind());
        let request = ControlRequest::current(
            request_id,
            lkjscript_contracts::sha256(&idempotency),
            operation,
        )
        .map_err(|error| error.to_string())?;
        let response = self
            .client
            .call(&request)
            .map_err(|error| error.to_string())?;
        response
            .result
            .map_err(|failure| format!("control rejected request: {failure:?}"))
    }
}

struct Configuration {
    endpoint: PathBuf,
    heartbeat_limit: Option<u32>,
}

impl Configuration {
    fn parse(arguments: &[String]) -> Result<Self, String> {
        if arguments.first().map(String::as_str) != Some("--foreground") {
            return Err(usage());
        }
        let mut endpoint = None;
        let mut backend = None;
        let mut heartbeat_limit = None;
        let mut cursor = 1;
        while cursor < arguments.len() {
            let value = arguments.get(cursor + 1).ok_or_else(usage)?;
            match arguments[cursor].as_str() {
                "--endpoint" => endpoint = Some(PathBuf::from(value)),
                "--backend" if value == "none" => backend = Some(()),
                "--heartbeat-limit" => {
                    let limit = value
                        .parse::<u32>()
                        .map_err(|_| "heartbeat limit must be a nonzero u32")?;
                    heartbeat_limit = Some(
                        std::num::NonZeroU32::new(limit)
                            .ok_or("heartbeat limit must be nonzero")?
                            .get(),
                    );
                }
                _ => return Err(usage()),
            }
            cursor += 2;
        }
        Ok(Self {
            endpoint: endpoint.ok_or_else(usage)?,
            heartbeat_limit: backend.map(|()| heartbeat_limit).ok_or_else(usage)?,
        })
    }
}

fn broker_instance(endpoint: &std::path::Path) -> Result<[u8; 32], String> {
    let mut identity = Vec::new();
    identity.extend_from_slice(&std::process::id().to_le_bytes());
    identity.extend_from_slice(endpoint.as_os_str().as_encoded_bytes());
    identity.extend_from_slice(&lkjscript_contracts::PLATFORM_REVISION.to_le_bytes());
    let identity = lkjscript_contracts::sha256(&identity);
    if identity == [0; 32] {
        Err("broker instance identity is zero".into())
    } else {
        Ok(identity)
    }
}

fn usage() -> String {
    "usage: lkjscript-session --foreground --endpoint PATH --backend none \
     [--heartbeat-limit NONZERO]"
        .into()
}
