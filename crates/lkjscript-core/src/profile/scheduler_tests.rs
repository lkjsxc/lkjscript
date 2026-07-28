#![allow(clippy::unwrap_used)]

use super::*;

#[test]
fn scheduler_category_names_order_and_units_are_exact() {
    let expected = [
        (ResourceCategory::WorkerThreads, "worker_threads", "records"),
        (ResourceCategory::WorkerGroups, "worker_groups", "records"),
        (ResourceCategory::TaskClasses, "task_classes", "records"),
        (ResourceCategory::TaskInstances, "task_instances", "records"),
        (
            ResourceCategory::TaskDependencies,
            "task_dependencies",
            "records",
        ),
        (
            ResourceCategory::TaskAccessRecords,
            "task_access_records",
            "records",
        ),
        (
            ResourceCategory::TaskScopeChildren,
            "task_scope_children",
            "records",
        ),
        (
            ResourceCategory::ReadyQueueEntries,
            "ready_queue_entries",
            "records",
        ),
        (
            ResourceCategory::TaskDescriptorBytes,
            "task_descriptor_bytes",
            "bytes",
        ),
        (
            ResourceCategory::TaskResultBytes,
            "task_result_bytes",
            "bytes",
        ),
        (
            ResourceCategory::WorkerScratchBytes,
            "worker_scratch_bytes",
            "bytes",
        ),
        (
            ResourceCategory::SchedulerWork,
            "scheduler_work",
            "work-units",
        ),
        (
            ResourceCategory::SchedulerDecisions,
            "scheduler_decisions",
            "records",
        ),
        (
            ResourceCategory::SchedulerSteals,
            "scheduler_steals",
            "records",
        ),
        (
            ResourceCategory::SchedulerMigrations,
            "scheduler_migrations",
            "records",
        ),
        (
            ResourceCategory::SchedulerWakeups,
            "scheduler_wakeups",
            "records",
        ),
        (
            ResourceCategory::RemoteReleaseRecords,
            "remote_release_records",
            "records",
        ),
        (
            ResourceCategory::DecisionTraceRecords,
            "decision_trace_records",
            "records",
        ),
    ];
    let actual = &ResourceCategory::ALL[54..];
    assert_eq!(actual.len(), expected.len());
    for (category, (expected_category, name, unit)) in actual.iter().zip(expected) {
        assert_eq!(*category, expected_category);
        assert_eq!(category.as_str(), name);
        assert_eq!(category.unit(), unit);
    }
}

#[test]
fn scheduler_profile_ceilings_are_exact() {
    let profiles = [
        ResourceProfileName::Sandbox,
        ResourceProfileName::Deterministic,
        ResourceProfileName::Default,
        ResourceProfileName::Build,
        ResourceProfileName::TrustedLocal,
    ];
    let expected = [
        [2, 4, 8, 32, 256],
        [1, 2, 4, 16, 128],
        [256, 1_024, 2_048, 8_192, 65_536],
        [8_192, 32_768, 65_536, 524_288, 4_194_304],
        [32_768, 131_072, 262_144, 2_097_152, 16_777_216],
        [32_768, 131_072, 262_144, 2_097_152, 16_777_216],
        [8_192, 32_768, 65_536, 524_288, 4_194_304],
        [8_192, 32_768, 65_536, 524_288, 4_194_304],
        [4_194_304, 16_777_216, 33_554_432, 134_217_728, 268_435_456],
        [4_194_304, 16_777_216, 33_554_432, 134_217_728, 268_435_456],
        [4_194_304, 16_777_216, 33_554_432, 134_217_728, 268_435_456],
        [1_000_000, 4_000_000, 8_000_000, 32_000_000, 64_000_000],
        [65_536, 262_144, 524_288, 2_097_152, 4_194_304],
        [16_384, 65_536, 131_072, 524_288, 1_048_576],
        [4_096, 16_384, 32_768, 131_072, 262_144],
        [16_384, 65_536, 131_072, 524_288, 1_048_576],
        [8_192, 32_768, 65_536, 262_144, 524_288],
        [65_536, 262_144, 524_288, 2_097_152, 4_194_304],
    ];
    for (category, expected) in ResourceCategory::ALL[54..].iter().zip(expected) {
        let actual = profiles.map(|name| ResourceProfile::new(name).ceilings().limit(*category));
        assert_eq!(actual, expected);
    }
}
