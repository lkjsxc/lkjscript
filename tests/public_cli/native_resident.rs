//! Source semantic residents own real listeners, applications and joined workers.
//! These observations are test-owned API observations, never child-process receipts.
use futures_util::{FutureExt, StreamExt, stream::FuturesUnordered};
use lkjscript::platform::deployment::shared::PreparedServiceSet;
use lkjscript::platform::deployment::{
    PreparedHttpApplication, PreparedSessionApplication, PreparedWorkerApplication,
};
use lkjscript::platform::execution::ExecutionControl;
use lkjscript::platform::runtime::structured::StructuredExecutor;
use lkjscript::platform::{PreparedDeployment, decode_deployment};
use serde_json::{Value, json};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio::sync::watch;

type Outcome = Result<Value, String>;

/// Existing public preparation resolves secrets from the process environment.
/// Such cases retain their closed executable environment instead of changing it.
pub(in super::super) fn eligible(paths: &[PathBuf], environment: &[(&str, &str)]) -> bool {
    environment.is_empty()
        && paths.iter().all(|path| {
            decode_deployment(&std::fs::read(path).unwrap())
                .unwrap()
                .secrets
                .is_empty()
        })
}

pub(in super::super) struct Resident {
    pub ready: Value,
    stop: Option<watch::Sender<bool>>,
    control: ExecutionControl,
    owner: Option<JoinHandle<Outcome>>,
    shutdown_timeout: Duration,
}

impl Resident {
    pub fn start(
        paths: Vec<PathBuf>,
        shared: bool,
        output: PathBuf,
        errors: PathBuf,
        readiness_timeout: Duration,
        shutdown_timeout: Duration,
    ) -> Self {
        assert!(!paths.is_empty());
        assert!(shared || paths.len() == 1);
        assert!(
            eligible(&paths, &[]),
            "secret-bound residents require a process"
        );
        File::create(&output).unwrap();
        File::create(errors).unwrap();
        let control = ExecutionControl::uncancelled();
        let preparing = control.clone();
        let (stop, stopping) = watch::channel(false);
        let (announce, readiness) = mpsc::sync_channel(1);
        let owner = std::thread::Builder::new()
            .name("native-semantic-resident".into())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build()
                    .map_err(|error| error.to_string())?;
                let outcome = runtime.block_on(async {
                    if shared {
                        shared_group(paths, preparing, stopping, &announce, &output).await
                    } else {
                        single(&paths[0], preparing, stopping, &announce, &output).await
                    }
                });
                if let Err(error) = &outcome {
                    let _ = announce.try_send(Err(error.clone()));
                }
                outcome
            })
            .unwrap();
        let mut resident = Self {
            ready: Value::Null,
            stop: Some(stop),
            control,
            owner: Some(owner),
            shutdown_timeout,
        };
        let ready = readiness
            .recv_timeout(readiness_timeout)
            .map_err(|error| error.to_string())
            .and_then(|value| value);
        if ready.is_err() {
            let joined = resident.finish();
            assert!(
                ready.is_ok(),
                "resident readiness: {ready:?}; joined={joined:?}"
            );
        }
        resident.ready = ready.unwrap();
        assert_eq!(resident.ready["execution_model"], "host_semantic");
        assert!(resident.ready.get("process_id").is_none());
        resident
    }

    fn finish(&mut self) -> Outcome {
        self.control.cancel();
        if let Some(stop) = self.stop.take() {
            stop.send_replace(true);
        }
        match self.owner.take() {
            Some(owner) => {
                let until = Instant::now() + self.shutdown_timeout;
                while !owner.is_finished() && Instant::now() < until {
                    std::thread::sleep(Duration::from_millis(5));
                }
                let within_deadline = owner.is_finished();
                // Deadline failure never detaches a still-owned resource owner.
                let outcome = owner.join().map_err(|_| {
                    "semantic resident owner panicked after owned cleanup".to_owned()
                })?;
                if !within_deadline {
                    return Err(format!(
                        "semantic resident exceeded {:?}; actual joined outcome={outcome:?}",
                        self.shutdown_timeout
                    ));
                }
                let value = outcome?;
                validate_terminal(&value)?;
                Ok(value)
            }
            None => Err("semantic resident owner was already joined".into()),
        }
    }

    pub fn stop(mut self) -> Value {
        let stopped = self.finish().unwrap();
        assert_eq!(stopped["execution_model"], "host_semantic");
        assert!(stopped.get("process_id").is_none());
        stopped
    }
}

impl Drop for Resident {
    fn drop(&mut self) {
        // A client assertion may unwind without calling stop. The API shutdown
        // future and native worker owner still finish before the fixture disappears.
        if self.owner.is_some() {
            let outcome = self.finish();
            if std::thread::panicking() {
                if let Err(error) = outcome {
                    eprintln!("joined semantic resident cleanup failed: {error}");
                }
            } else {
                assert!(
                    outcome.is_ok(),
                    "joined semantic resident cleanup: {outcome:?}"
                );
            }
        }
    }
}

pub(in super::super) fn check_executor(executor: &Value) -> Result<(), String> {
    let counter = |name: &str| {
        executor[name]
            .as_u64()
            .ok_or_else(|| format!("actual executor counter {name} is absent: {executor}"))
    };
    let acquired = counter("workers_started")?
        .checked_add(counter("workers_received")?)
        .ok_or_else(|| "actual executor custody accounting overflowed".to_owned())?;
    let released = counter("joined_workers")?
        .checked_add(counter("workers_handed_off")?)
        .ok_or_else(|| "actual executor custody accounting overflowed".to_owned())?;
    if executor["dispatch_open"] != false
        || executor["active_dispatches"] != 0
        || executor["remaining_workers"] != 0
        || acquired != released
    {
        return Err(format!(
            "actual executor did not close every acquired worker custody: {executor}"
        ));
    }
    Ok(())
}

fn validate_terminal(value: &Value) -> Result<(), String> {
    let shared = value.get("shared_runtime");
    let executor = shared.map_or(&value["executor"], |runtime| &runtime["executor"]);
    check_executor(executor)?;
    let receipts: Vec<_> = match shared {
        Some(_) => value["instances"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| &item["receipt"])
            .collect(),
        None => vec![&value["receipt"]],
    };
    for receipt in receipts {
        if receipt["shutdown"]["admission_stopped"] != true
            || receipt["shutdown"]["remaining_tasks"] != 0
            || receipt["shutdown"]["cleanup_failures"] != json!([])
            || (!receipt["runtime"].is_null()
                && (receipt["runtime"]["admission_permits"] != 0
                    || receipt["runtime"]["worker_permits"] != 0
                    || receipt["runtime"]["resident"]["active"] != 0
                    || receipt["runtime"]["resident"]["queued"] != 0))
            || (!receipt["sessions"].is_null()
                && (receipt["sessions"]["active_sessions"] != 0
                    || receipt["sessions"]["pending_handshakes"] != 0))
        {
            return Err(format!("actual resident did not quiesce: {value}"));
        }
    }
    Ok(())
}

#[derive(Clone)]
enum Application {
    Http(Box<PreparedHttpApplication>),
    Session(Box<PreparedSessionApplication>),
    Worker(Box<PreparedWorkerApplication>),
}

impl Application {
    fn new(prepared: &PreparedDeployment) -> Result<Self, String> {
        match prepared.observe_redacted().runner.as_str() {
            "http" => prepared.http_application().map(Box::new).map(Self::Http),
            "interactive" => prepared
                .session_application()
                .map(Box::new)
                .map(Self::Session),
            "worker" => prepared
                .worker_application()
                .map(Box::new)
                .map(Self::Worker),
            other => return Err(format!("unsupported semantic resident runner: {other}")),
        }
        .map_err(|error| format!("{error:?}"))
    }

    async fn shutdown(&self) -> Value {
        match self {
            Self::Http(application) => serde_json::to_value(application.shutdown().await),
            Self::Session(application) => serde_json::to_value(application.shutdown().await),
            Self::Worker(application) => serde_json::to_value(application.shutdown().await),
        }
        .unwrap()
    }

    async fn serve(
        self,
        listener: Option<TcpListener>,
        mut stop: watch::Receiver<bool>,
    ) -> Outcome {
        let cleanup = self.clone();
        let shutdown = async move {
            let _ = stop.wait_for(|stopped| *stopped).await;
        };
        let operation = async move {
            let receipt = match self {
                Self::Http(application) => serde_json::to_value(
                    application
                        .serve(listener.unwrap(), shutdown)
                        .await
                        .map_err(|error| format!("{error:?}"))?,
                ),
                Self::Session(application) => serde_json::to_value(
                    application
                        .serve(listener.unwrap(), shutdown)
                        .await
                        .map_err(|error| format!("{error:?}"))?,
                ),
                Self::Worker(application) => serde_json::to_value(
                    application
                        .run(shutdown)
                        .await
                        .map_err(|error| format!("{error:?}"))?,
                ),
            };
            receipt.map_err(|error| error.to_string())
        };
        match AssertUnwindSafe(operation).catch_unwind().await {
            Ok(Ok(receipt)) => Ok(receipt),
            Ok(Err(error)) => Err(format!(
                "{error}; actual joined shutdown={}",
                cleanup.shutdown().await
            )),
            Err(_) => Err(format!(
                "resident future panicked; actual joined shutdown={}",
                cleanup.shutdown().await
            )),
        }
    }
}

fn emit(path: &Path, value: &Value) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .append(true)
        .open(path)
        .map_err(|error| error.to_string())?;
    let bytes = serde_json::to_vec(value).map_err(|error| error.to_string())?;
    let current = file.metadata().map_err(|error| error.to_string())?.len();
    if current.saturating_add(bytes.len() as u64 + 1) > 2 * 1024 * 1024 {
        return Err("bounded semantic resident observation output exceeded".into());
    }
    file.write_all(&bytes)
        .and_then(|()| file.write_all(b"\n"))
        .map_err(|error| error.to_string())
}

fn join_executor(executor: &mut StructuredExecutor, outcome: Outcome) -> Outcome {
    executor.close_dispatch();
    match executor.shutdown() {
        Ok(observed) => outcome.map(|mut value| {
            value["executor"] = serde_json::to_value(observed).unwrap();
            value
        }),
        Err(error) => Err(format!(
            "{outcome:?}; actual structured worker cleanup failed: {error:?}"
        )),
    }
}

fn close_prepared(mut prepared: PreparedDeployment, message: String) -> String {
    let mut error = lkjscript::platform::Diagnostic::new(
        lkjscript::platform::DiagnosticClass::Infrastructure,
        "semantic_resident_preparation",
        message,
    );
    prepared.close_uninvoked(&mut error);
    if let Some(mut executor) = prepared.take_executor() {
        return format!(
            "{error:?}; {:?}",
            join_executor(&mut executor, Err(format!("{error:?}")))
        );
    }
    format!("{error:?}")
}

async fn single(
    path: &Path,
    control: ExecutionControl,
    mut stop: watch::Receiver<bool>,
    announce: &mpsc::SyncSender<Outcome>,
    output: &Path,
) -> Outcome {
    let mut prepared = PreparedDeployment::load(path, tokio::runtime::Handle::current())
        .map_err(|error| format!("{error:?}"))?;
    if control.check().is_err() {
        return Err(close_prepared(
            prepared,
            "semantic resident preparation cancelled".into(),
        ));
    }
    let application = match Application::new(&prepared) {
        Ok(application) => application,
        Err(error) => return Err(close_prepared(prepared, error)),
    };
    let mut executor = prepared.take_executor().unwrap();
    let listener = if matches!(&application, Application::Worker(_)) {
        None
    } else {
        match TcpListener::bind(prepared.listen().unwrap()).await {
            Ok(listener) => Some(listener),
            Err(error) => {
                executor.close_dispatch();
                let actual = application.shutdown().await;
                return join_executor(
                    &mut executor,
                    Err(format!("listener bind: {error}; shutdown={actual}")),
                );
            }
        }
    };
    let ready = json!({
        "event":"ready", "execution_model":"host_semantic",
        "deployment":prepared.observe_redacted(), "executor":executor.observe(),
        "local_address":listener.as_ref().map(|listener| listener.local_addr().unwrap().to_string()),
    });
    drop(prepared);
    if let Err(error) = emit(output, &ready)
        .and_then(|()| announce.send(Ok(ready)).map_err(|error| error.to_string()))
    {
        executor.close_dispatch();
        let actual = application.shutdown().await;
        return join_executor(
            &mut executor,
            Err(format!("readiness: {error}; shutdown={actual}")),
        );
    }
    let cleanup = application.clone();
    let (local_stop, stopping) = watch::channel(false);
    let operation = application.serve(listener, stopping);
    tokio::pin!(operation);
    let outcome = tokio::select! {
        biased;
        _ = stop.wait_for(|stopped| *stopped) => {
            executor.close_dispatch();
            local_stop.send_replace(true);
            operation.await
        }
        outcome = &mut operation => outcome,
    };
    let stopped = join_executor(
        &mut executor,
        outcome.map(|receipt| {
            json!({
                "event":"stopped", "execution_model":"host_semantic", "receipt":receipt,
            })
        }),
    );
    // Keep the actual application alive until the worker owner has joined.
    drop(cleanup);
    let stopped = stopped?;
    emit(output, &stopped)?;
    Ok(stopped)
}

async fn shared_group(
    paths: Vec<PathBuf>,
    control: ExecutionControl,
    mut stop: watch::Receiver<bool>,
    announce: &mpsc::SyncSender<Outcome>,
    output: &Path,
) -> Outcome {
    let mut set = PreparedServiceSet::load(&paths, tokio::runtime::Handle::current(), &control)
        .map_err(|error| format!("{error:?}"))?;
    let mut observation = set.observe();
    let mut bound = Vec::new();
    let mut readiness = Vec::new();
    let preparation: Result<(), String> = async {
        for (index, prepared) in set.deployments().iter().enumerate() {
            let application = Application::new(prepared)?;
            let listener = TcpListener::bind(prepared.listen().unwrap()).await.map_err(|error| error.to_string())?;
            readiness.push(json!({"instance":index,"local_address":listener.local_addr().unwrap().to_string(),"deployment":prepared.observe_redacted()}));
            bound.push((application, listener));
        }
        Ok(())
    }.await;
    if let Err(message) = preparation {
        let mut error = lkjscript::platform::Diagnostic::new(
            lkjscript::platform::DiagnosticClass::Infrastructure,
            "semantic_resident_preparation",
            message,
        );
        set.close_uninvoked(&mut error);
        return Err(format!("{error:?}"));
    }
    let ready = json!({"event":"ready","execution_model":"host_semantic","shared_runtime":observation,"instances":readiness});
    if let Err(message) = emit(output, &ready)
        .and_then(|()| announce.send(Ok(ready)).map_err(|error| error.to_string()))
    {
        let mut error = lkjscript::platform::Diagnostic::new(
            lkjscript::platform::DiagnosticClass::Infrastructure,
            "semantic_resident_readiness",
            message,
        );
        set.close_uninvoked(&mut error);
        return Err(format!("{error:?}"));
    }
    let (deployments, mut executor) = set.into_parts();
    drop(deployments);
    let (local_stop, stopping) = watch::channel(false);
    let mut running = FuturesUnordered::new();
    for (index, (application, listener)) in bound.into_iter().enumerate() {
        let stopping = stopping.clone();
        running.push(async move { (index, application.serve(Some(listener), stopping).await) });
    }
    drop(stopping);
    let mut stopping = false;
    let mut failures = Vec::new();
    let mut receipts = vec![Value::Null; readiness.len()];
    while !running.is_empty() {
        tokio::select! {
            biased;
            _ = stop.wait_for(|stopped| *stopped), if !stopping => {
                stopping = true;
                executor.close_dispatch();
                local_stop.send_replace(true);
            }
            completed = running.next() => {
                if let Some((index, outcome)) = completed {
                    match outcome {
                        Ok(receipt) => receipts[index]=json!({"instance":index,"receipt":receipt}),
                        Err(error) => {
                            failures.push(format!("instance {index}: {error}"));
                            stopping = true;
                            executor.close_dispatch();
                            local_stop.send_replace(true);
                        }
                    }
                }
            }
        }
    }
    executor.close_dispatch();
    match executor.shutdown() {
        Ok(joined) => observation.executor = joined,
        Err(error) => failures.push(format!("actual worker cleanup: {error:?}")),
    }
    if !failures.is_empty() {
        return Err(format!(
            "{failures:?}; actual executor={:?}",
            executor.observe()
        ));
    }
    let stopped = json!({"event":"stopped","execution_model":"host_semantic","shared_runtime":observation,"instances":receipts});
    emit(output, &stopped)?;
    Ok(stopped)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_resident_client_unwind_joins_listener_and_workers() {
        let (public, descriptor) = super::super::super::resident_termination::author("http");
        let deployment = public.input("unwind.json", &descriptor.to_string());
        let output = public.root.path().join("unwind.observations.jsonl");
        let errors = public.root.path().join("unwind.stderr");
        let resident = Resident::start(
            vec![deployment],
            false,
            output.clone(),
            errors.clone(),
            Duration::from_secs(30),
            Duration::from_secs(15),
        );
        let address = resident.ready["local_address"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(
            super::super::send(address, "GET", "/health", &[("Host", "localhost")], "").body,
            "7"
        );
        let failure = std::panic::catch_unwind(AssertUnwindSafe(move || {
            let _owned_until_client_failure = resident;
            std::panic::resume_unwind(Box::new("independent fixture client failed"));
        }));
        assert!(failure.is_err());
        let bytes = std::fs::read(&output).unwrap();
        assert!(bytes.len() <= 2 * 1024 * 1024);
        let events: Vec<Value> = bytes
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
            .map(|line| serde_json::from_slice(line).unwrap())
            .collect();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0]["event"], "ready");
        assert_eq!(events[1]["event"], "stopped");
        validate_terminal(&events[1]).unwrap();
        assert!(std::fs::read(errors).unwrap().is_empty());
        assert!(
            std::net::TcpStream::connect_timeout(&address, Duration::from_millis(100)).is_err(),
            "the listener must close before the client's unwind escapes"
        );
    }
}
