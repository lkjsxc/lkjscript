use std::sync::atomic::{AtomicU64, Ordering};

use super::*;
use lkjscript_core::{ResourceProfile, ResourceProfileName};
use lkjscript_native::{
    encode, BackendLimits, EncodingConfig, MachinePlanBuilder, RuntimeCallSlot, Signature,
    SourceFunctionId, ValueType,
};

static NEXT: AtomicU64 = AtomicU64::new(0);

pub(super) struct Root(pub(super) std::path::PathBuf);

impl Root {
    pub(super) fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "lkjscript-native-cache-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).expect("temporary cache package root");
        Self(path)
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub(super) fn context(root: &Root) -> CacheContext {
    CacheContext {
        package_root: root.0.clone(),
        module_path: "src/main.lkjscript".into(),
        source_sha256: [1; 32],
        module_sha256: [2; 32],
        package_sha256: [3; 32],
        lock_sha256: [4; 32],
        profile: ResourceProfile::new(ResourceProfileName::Default).identity(),
    }
}

pub(super) fn key(root: &Root) -> ArtifactKey {
    artifact_key(
        &context(root),
        [5; 32],
        CacheTier::Baseline,
        0,
        [0; 32],
        BackendLimits::default(),
    )
    .expect("artifact key")
}

pub(super) fn image() -> lkjscript_native::InstallableImage {
    let mut plan = MachinePlanBuilder::new();
    let function = plan
        .declare_function(
            SourceFunctionId::new(0),
            Signature::new(vec![ValueType::I64], ValueType::I64).expect("signature"),
        )
        .expect("declare");
    let mut builder = plan.function_builder(function).expect("builder");
    let entry = builder.create_block().expect("block");
    builder.set_entry(entry).expect("entry");
    let input = builder.parameter(0).expect("parameter");
    let value = builder
        .runtime_call(entry, RuntimeCallSlot::IdentityI64, vec![input])
        .expect("runtime call");
    builder.return_value(entry, value).expect("return");
    plan.define_function(builder.finish()).expect("definition");
    encode(
        plan.verify(BackendLimits::default())
            .expect("verified plan"),
        EncodingConfig::default(),
    )
    .expect("image")
}
