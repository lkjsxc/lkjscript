//! A bounded detached semantic matrix. Only admitted immutable code crosses workers.

use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

const MAXIMUM_WORKERS: usize = 4;
const MAXIMUM_CANDIDATE_ROWS: usize = 16;

pub struct Case {
    pub name: String,
    pub descriptor: PathBuf,
    pub arguments: PathBuf,
    pub expected: Value,
}

struct PreparedCase {
    name: String,
    target: String,
    arguments: Arc<[u8]>,
    expected: Value,
    policy: RunPolicy,
    artifact: PathBuf,
    descriptor: PathBuf,
    descriptor_value: Value,
}

struct HostCase {
    case: PreparedCase,
    tool: Arc<lkjscript::platform::native_tool::PureTool>,
}

struct CandidateJob {
    descriptor: PathBuf,
    cases: Vec<(usize, batch::BatchCase)>,
}

type CandidateRows = (PathBuf, Value, Vec<(usize, batch::BatchCase)>);

struct HostOutput {
    stdout: String,
    result_json: Arc<[u8]>,
}

struct Joined<T> {
    rows: Vec<Option<T>>,
    failure: Option<Box<dyn std::any::Any + Send>>,
    workers: usize,
}

impl<T> Joined<T> {
    fn finish(self) -> Vec<T> {
        if let Some(error) = self.failure {
            resume_unwind(error);
        }
        self.rows
            .into_iter()
            .map(|row| row.unwrap_or_else(|| panic!("joined matrix lost an inventoried row")))
            .collect()
    }
}

fn workers(jobs: usize) -> usize {
    std::thread::available_parallelism()
        .map_or(1, usize::from)
        .min(MAXIMUM_WORKERS)
        .min(jobs)
}

/// Every worker is explicitly joined before any captured failure is propagated.
/// No Native bookkeeping or mutable invocation owner is shared across threads.
fn scoped_map<J: Sync, T: Send>(jobs: &[J], execute: impl Fn(usize, &J) -> T + Sync) -> Joined<T> {
    scoped_map_with_workers(jobs, workers(jobs.len()), execute)
}

fn scoped_map_with_workers<J: Sync, T: Send>(
    jobs: &[J],
    worker_count: usize,
    execute: impl Fn(usize, &J) -> T + Sync,
) -> Joined<T> {
    assert!(!jobs.is_empty());
    assert!(worker_count > 0 && worker_count <= MAXIMUM_WORKERS && worker_count <= jobs.len());
    let next = AtomicUsize::new(0);
    let stopped = AtomicBool::new(false);
    let mut rows = (0..jobs.len()).map(|_| None).collect::<Vec<_>>();
    let mut failure = None;
    let started_workers = std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for worker in 0..worker_count {
            let execute = &execute;
            let next = &next;
            let stopped = &stopped;
            match std::thread::Builder::new()
                .name(format!("native-matrix-{worker}"))
                .spawn_scoped(scope, move || {
                    let mut completed = Vec::new();
                    while !stopped.load(Ordering::Acquire) {
                        let index = next.fetch_add(1, Ordering::Relaxed);
                        let Some(job) = jobs.get(index) else { break };
                        let outcome = catch_unwind(AssertUnwindSafe(|| execute(index, job)));
                        if outcome.is_err() {
                            stopped.store(true, Ordering::Release);
                        }
                        completed.push((index, outcome));
                    }
                    completed
                }) {
                Ok(handle) => handles.push(handle),
                Err(error) => {
                    stopped.store(true, Ordering::Release);
                    failure = Some(
                        Box::new(format!("cannot start native matrix worker: {error}"))
                            as Box<dyn std::any::Any + Send>,
                    );
                    break;
                }
            }
        }
        let started_workers = handles.len();
        for handle in handles {
            match handle.join() {
                Ok(completed) => {
                    for (index, outcome) in completed {
                        match outcome {
                            Ok(row) => rows[index] = Some(row),
                            Err(error) if failure.is_none() => failure = Some(error),
                            Err(_) => {}
                        }
                    }
                }
                Err(error) if failure.is_none() => failure = Some(error),
                Err(_) => {}
            }
        }
        started_workers
    });
    Joined {
        rows,
        failure,
        workers: started_workers,
    }
}

impl Native {
    /// Preserve a serial ordinary executable boundary after source removal.
    pub fn matrix_boundary(&self, name: &str, case: &Case) {
        assert!(self.detached.get());
        if self.mode == semantic_cli::Mode::HostSemantic {
            self.detached_boundary(&case.descriptor, &case.arguments, &case.expected);
        } else {
            self.compare(name, &case.descriptor, &case.arguments, &case.expected);
        }
    }

    pub fn matrix(&self, label: &str, cases: &[Case]) -> Vec<String> {
        assert!(
            self.detached.get(),
            "full matrices require the detached barrier"
        );
        assert!(!cases.is_empty());
        let mut names = BTreeSet::new();
        let prepared =
            cases
                .iter()
                .map(|case| {
                    assert!(!case.name.is_empty() && case.name.len() <= 128);
                    assert!(
                        case.name.bytes().all(|byte| byte.is_ascii_alphanumeric()
                            || matches!(byte, b'.' | b'_' | b'-'))
                    );
                    assert!(
                        names.insert(case.name.as_str()),
                        "duplicate matrix identity"
                    );
                    for suffix in ["stdout", "result.json", "actual.json", "failure.stdout"] {
                        let output = self.root.join(format!("matrix-{}.{suffix}", case.name));
                        assert!(
                            !output.exists() && !self.memory.borrow().contains_key(&output),
                            "matrix evidence must be newly owned"
                        );
                    }
                    let bytes = self.bytes(&case.descriptor);
                    let descriptor = decode_deployment(&bytes).unwrap();
                    assert!(descriptor.grants.is_empty());
                    assert!(descriptor.configuration.is_empty() && descriptor.secrets.is_empty());
                    assert!(descriptor.runtime.is_none() && descriptor.listen.is_none());
                    assert!(
                        descriptor.http.is_none()
                            && descriptor.session.is_none()
                            && descriptor.worker.is_none()
                    );
                    PreparedCase {
                        name: case.name.clone(),
                        target: descriptor.target,
                        policy: descriptor.execution.unwrap_or_else(RunPolicy::unmetered),
                        artifact: case.descriptor.parent().unwrap().join(descriptor.artifact),
                        descriptor: case.descriptor.clone(),
                        descriptor_value: serde_json::from_slice(&bytes).unwrap(),
                        arguments: self.bytes(&case.arguments),
                        expected: case.expected.clone(),
                    }
                })
                .collect::<Vec<_>>();
        for case in &prepared {
            assert!(case.arguments.len() <= JsonLimits::default().maximum_bytes);
        }
        let mut artifact_hashes = BTreeMap::new();
        for case in &prepared {
            artifact_hashes
                .entry(case.artifact.clone())
                .or_insert_with(|| digest(&case.artifact));
        }
        let executable_hash = digest(&self.root.join("lkjscript"));
        let (outputs, jobs, worker_count): (Vec<String>, usize, usize) = if self.mode
            == semantic_cli::Mode::HostSemantic
        {
            let host = prepared
                .into_iter()
                .map(|case| {
                    // This replaces the pre-detachment binding with a genuinely new strict
                    // admission. Every full matrix invocation uses that newly admitted Arc.
                    let tool = self
                        .artifacts
                        .fresh_after_detachment(&case.artifact)
                        .unwrap();
                    HostCase { case, tool }
                })
                .collect::<Vec<_>>();
            let root = &self.root;
            let outputs = scoped_map(&host, |_, job| {
                let outcome = job.tool.run_json_observed(
                    &job.case.target,
                    &job.case.arguments,
                    JsonLimits::default(),
                    job.case.policy,
                    &ExecutionControl::uncancelled(),
                );
                match outcome {
                    Ok(receipt) => {
                        let output = observed_output(&receipt);
                        let checked = catch_unwind(AssertUnwindSafe(|| {
                            let result: Value =
                                serde_json::from_slice(&receipt.result_json).unwrap();
                            assert_eq!(
                                result, job.case.expected,
                                "{}: full independent matrix result",
                                job.case.name
                            );
                            require_clean(&output, &job.case.name);
                        }));
                        if let Err(error) = checked {
                            fs::write(
                                root.join(format!("matrix-{}.actual.json", job.case.name)),
                                &receipt.result_json,
                            )
                            .unwrap();
                            fs::write(
                                root.join(format!("matrix-{}.stdout", job.case.name)),
                                &output,
                            )
                            .unwrap();
                            resume_unwind(error);
                        }
                        HostOutput {
                            stdout: output,
                            result_json: receipt.result_json.into(),
                        }
                    }
                    Err(error) => {
                        let response = semantic_cli::failure("run", &[error]);
                        let file = root.join(format!("matrix-{}.failure.stdout", job.case.name));
                        fs::write(&file, response.stdout).unwrap();
                        panic!(
                            "{}: actual matrix invocation failed; {}",
                            job.case.name,
                            file.display()
                        );
                    }
                }
            });
            // Keep every completed actual row, even if a different worker failed. Only
            // after retention and all joins may the captured panic leave this owner.
            for (job, output) in host.iter().zip(&outputs.rows) {
                if let Some(output) = output {
                    self.memory.borrow_mut().insert(
                        self.root
                            .join(format!("matrix-{}.result.json", job.case.name)),
                        Arc::clone(&output.result_json),
                    );
                    self.memory.borrow_mut().insert(
                        self.root.join(format!("matrix-{}.stdout", job.case.name)),
                        output.stdout.as_bytes().into(),
                    );
                }
            }
            let worker_count = outputs.workers;
            let outputs = outputs
                .finish()
                .into_iter()
                .map(|output| output.stdout)
                .collect();
            (outputs, host.len(), worker_count)
        } else {
            let mut artifacts: BTreeMap<PathBuf, CandidateRows> = BTreeMap::new();
            for (index, case) in prepared.into_iter().enumerate() {
                let mut context = case.descriptor_value;
                context.as_object_mut().unwrap().remove("target");
                context.as_object_mut().unwrap().remove("execution");
                let mut row = batch::BatchCase::success(
                    &case.name,
                    &serde_json::from_slice::<Value>(&case.arguments).unwrap(),
                    case.expected,
                );
                // Preserve the exact original argument bytes and per-row execution policy.
                row.arguments_json = String::from_utf8(case.arguments.to_vec()).unwrap();
                row.target = Some(case.target);
                row.execution = Some(serde_json::to_value(case.policy).unwrap());
                let entry = artifacts
                    .entry(case.artifact)
                    .or_insert_with(|| (case.descriptor, context.clone(), Vec::new()));
                assert_eq!(
                    entry.1, context,
                    "candidate rows must share all deployment context except target/policy"
                );
                entry.2.push((index, row));
            }
            let mut jobs = Vec::new();
            for (_, (descriptor, _, rows)) in artifacts {
                let mut rows = rows.into_iter();
                loop {
                    let cases = rows
                        .by_ref()
                        .take(MAXIMUM_CANDIDATE_ROWS)
                        .collect::<Vec<_>>();
                    if cases.is_empty() {
                        break;
                    }
                    jobs.push(CandidateJob {
                        descriptor: descriptor.clone(),
                        cases,
                    });
                }
            }
            let executable = self.root.join("lkjscript");
            let root = &self.root;
            let chunks = scoped_map(&jobs, |index, job| {
                let rows = job.cases.iter().map(|(_, row)| row).collect::<Vec<_>>();
                // Existing adapter owns the real selected process through terminal, strict
                // per-row result/target/cleanup checks and immutable byte bindings.
                batch::execute_refs(
                    root,
                    &executable,
                    &format!("{label}-{index}"),
                    &job.descriptor,
                    &rows,
                )
            });
            let worker_count = chunks.workers;
            let chunks = chunks.finish();
            let mut outputs = (0..cases.len()).map(|_| None).collect::<Vec<_>>();
            for (job, chunk) in jobs.iter().zip(chunks) {
                assert_eq!(job.cases.len(), chunk.len());
                for ((index, _), output) in job.cases.iter().zip(chunk) {
                    outputs[*index] = Some(output);
                }
            }
            (
                outputs.into_iter().map(|output| output.unwrap()).collect(),
                jobs.len(),
                worker_count,
            )
        };
        for (case, output) in cases.iter().zip(&outputs) {
            self.memory.borrow_mut().insert(
                self.root.join(format!("matrix-{}.stdout", case.name)),
                output.as_bytes().into(),
            );
        }
        for (artifact, expected) in artifact_hashes {
            assert_eq!(digest(&artifact), expected);
        }
        assert_eq!(digest(&self.root.join("lkjscript")), executable_hash);
        self.matrix_evidence.borrow_mut().push(serde_json::json!({
            "label":label, "mode":self.mode.label(), "rows":cases.len(),
            "worker_threads":worker_count, "worker_ceiling":MAXIMUM_WORKERS,
            "candidate_batch_processes":if self.mode == semantic_cli::Mode::HostSemantic { 0 } else { jobs },
            "maximum_candidate_rows_per_process":MAXIMUM_CANDIDATE_ROWS,
            "retention_order":"original inventory", "joined":true,
        }));
        outputs
    }
}

fn require_clean(output: &str, name: &str) {
    let observation = quoted(output, "production-observation");
    for field in [
        "live_handles_after",
        "live_locals_after",
        "live_operands_after",
        "live_call_frames_after",
        "live_transactions_after",
        "live_type_bindings_after",
    ] {
        assert_eq!(observation[field], 0, "{name}: {field}");
    }
    let executor = quoted(output, "executor-observation");
    assert_eq!(executor["remaining_workers"], 0, "{name}");
    assert_eq!(executor["active_dispatches"], 0, "{name}");
    assert_eq!(executor["dispatch_open"], false, "{name}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Barrier, Mutex, mpsc};

    #[test]
    fn scoped_rows_keep_original_order_when_completion_order_differs() {
        let (sent, received) = mpsc::channel();
        let received = Mutex::new(received);
        let completed = Mutex::new(Vec::new());
        let rows = scoped_map_with_workers(&[0, 1, 2, 3], 2, |index, _| {
            if index == 0 {
                received.lock().unwrap().recv().unwrap();
            }
            completed.lock().unwrap().push(index);
            if index == 1 {
                sent.send(()).unwrap();
            }
            index
        });
        assert_eq!(completed.lock().unwrap()[0], 1);
        assert_eq!(rows.workers, 2);
        assert_eq!(rows.finish(), vec![0, 1, 2, 3]);
    }

    #[test]
    fn failed_scope_joins_live_owners_and_retains_completed_rows_before_propagation() {
        struct Owner<'a>(&'a AtomicUsize);
        impl Drop for Owner<'_> {
            fn drop(&mut self) {
                self.0.fetch_sub(1, Ordering::SeqCst);
            }
        }
        let live = AtomicUsize::new(0);
        let entered = Barrier::new(2);
        let (sent, received) = mpsc::channel();
        let received = Mutex::new(received);
        let rows = scoped_map_with_workers(&[0, 1], 2, |index, _| {
            live.fetch_add(1, Ordering::SeqCst);
            let _owner = Owner(&live);
            entered.wait();
            if index == 1 {
                sent.send(()).unwrap();
                panic!("deliberate worker failure while another row owns live state");
            }
            received.lock().unwrap().recv().unwrap();
            index
        });
        assert_eq!(live.load(Ordering::SeqCst), 0);
        assert_eq!(rows.rows, vec![Some(0), None]);
        assert!(rows.failure.is_some());
        assert!(catch_unwind(AssertUnwindSafe(|| rows.finish())).is_err());
    }
}
