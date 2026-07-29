use std::io::Write;

use lkjscript_runtime::{
    ControlResponse, ControlSuccess, ControlledApplication, ControlledApplicationState,
};

pub(super) fn print(response: ControlResponse) -> Result<(), String> {
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
        ControlSuccess::Application(application) => print_application(&application),
        ControlSuccess::Applications(applications) => {
            for application in applications {
                print_application(&application);
            }
        }
        ControlSuccess::ApplicationRemoved { application } => {
            println!("application: {application}");
            println!("removed: true");
        }
        ControlSuccess::ApplicationInvoked {
            application,
            outcome,
            output,
        } => {
            std::io::stdout()
                .write_all(&output)
                .and_then(|()| std::io::stdout().flush())
                .map_err(|error| format!("write application output: {error}"))?;
            eprintln!("application {application} outcome: {}", outcome.summary());
        }
    }
    Ok(())
}

fn print_application(application: &ControlledApplication) {
    let state = match application.state {
        ControlledApplicationState::Installed => "installed",
        ControlledApplicationState::Running => "running",
        ControlledApplicationState::Stopped => "stopped",
        ControlledApplicationState::Failed => "failed",
    };
    println!("application: {}", application.application);
    println!("name: {}", application.name);
    println!(
        "desired: {}",
        if application.desired_running {
            "running"
        } else {
            "stopped"
        }
    );
    println!("state: {state}");
    println!(
        "database: {}",
        if application.database_attached {
            "attached"
        } else {
            "detached"
        }
    );
    if let Some(incarnation) = application.incarnation {
        println!("incarnation: {incarnation}");
    }
    if let Some(process) = application.process {
        println!("process: {process}");
    }
}
