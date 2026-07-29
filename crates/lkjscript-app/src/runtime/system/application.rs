use lkjscript_runtime::{ApplicationInstallRequest, ControlOperation};

pub(super) fn parse(arguments: &[String]) -> Result<(ControlOperation, String), String> {
    if arguments.get(2).map(String::as_str) == Some("install") {
        return parse_install(arguments);
    }
    match arguments {
        [_, app, operation, endpoint_flag, endpoint]
            if app == "app" && endpoint_flag == "--endpoint" =>
        {
            let operation = match operation.as_str() {
                "list" => ControlOperation::ApplicationList,
                _ => return Err(usage()),
            };
            Ok((operation, endpoint.clone()))
        }
        [_, app, operation, endpoint_flag, endpoint, application_flag, application]
            if app == "app"
                && endpoint_flag == "--endpoint"
                && application_flag == "--application" =>
        {
            let application = application
                .parse::<u64>()
                .map_err(|_| "application identity must be a nonzero u64")?;
            let operation = match operation.as_str() {
                "start" => ControlOperation::ApplicationStart { application },
                "stop" => ControlOperation::ApplicationStop { application },
                "restart" => ControlOperation::ApplicationRestart { application },
                "remove" => ControlOperation::ApplicationRemove { application },
                _ => return Err(usage()),
            };
            Ok((operation, endpoint.clone()))
        }
        [_, app, operation, endpoint_flag, endpoint, application_flag, application, separator, rest @ ..]
            if app == "app"
                && operation == "invoke"
                && endpoint_flag == "--endpoint"
                && application_flag == "--application"
                && separator == "--" =>
        {
            let application = application
                .parse::<u64>()
                .map_err(|_| "application identity must be a nonzero u64")?;
            Ok((
                ControlOperation::ApplicationInvoke {
                    application,
                    arguments: rest.to_vec(),
                },
                endpoint.clone(),
            ))
        }
        _ => Err(usage()),
    }
}

fn parse_install(arguments: &[String]) -> Result<(ControlOperation, String), String> {
    const FLAGS: [(usize, &str); 8] = [
        (3, "--endpoint"),
        (5, "--name"),
        (7, "--package"),
        (9, "--root"),
        (11, "--entry"),
        (13, "--capabilities"),
        (15, "--concurrent"),
        (17, "--total"),
    ];
    if arguments.len() != 19
        || arguments.get(1).map(String::as_str) != Some("app")
        || FLAGS
            .iter()
            .any(|(index, flag)| arguments.get(*index).map(String::as_str) != Some(*flag))
    {
        return Err(usage());
    }
    install(
        &arguments[4],
        &arguments[6],
        &arguments[8],
        &arguments[10],
        &arguments[12],
        &arguments[14],
        &arguments[16],
        &arguments[18],
    )
}

#[allow(clippy::too_many_arguments)]
fn install(
    endpoint: &str,
    name: &str,
    package: &str,
    root: &str,
    entry: &str,
    capabilities: &str,
    concurrent: &str,
    total: &str,
) -> Result<(ControlOperation, String), String> {
    if name.is_empty() || name.len() > 64 {
        return Err("application name must contain 1..=64 bytes".into());
    }
    let package = lkjscript_contracts::ContractDigest::from_hex(package)
        .map(lkjscript_contracts::ContractDigest::as_bytes)
        .filter(|digest| *digest != [0; 32])
        .ok_or("package identity must be full lowercase nonzero SHA-256")?;
    let root = std::path::Path::new(root)
        .canonicalize()
        .map_err(|error| format!("canonicalize package root: {error}"))?;
    let root = root
        .to_str()
        .ok_or("canonical package root is not UTF-8")?
        .to_owned();
    lkjscript_host::ApplicationPath::parse(entry.to_owned()).map_err(|error| error.to_string())?;
    let mut capabilities = if capabilities == "none" {
        Vec::new()
    } else {
        capabilities
            .split(',')
            .map(|name| {
                lkjscript_core::CapabilityKind::parse(name)
                    .ok_or_else(|| format!("unknown capability: {name}"))
            })
            .collect::<Result<Vec<_>, _>>()?
    };
    capabilities.sort_unstable();
    if capabilities.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("capability grants must be unique".into());
    }
    let max_concurrent_invocations = concurrent
        .parse::<u16>()
        .map_err(|_| "concurrent quota must be 1..=64")?;
    let max_total_invocations = total
        .parse::<u64>()
        .map_err(|_| "total quota must be nonzero u64")?;
    Ok((
        ControlOperation::ApplicationInstall(ApplicationInstallRequest {
            name: name.to_owned(),
            package,
            package_root: root,
            entry: entry.to_owned(),
            capabilities,
            max_concurrent_invocations,
            max_total_invocations,
        }),
        endpoint.to_owned(),
    ))
}

fn usage() -> String {
    "usage: system app install --endpoint SOCKET --name NAME --package SHA256 --root DIR \
     --entry RELATIVE --capabilities CSV|none --concurrent N --total N; \
     system app list --endpoint SOCKET; system app start|stop|restart|remove \
     --endpoint SOCKET --application ID; system app invoke --endpoint SOCKET \
     --application ID -- [ARG ...]"
        .into()
}
