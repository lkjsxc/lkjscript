#[cfg(target_os = "linux")]
fn main() -> std::process::ExitCode {
    match linux::run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("lkjscriptd: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn main() -> std::process::ExitCode {
    eprintln!("lkjscriptd: foreground coordinator transport is unsupported on this host");
    std::process::ExitCode::FAILURE
}

#[cfg(target_os = "linux")]
mod linux {
    use std::error::Error;
    use std::num::NonZeroUsize;
    use std::path::PathBuf;

    use lkjscript_host::PortableDurableStorage;
    use lkjscript_runtime::{
        ControlOperation, CoordinatorIdentity, CoordinatorLease, MachineCoordinator,
        UnixControlServer,
    };

    pub(super) fn run() -> Result<(), Box<dyn Error>> {
        let arguments: Vec<String> = std::env::args().skip(1).collect();
        let configuration = Configuration::parse(&arguments)?;
        let identity = CoordinatorIdentity::new(configuration.coordinator)
            .ok_or("coordinator identity must be nonzero")?;
        let lease = CoordinatorLease::acquire(&configuration.state_directory, identity)?;
        lease.sync()?;
        let storage = PortableDurableStorage::new(&configuration.state_directory)?;
        let mut coordinator = MachineCoordinator::start(
            identity,
            configuration.principal,
            storage,
            NonZeroUsize::new(256).ok_or("cache bound")?,
        )?;
        let socket = configuration.state_directory.join("control.sock");
        let mut control = UnixControlServer::bind(socket, configuration.principal)?;
        println!(
            "lkjscriptd ready coordinator={} platform-revision={} endpoint={}",
            identity.get(),
            lkjscript_contracts::PLATFORM_REVISION,
            control.path().display()
        );
        loop {
            match control.serve_one(|request| coordinator.handle_control(request)) {
                Ok(ControlOperation::Shutdown) => break,
                Ok(_) if configuration.once => break,
                Ok(_) => {}
                Err(error) if configuration.once => return Err(error.into()),
                Err(error) => eprintln!("lkjscriptd control request rejected: {error}"),
            }
        }
        coordinator.shutdown()?;
        drop(control);
        drop(lease);
        Ok(())
    }

    struct Configuration {
        state_directory: PathBuf,
        principal: u32,
        coordinator: u64,
        once: bool,
    }

    impl Configuration {
        fn parse(arguments: &[String]) -> Result<Self, &'static str> {
            if !arguments.iter().any(|argument| argument == "--foreground") {
                return Err("--foreground is required; service launch is an external adapter");
            }
            let state_directory = value(arguments, "--state-dir")
                .map(PathBuf::from)
                .ok_or("--state-dir is required")?;
            let principal = value(arguments, "--principal")
                .and_then(|value| value.parse::<u32>().ok())
                .ok_or("--principal must be a u32")?;
            let coordinator = value(arguments, "--coordinator")
                .and_then(|value| value.parse::<u64>().ok())
                .ok_or("--coordinator must be a nonzero u64")?;
            let once = arguments.iter().any(|argument| argument == "--once");
            let expected = 7 + usize::from(once);
            if arguments.len() != expected {
                return Err(
                    "usage: lkjscriptd --foreground --state-dir PATH --principal UID --coordinator ID [--once]",
                );
            }
            Ok(Self {
                state_directory,
                principal,
                coordinator,
                once,
            })
        }
    }

    fn value<'a>(arguments: &'a [String], name: &str) -> Option<&'a str> {
        arguments
            .windows(2)
            .find(|pair| pair[0] == name)
            .map(|pair| pair[1].as_str())
    }
}
