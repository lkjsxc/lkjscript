use std::num::{NonZeroU64, NonZeroUsize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use lkjscript_core::{CapabilityKind, ExecutionConfig};
use lkjscript_runtime::{
    ApplicationKind, ApplicationManifest, DeploymentScope, ExecutionCellClass, PackageContentId,
    ResourceQuota, RestartPolicy,
};

pub(super) fn manifest(name: &str) -> ApplicationManifest {
    ApplicationManifest {
        name: name.into(),
        kind: ApplicationKind::Service,
        scope: DeploymentScope::Standalone,
        cell: ExecutionCellClass::IsolatedProcess {
            entry: lkjscript_host::ApplicationPath::parse("src/examples/hello/main.lkjscript")
                .expect("portable entry"),
        },
        capabilities: vec![CapabilityKind::Stdio],
        quota: ResourceQuota {
            max_concurrent_invocations: NonZeroUsize::new(2).expect("concurrency"),
            max_total_invocations: NonZeroU64::new(8).expect("total"),
            execution: ExecutionConfig {
                max_output_bytes: 1024 * 1024,
                ..ExecutionConfig::default()
            },
        },
        restart: RestartPolicy::Never,
    }
}

pub(super) fn host(stdio: &lkjscript_host::BufferedStdio) -> lkjscript_host::HostEnvironment {
    lkjscript_host::HostEnvironment {
        stdio: Some(Arc::new(stdio.clone())),
        ..lkjscript_host::HostEnvironment::default()
    }
}

pub(super) fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

pub(super) fn package() -> PackageContentId {
    let digest = lkjscript_contracts::ContractDigest::from_hex(
        "5abaa965951efcfd87b27e57d776a8eddec68e25ae644cf167d62a7b04e47b80",
    )
    .expect("workspace package identity");
    PackageContentId::new(digest.as_bytes()).expect("package identity")
}
