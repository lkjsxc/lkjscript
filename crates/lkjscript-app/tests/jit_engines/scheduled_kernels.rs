use crate::canonical::{compile, f64_loop};
use lkjscript_core::{ExecutionConfig, ExecutionOutcome};
use lkjscript_ir::VerifiedProgram;
use lkjscript_jit::{execute_forced, execute_optimizing, JitConfig, JitExecution};
use lkjscript_linux_host::{discover_linux_host, LinuxWorkerBinder};
use lkjscript_resource::*;

struct KernelExecutor<'a> {
    scalar: &'a VerifiedProgram,
    unique: &'a VerifiedProgram,
}

#[derive(Debug)]
struct KernelEvidence {
    returned: bool,
    native_entries: u64,
    direct_calls: u64,
    vm_fallbacks: u64,
    unique_calls: u64,
    live_owners: u64,
    live_loans: u64,
    release_backlog: u64,
}

impl TaskExecutor for KernelExecutor<'_> {
    type Output = KernelEvidence;
    type Error = String;

    fn execute(&self, task: TaskId, _worker: WorkerId) -> Result<Self::Output, Self::Error> {
        let program = if task.slot < 2 {
            self.scalar
        } else {
            self.unique
        };
        let execution = if task.slot.is_multiple_of(2) {
            execute_forced(program, &ExecutionConfig::default(), JitConfig::default())
        } else {
            execute_optimizing(program, &ExecutionConfig::default(), JitConfig::default())
        }
        .map_err(|error| error.to_string())?;
        Ok(evidence(execution))
    }
}

#[test]
fn scheduled_tasks_execute_actual_collector_free_generated_kernels() {
    let scalar = compile(&f64_loop(), "scheduled-scalar-kernel.lkjscript");
    let unique = compile(unique_source(), "scheduled-unique-kernel.lkjscript");
    let graph = kernel_graph();
    let host = discover_linux_host().expect("discover host topology");
    let plan = ResourcePlanner::plan(
        &host.topology,
        PlacementMode::KernelManaged,
        PlanCaps {
            task_cap: 4,
            request_cap: 2,
            profile_cap: 2,
            queue_bytes_per_worker: 16 * 1024,
            scratch_bytes_per_worker: 1024 * 1024,
        },
        2,
    )
    .expect("plan generated kernels");
    let config = RuntimeConfig {
        workers: plan
            .workers
            .iter()
            .map(|worker| WorkerDescriptor {
                id: worker.worker,
                allowed: worker.exact_mask.clone(),
                group: worker.group,
                numa_node: host
                    .topology
                    .numa_nodes
                    .iter()
                    .find(|node| {
                        worker
                            .exact_mask
                            .as_slice()
                            .iter()
                            .any(|cpu| node.cpus.contains(*cpu))
                    })
                    .map(|node| node.id),
            })
            .collect(),
        queue_capacity: 4,
        spin_limit: 32,
        policy: SchedulePolicy::LocalWorkStealing,
    };
    let executor = KernelExecutor {
        scalar: scalar.ssa(),
        unique: unique.ssa(),
    };
    let binder = LinuxWorkerBinder::for_mode(plan.mode);
    let report =
        ScopedRuntime::run(&graph, config, &executor, &binder).expect("run generated kernel tasks");
    assert!(report.failures.is_empty());
    assert_eq!(report.outputs.len(), 4);
    assert_eq!(report.metrics.executed, 4);
    assert_eq!(report.metrics.active_workers, 0);
    assert_eq!(
        report.metrics.same_group_steals + report.metrics.cross_group_steals,
        report.metrics.steals
    );
    assert!(report.metrics.cross_numa_steals <= report.metrics.cross_group_steals);
    for (_, evidence) in &report.outputs {
        assert!(evidence.returned);
        assert!(evidence.native_entries > 0);
        assert!(evidence.direct_calls > 0);
        assert_eq!(evidence.vm_fallbacks, 0);
        assert_eq!(evidence.live_owners, 0);
        assert_eq!(evidence.live_loans, 0);
        assert_eq!(evidence.release_backlog, 0);
    }
    assert!(report
        .outputs
        .iter()
        .filter(|(task, _)| task.slot >= 2)
        .all(|(_, evidence)| evidence.unique_calls > 0));
}

fn evidence(execution: JitExecution) -> KernelEvidence {
    KernelEvidence {
        returned: matches!(execution.outcome, ExecutionOutcome::Returned(_)),
        native_entries: execution.stats.native_entries,
        direct_calls: execution.stats.direct_native_calls,
        vm_fallbacks: execution.stats.vm_fallbacks,
        unique_calls: execution.stats.unique_runtime_calls,
        live_owners: execution.stats.native_unique.live_owners,
        live_loans: execution.stats.native_unique.live_loans,
        release_backlog: execution.stats.native_unique.release_backlog,
    }
}

fn kernel_graph() -> VerifiedTaskGraph {
    let mut builder = TaskGraphBuilder::new();
    let scope = TaskScopeId::new(0, 1);
    builder
        .add_scope(TaskScope {
            id: scope,
            parent: None,
        })
        .expect("kernel scope");
    for slot in 0..4_u32 {
        let owner = DataOwnerId::new(slot + 1, 1);
        builder
            .add_task(TaskNode {
                id: TaskId::new(slot, 1),
                class: TaskClassId::from_name(if slot < 2 {
                    "reuse-generated-kernel"
                } else {
                    "unique-generated-kernel"
                }),
                scope,
                result: TaskResultId::new(slot, 1),
                result_owner: owner,
                dependencies: Vec::new(),
                accesses: vec![AccessRecord {
                    id: AccessRecordId::new(slot, 1),
                    owner,
                    mode: AccessMode::Produce,
                    range: None,
                }],
                blocking: false,
                portable: true,
                compute_units: if slot == 3 { 8 } else { 1 },
                scratch_bytes: 4096,
                cleanup: true,
            })
            .expect("kernel task");
    }
    TaskGraphVerifier::verify(builder.build(), GraphLimits::default()).expect("verify kernel graph")
}

fn unique_source() -> &'static str {
    concat!(
        "def/\nname/\ntake\n/name\nfn/\nsig/\ninputs/\nbyte-vector\n/inputs\n",
        "output/\ni64\n/output\n/sig\nparams/\nb\nbyte-vector\n/params\n",
        "byte-slice-length/\nborrow/\nb\n/borrow\n/byte-slice-length\n/fn\n/def\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n",
        "let/\nbind/\nb\nnew-byte-vector/\n3\n/new-byte-vector\n/bind\n",
        "take/\nmove/\nb\n/move\n/take\n/let\n/main\n",
    )
}
