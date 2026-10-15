//! The source owner uses the public HTTP owner without manufacturing a child.

use super::*;
use lkjscript::platform::deployment::PreparedHttpApplication;
use lkjscript::platform::http::HttpServerReceipt;
use lkjscript::platform::runtime::structured::StructuredExecutor;
use lkjscript::platform::{Diagnostic, DiagnosticClass};

pub(super) struct Server {
    pub(super) address: std::net::SocketAddr,
    stop: tokio::sync::oneshot::Sender<()>,
    owner: tokio::task::JoinHandle<Result<HttpServerReceipt, Diagnostic>>,
    application: PreparedHttpApplication,
    executor: StructuredExecutor,
}

impl Server {
    pub(super) fn start(descriptor: &Path) -> Result<Self, DevError> {
        let runtime = semantic_cli::runtime()?;
        // Bind before preparing any application or executor owner.
        let listener = runtime.block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))?;
        let address = listener.local_addr()?;
        let mut prepared = lkjscript::platform::deployment::PreparedDeployment::load(
            descriptor,
            runtime.handle().clone(),
        )
        .map_err(|error| DevError::corrupt(error.to_string()))?;
        let Some(mut executor) = prepared.take_executor() else {
            let mut error = Diagnostic::new(
                DiagnosticClass::Corrupt,
                "pure_tail_executor_owner",
                "source HTTP executor owner missing",
            );
            prepared.close_uninvoked(&mut error);
            return Err(DevError::corrupt(error.to_string()));
        };
        let application = match prepared.http_application() {
            Ok(application) => application,
            Err(mut original) => {
                prepared.close_uninvoked(&mut original);
                return match executor.shutdown() {
                    Ok(_) => Err(DevError::corrupt(original.to_string())),
                    Err(cleanup) => Err(DevError::corrupt(format!(
                        "{original}; joined executor cleanup also failed: {cleanup:?}"
                    ))),
                };
            }
        };
        drop(prepared);
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let serving = application.clone();
        // Tokio spawn has no fallible native-thread handoff. This owner retains
        // the application and executor through joined shutdown on every return.
        let owner = runtime.spawn(async move {
            serving
                .serve(listener, async move {
                    let _ = stopped.await;
                })
                .await
        });
        Ok(Self {
            address,
            stop,
            owner,
            application,
            executor,
        })
    }

    pub(super) fn finish(mut self) -> Result<serde_json::Value, DevError> {
        let runtime = semantic_cli::runtime()?;
        self.executor.close_dispatch();
        let _ = self.stop.send(());
        let result = runtime
            .block_on(self.owner)
            .map_err(|error| DevError::corrupt(format!("joined HTTP owner failed: {error}")))
            .and_then(|result| result.map_err(|error| DevError::corrupt(error.to_string())));
        // Also close the retained application if its serving task failed before
        // reaching the public server's own joined shutdown.
        let shutdown = runtime.block_on(self.application.shutdown());
        let joined = self
            .executor
            .shutdown()
            .map_err(|error| DevError::corrupt(format!("{error:?}")));
        let shutdown_clean = shutdown.admission_stopped
            && shutdown.remaining_tasks == 0
            && shutdown.cleanup_failures.is_empty();
        let receipt = match (result, joined) {
            (Ok(receipt), Ok(executor)) if shutdown_clean => {
                require(
                    receipt.shutdown.admission_stopped
                        && receipt.shutdown.remaining_tasks == 0
                        && receipt.shutdown.cleanup_failures.is_empty()
                        && receipt.runtime.resident.active == 0
                        && receipt.runtime.resident.queued == 0
                        && receipt.runtime.admission_permits == 0
                        && receipt.runtime.worker_permits == 0
                        && executor.active_dispatches == 0
                        && executor.remaining_workers == 0
                        && !executor.dispatch_open,
                    "linked HTTP owner retained tasks, permits, or executor workers",
                )?;
                serde_json::json!({"server":receipt,"retained_application_shutdown":shutdown,"executor":executor})
            }
            (original, executor) => {
                return Err(DevError::corrupt(format!(
                    "HTTP result: {original:?}; retained application shutdown: {shutdown:?}; joined executor: {executor:?}"
                )));
            }
        };
        Ok(receipt)
    }
}
