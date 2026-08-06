use std::num::{NonZeroU64, NonZeroUsize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use lkjscript_core::{CapabilityKind, ExecutionConfig};
use lkjscript_runtime::{
    ApplicationKind, ApplicationManifest, DeploymentScope, ExecutionCellClass, PackageContentId,
    ResourceQuota, RestartPolicy,
};

struct PackageFixture {
    root: PathBuf,
    package: PackageContentId,
}

static PACKAGE: OnceLock<PackageFixture> = OnceLock::new();

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

pub(super) fn structural_manifest(name: &str) -> ApplicationManifest {
    let mut value = manifest(name);
    value.cell = ExecutionCellClass::IsolatedProcess {
        entry: lkjscript_host::ApplicationPath::parse("structural/main.lkjscript")
            .expect("structural entry"),
    };
    value.capabilities.clear();
    value
}

pub(super) fn host(stdio: &lkjscript_host::BufferedStdio) -> lkjscript_host::HostEnvironment {
    lkjscript_host::HostEnvironment {
        stdio: Some(Arc::new(stdio.clone())),
        ..lkjscript_host::HostEnvironment::default()
    }
}

pub(super) fn root() -> PathBuf {
    fixture().root.clone()
}

pub(super) fn package() -> PackageContentId {
    fixture().package
}

fn fixture() -> &'static PackageFixture {
    PACKAGE.get_or_init(|| build_fixture().expect("temporary locked process package"))
}

fn build_fixture() -> Result<PackageFixture, Box<dyn std::error::Error>> {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let root = target.join(format!("process-package-{}", std::process::id()));
    if root.exists() {
        std::fs::remove_dir_all(&root)?;
    }
    std::fs::create_dir_all(&root)?;
    let modules = [
        "src/examples/hello/base.lkjscript",
        "src/examples/hello/dec.lkjscript",
        "src/examples/hello/fact.lkjscript",
        "src/examples/hello/main.lkjscript",
    ];
    for module in modules {
        let destination = root.join(module);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(workspace.join(module), destination)?;
    }
    std::fs::create_dir_all(root.join("structural"))?;
    std::fs::write(root.join("structural/main.lkjscript"), NESTED_RETURN)?;
    let contracts = lkjscript_contracts::current_contracts()?;
    let contract = contracts
        .get(lkjscript_contracts::PACKAGE_MANIFEST)
        .ok_or("package manifest contract is absent")?
        .digest()
        .to_hex();
    let manifest = format!(
        "{{\"schema\":\"lkjscript.package\",\"contract\":\"{contract}\",\
         \"name\":\"process-test\",\"source_root\":\".\",\
         \"modules\":[\"src/examples/hello/base.lkjscript\",\
         \"src/examples/hello/dec.lkjscript\",\"src/examples/hello/fact.lkjscript\",\
         \"src/examples/hello/main.lkjscript\",
         \"structural/main.lkjscript\"],\
         \"public\":[\"src/examples/hello/main.lkjscript\",\"structural/main.lkjscript\"],\
         \"dependencies\":[],\"capabilities\":[\"stdio\"],\
         \"targets\":[{{\"name\":\"hello-main\",\"module\":\"src/examples/hello/main.lkjscript\"}},\
         {{\"name\":\"nested-return\",\"module\":\"structural/main.lkjscript\"}}]}}"
    );
    std::fs::write(
        root.join(lkjscript_compiler::package::MANIFEST_FILE),
        manifest,
    )?;
    let entry = root.join("src/examples/hello/main.lkjscript");
    let (lock_path, lock) = lkjscript_compiler::package::create_lock(&entry)?;
    std::fs::write(lock_path, lock)?;
    let (_, _, content) = lkjscript_compiler::package::verify_content(&entry)?;
    Ok(PackageFixture {
        root,
        package: PackageContentId::new(content.as_bytes()).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "zero package identity")
        })?,
    })
}

const NESTED_RETURN: &str = concat!(
    "product/\nname/\ninner\n/name\nfields/\nfield/\nname/\nvalue\n/name\n",
    "type/\ni64\n/type\n/field\n/fields\n/product\nproduct/\nname/\nouter\n/name\n",
    "fields/\nfield/\nname/\nnested\n/name\ntype/\nproduct\ninner\n/type\n/field\n",
    "field/\nname/\nanswer\n/name\ntype/\ni64\n/type\n/field\n/fields\n/product\n",
    "main/\nsig/\ninputs/\n/inputs\noutput/\nproduct/\nouter\n/product\n/output\n/sig\n",
    "product-value/\nouter\nfield/\nnested\nproduct-value/\ninner\nfield/\nvalue\n",
    "7\n/field\n/product-value\n/field\nfield/\nanswer\n42\n/field\n/product-value\n/main\n",
);
