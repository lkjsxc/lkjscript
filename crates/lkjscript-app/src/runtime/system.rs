use std::process::ExitCode;

#[cfg(target_os = "linux")]
use lkjscript_runtime::UnixControlClient;
use lkjscript_runtime::{
    ControlOperation, ControlRequest, ControlResponse, ControlSuccess, ServiceBundle,
    ServiceConfiguration,
};

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
    let (operation, endpoint) = match arguments {
        [_, operation, flag, endpoint] if flag == "--endpoint" => {
            let operation = match operation.as_str() {
                "describe" => ControlOperation::Describe,
                "status" => ControlOperation::Status,
                "stop" => ControlOperation::Shutdown,
                _ => return Err("system operation must be describe, status, or stop".to_string()),
            };
            (operation, endpoint)
        }
        _ => {
            return Err(
                "usage: lkjscript system describe|status|stop --endpoint LOCAL-SOCKET; \
                 or system install --output DIR --principal UID --coordinator ID; \
                 or system uninstall --output DIR"
                    .to_string(),
            )
        }
    };
    let request_id = u64::from(std::process::id()).max(1);
    let idempotency_id = if operation.modifies() {
        let mut bytes = operation_name(operation).as_bytes().to_vec();
        bytes.extend_from_slice(endpoint.as_bytes());
        bytes.extend_from_slice(&request_id.to_le_bytes());
        lkjscript_contracts::sha256(&bytes)
    } else {
        [0; 32]
    };
    let request = ControlRequest::current(request_id, idempotency_id, operation)
        .map_err(|error| error.to_string())?;
    let response = UnixControlClient::new(endpoint)
        .call(&request)
        .map_err(|error| error.to_string())?;
    print(response)?;
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
            let result = ServiceBundle::remove_from(std::path::Path::new(directory))
                .map_err(|error| error.to_string())
                .map(|()| {
                    println!("service definitions uninstalled: {directory}");
                    ExitCode::SUCCESS
                });
            Some(result)
        }
        _ => None,
    }
}

fn print(response: ControlResponse) -> Result<(), String> {
    match response
        .result
        .map_err(|failure| format!("system control failed: {failure:?}"))?
    {
        ControlSuccess::Description {
            platform_revision,
            contract_digest,
            product,
        } => {
            println!("product: {product}");
            println!("platform-revision: {platform_revision}");
            println!("contract-digest: {contract_digest}");
        }
        ControlSuccess::Status {
            coordinator,
            clean_shutdown,
            control_sequence,
            applications,
        } => {
            println!("coordinator: {coordinator}");
            println!("previous-clean-shutdown: {clean_shutdown}");
            println!("control-sequence: {control_sequence}");
            println!("applications: {applications}");
        }
        ControlSuccess::ShutdownAccepted => println!("shutdown: accepted"),
    }
    Ok(())
}

const fn operation_name(operation: ControlOperation) -> &'static str {
    match operation {
        ControlOperation::Describe => "describe",
        ControlOperation::Status => "status",
        ControlOperation::Shutdown => "stop",
    }
}
