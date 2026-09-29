use std::{
    ffi::OsString,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    process::ExitCode,
};

enum Action {
    Help,
    Version,
    Serve(SocketAddr),
}

fn arguments(values: impl IntoIterator<Item = OsString>) -> Result<Action, String> {
    let mut values = values.into_iter();
    let mut address = None;
    let mut action = None;
    while let Some(value) = values.next() {
        let value = value.into_string().map_err(|_| "arguments must be UTF-8")?;
        match value.as_str() {
            "--help" | "--version" if action.is_none() && address.is_none() => {
                action = Some(if value == "--help" {
                    Action::Help
                } else {
                    Action::Version
                });
            }
            "--listen" if address.is_none() && action.is_none() => {
                let value = values
                    .next()
                    .ok_or("--listen requires an IP:port")?
                    .into_string()
                    .map_err(|_| "listen address must be UTF-8")?;
                address = Some(
                    value
                        .parse::<SocketAddr>()
                        .map_err(|_| "listen address must be an IP:port")?,
                );
            }
            _ => return Err(format!("unknown, duplicate or conflicting option: {value}")),
        }
    }
    Ok(action.unwrap_or(Action::Serve(
        address.unwrap_or(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8798)),
    )))
}

#[tokio::main(worker_threads = 2)]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("lkjscript-site: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let action = arguments(std::env::args_os().skip(1))?;
    let address = match action {
        Action::Help => {
            println!(
                "lkjscript-site [--listen IP:PORT]\n\nServe embedded public documentation. Default: 127.0.0.1:8798.\nTLS and public hostname routing belong to the reverse proxy.\nNo project filesystem or program execution is exposed."
            );
            return Ok(());
        }
        Action::Version => {
            println!(
                "lkjscript-site {} source={}",
                env!("CARGO_PKG_VERSION"),
                lkjscript_site::revision()
            );
            return Ok(());
        }
        Action::Serve(address) => address,
    };
    #[cfg(unix)]
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let listener = tokio::net::TcpListener::bind(address).await?;
    eprintln!(
        "lkjscript-site listening={} source={}",
        listener.local_addr()?,
        lkjscript_site::revision()
    );
    let shutdown = async move {
        #[cfg(unix)]
        tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = terminate.recv() => {} }
        #[cfg(not(unix))]
        {
            let _ = tokio::signal::ctrl_c().await;
        }
    };
    axum::serve(listener, lkjscript_site::app())
        .with_graceful_shutdown(shutdown)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(values: &[&str]) -> Result<Action, String> {
        arguments(values.iter().map(OsString::from))
    }
    #[test]
    fn command_line_has_no_implicit_public_bind() {
        assert!(
            matches!(parse(&[]), Ok(Action::Serve(address)) if address.ip().is_loopback() && address.port() == 8798)
        );
        assert!(
            matches!(parse(&["--listen", "[::1]:9000"]), Ok(Action::Serve(address)) if address.port() == 9000)
        );
        assert!(matches!(parse(&["--help"]), Ok(Action::Help)));
        for values in [
            vec!["--listen"],
            vec!["--listen", "evil.test:80"],
            vec!["--help", "--version"],
            vec!["--listen", "127.0.0.1:9", "--listen", "127.0.0.1:10"],
            vec!["--unknown"],
        ] {
            assert!(parse(&values).is_err());
        }
    }
}
