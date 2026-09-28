//! Multiple exact services in one process, with joined group termination.

use super::{ProcessTermination, cli_error, write_json};
use futures_util::stream::{FuturesUnordered, StreamExt};
use lkjscript::platform::deployment::shared::{MAXIMUM_SHARED_DEPLOYMENTS, PreparedServiceSet};
use lkjscript::platform::deployment::{PreparedHttpApplication, PreparedSessionApplication};
use lkjscript::platform::execution::ExecutionControl;
use lkjscript::platform::{Diagnostic, DiagnosticClass, PreparedDeployment};
use serde_json::{Value, json};
use std::path::PathBuf;
use tokio::net::TcpListener;
use tokio::sync::watch;

enum Service {
    Http(Box<PreparedHttpApplication>),
    Interactive(Box<PreparedSessionApplication>),
}

impl Service {
    fn new(prepared: &PreparedDeployment) -> Result<Self, Diagnostic> {
        match prepared.observe_redacted().runner.as_str() {
            "http" => prepared.http_application().map(Box::new).map(Self::Http),
            "interactive" => prepared
                .session_application()
                .map(Box::new)
                .map(Self::Interactive),
            _ => Err(cli_error(
                "shared serve requires HTTP or interactive targets",
            )),
        }
    }

    async fn serve(
        self,
        listener: TcpListener,
        mut stop: watch::Receiver<bool>,
    ) -> Result<Value, Diagnostic> {
        let shutdown = async move {
            let _ = stop.wait_for(|stopped| *stopped).await;
        };
        let encoded = match self {
            Self::Http(application) => {
                serde_json::to_value(application.serve(listener, shutdown).await?)
            }
            Self::Interactive(application) => {
                serde_json::to_value(application.serve(listener, shutdown).await?)
            }
        };
        encoded.map_err(|source| {
            infrastructure(
                "shared_serve_receipt",
                format!("joined service receipt could not be encoded: {source}"),
            )
        })
    }
}

fn paths(arguments: &[String]) -> Result<Vec<PathBuf>, Diagnostic> {
    if arguments.is_empty()
        || !arguments.len().is_multiple_of(2)
        || arguments.len() / 2 > MAXIMUM_SHARED_DEPLOYMENTS
    {
        return Err(cli_error(format!(
            "serve requires 1 through {MAXIMUM_SHARED_DEPLOYMENTS} --deployment DESCRIPTOR pairs"
        )));
    }
    arguments
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            if pair[0] != "--deployment" || pair[1].is_empty() {
                Err(cli_error(
                    "serve accepts only --deployment DESCRIPTOR pairs",
                ))
            } else {
                Ok(PathBuf::from(&pair[1]))
            }
        })
        .collect()
}

pub(super) async fn serve(arguments: &[String]) -> Result<(), Diagnostic> {
    let paths = paths(arguments)?;
    let mut signals = ProcessTermination::register("resident_signal")?;
    let control = ExecutionControl::uncancelled();
    let prepare_control = control.clone();
    let runtime = tokio::runtime::Handle::current();
    let mut preparation = tokio::task::spawn_blocking(move || {
        PreparedServiceSet::load(&paths, runtime, &prepare_control)
    });
    // Cancellation requests cooperative stop, then joins the blocking owner. Dropping
    // a JoinHandle or aborting it would not stop a running native preparation call.
    let (outcome, interrupted) = tokio::select! {
        biased;
        _ = signals.wait() => {
            control.cancel();
            (preparation.await, true)
        }
        outcome = &mut preparation => (outcome, false),
    };
    let set = outcome
        .map_err(|_| infrastructure("shared_serve_prepare", "shared preparation owner failed"))??;
    if interrupted {
        let mut error = cancelled();
        set.close_uninvoked(&mut error);
        return Err(error);
    }
    let bound = match bind_all(&set, &mut signals).await {
        Ok(bound) => bound,
        Err(mut error) => {
            set.close_uninvoked(&mut error);
            return Err(error);
        }
    };
    let observation = set.observe().clone();
    let readiness = bound
        .iter()
        .map(|(_, _, ready)| ready.clone())
        .collect::<Vec<_>>();
    if let Err(mut error) = write_json(&json!({
        "ok": true, "event": "ready", "process_id": std::process::id(),
        "shared_runtime": observation, "instances": readiness,
    })) {
        set.close_uninvoked(&mut error);
        return Err(error);
    }
    // Services now own their code and private adapters. This set must not pin code
    // after the last service using it has stopped.
    drop(set);
    let (stop, receiver) = watch::channel(false);
    let mut running = FuturesUnordered::new();
    for (index, (service, listener, _)) in bound.into_iter().enumerate() {
        let receiver = receiver.clone();
        running.push(async move { (index, service.serve(listener, receiver).await) });
    }
    drop(receiver);
    let mut stopping = false;
    let mut failure: Option<Diagnostic> = None;
    let mut receipts = vec![Value::Null; readiness.len()];
    while !running.is_empty() {
        tokio::select! {
            biased;
            _ = signals.wait(), if !stopping => {
                stopping = true;
                stop.send_replace(true);
            }
            completed = running.next() => {
                if let Some((index, outcome)) = completed {
                    match outcome {
                        Ok(receipt) => receipts[index] = json!({"instance": index, "receipt": receipt}),
                        Err(mut error) => {
                            error.notes.push(format!("shared service instance {index} failed"));
                            if let Some(primary) = &mut failure {
                                primary.notes.push(format!("instance {index}: {}", error.code));
                                primary.notes.extend(error.notes);
                            } else {
                                failure = Some(error);
                            }
                            // A service infrastructure failure closes the group; ordinary
                            // handled request failures remain local to their own service.
                            stopping = true;
                            stop.send_replace(true);
                        }
                    }
                }
            }
        }
    }
    // Never early-return or drop unjoined service futures on a member's failure.
    if let Some(error) = failure {
        return Err(error);
    }
    write_json(&json!({
        "ok": true, "event": "stopped", "process_id": std::process::id(),
        "shared_runtime": observation, "instances": receipts,
    }))
}

async fn bind_all(
    set: &PreparedServiceSet,
    signals: &mut ProcessTermination,
) -> Result<Vec<(Service, TcpListener, Value)>, Diagnostic> {
    let mut bound = Vec::with_capacity(set.deployments().len());
    for (index, prepared) in set.deployments().iter().enumerate() {
        let service = Service::new(prepared)?;
        let address = prepared
            .listen()
            .ok_or_else(|| cli_error("service listen address is missing"))?;
        let listener = tokio::select! {
            biased;
            _ = signals.wait() => return Err(cancelled()),
            listener = TcpListener::bind(address) => listener.map_err(|source| infrastructure(
                "serve_bind", format!("instance {index} listener could not bind: {source}")
            ))?,
        };
        let address = listener.local_addr().map_err(|source| {
            infrastructure(
                "serve_address",
                format!("instance {index} listener address is unavailable: {source}"),
            )
        })?;
        let ready = json!({
            "instance": index, "local_address": address.to_string(),
            "deployment": prepared.observe_redacted(),
        });
        bound.push((service, listener, ready));
    }
    Ok(bound)
}

fn cancelled() -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Cancelled,
        "shared_serve_cancelled",
        "shared service admission was cancelled; no application invocation was started",
    )
}

fn infrastructure(code: &'static str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Infrastructure, code, message)
}
