use std::process::ExitCode;

#[cfg(target_os = "linux")]
use lkjscript_runtime::UnixControlClient;
use lkjscript_runtime::{ControlOperation, ControlRequest, ServiceBundle, ServiceConfiguration};

mod application;
mod output;

pub(super) fn command(arguments: &[String]) -> Result<ExitCode, String> {
    if let Some(result) = service_command(arguments) {
        return result;
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = arguments;
        return Err("system local control is unsupported on this host".to_string());
    }
    #[cfg(target_os = "linux")]
    command_linux(arguments)
}

#[cfg(target_os = "linux")]
fn command_linux(arguments: &[String]) -> Result<ExitCode, String> {
    let (operation, endpoint) = if arguments.get(1).map(String::as_str) == Some("app") {
        application::parse(arguments)?
    } else {
        match arguments {
            [_, operation, flag, endpoint] if flag == "--endpoint" => {
                let operation = match operation.as_str() {
                    "describe" => ControlOperation::Describe,
                    "status" => ControlOperation::Status,
                    "stop" => ControlOperation::Shutdown,
                    _ => return Err(usage()),
                };
                (operation, endpoint.clone())
            }
            _ => return Err(usage()),
        }
    };
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "system clock precedes Unix epoch")?
        .as_nanos();
    let mut identity_bytes = Vec::new();
    identity_bytes.push(operation.kind());
    identity_bytes.extend_from_slice(endpoint.as_bytes());
    identity_bytes.extend_from_slice(&std::process::id().to_le_bytes());
    identity_bytes.extend_from_slice(&nonce.to_le_bytes());
    let idempotency_id = lkjscript_contracts::sha256(&identity_bytes);
    let mut request_id = u64::from_le_bytes(
        idempotency_id[..8]
            .try_into()
            .map_err(|_| "request identity width")?,
    );
    if request_id == 0 {
        request_id = 1;
    }
    let idempotency_id = if operation.modifies() {
        idempotency_id
    } else {
        [0; 32]
    };
    let request = ControlRequest::current(request_id, idempotency_id, operation)
        .map_err(|error| error.to_string())?;
    let response = UnixControlClient::new(endpoint)
        .call(&request)
        .map_err(|error| error.to_string())?;
    output::print(response)?;
    Ok(ExitCode::SUCCESS)
}

fn service_command(arguments: &[String]) -> Option<Result<ExitCode, String>> {
    match arguments {
        [_, operation, output_flag, directory, principal_flag, principal, coordinator_flag, coordinator]
            if operation == "install"
                && output_flag == "--output"
                && principal_flag == "--principal"
                && coordinator_flag == "--coordinator" =>
        {
            let result = (|| {
                let principal = principal
                    .parse::<u32>()
                    .map_err(|_| "service principal must be a u32")?;
                let coordinator = coordinator
                    .parse::<u64>()
                    .map_err(|_| "service coordinator must be a nonzero u64")?;
                ServiceBundle::new(ServiceConfiguration {
                    principal,
                    coordinator,
                })
                .and_then(|bundle| bundle.write_to(std::path::Path::new(directory)))
                .map_err(|error| error.to_string())?;
                println!("service definitions installed: {directory}");
                Ok(ExitCode::SUCCESS)
            })();
            Some(result)
        }
        [_, operation, output_flag, directory]
            if operation == "uninstall" && output_flag == "--output" =>
        {
            Some(
                ServiceBundle::remove_from(std::path::Path::new(directory))
                    .map_err(|error| error.to_string())
                    .map(|()| {
                        println!("service definitions uninstalled: {directory}");
                        ExitCode::SUCCESS
                    }),
            )
        }
        _ => None,
    }
}

fn usage() -> String {
    "usage: lkjscript system describe|status|stop --endpoint SOCKET; \
     system app install|list|start|stop|restart|remove|invoke ...; \
     system install --output DIR --principal UID --coordinator ID; \
     system uninstall --output DIR"
        .into()
}
