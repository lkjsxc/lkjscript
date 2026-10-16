//! Bounded immutable supplier compilation reuse after complete current source admission.
//!
//! This cache owns no repository, readiness binding, grant or invocation state. Every caller
//! first exports and admits its physical source closure and independently reconstructs the
//! reference oracle. Only the resulting immutable compiler/linker output is shared here.

use crate::platform::compiler::{LoadedArtifact, OptimizationPolicy, compile_immutable};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::execution::ExecutionControl;
use crate::platform::execution::normalized::execution_diagnostic;
use crate::platform::kernel::PackageRevisionDigest;
use crate::platform::package_transport::source::{
    AdmittedClosure, ImmutablePackage, PackageContainer,
};
use std::collections::VecDeque;
use std::mem::size_of;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock};
use std::time::Duration;

const MAXIMUM_RETAINED_BYTES: usize = 64 * 1_048_576;
const MAXIMUM_ENTRIES: usize = 32;

#[derive(Clone, Debug, Eq, PartialEq)]
struct DependencyKey {
    manifest: [u8; 32],
    bundle: [u8; 32],
    admitted_content: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Key {
    revision: [u8; 32],
    transport: [u8; 32],
    witness: [u8; 32],
    schema: u16,
    validator: [u8; 32],
    implementation: [u8; 32],
    optimization: OptimizationPolicy,
    dependencies: Vec<DependencyKey>,
}

impl Key {
    fn admitted(
        package: &ImmutablePackage,
        dependencies: &[LoadedArtifact],
        control: &ExecutionControl,
    ) -> Result<Self, Diagnostic> {
        checkpoint(control)?;
        if dependencies.len() > crate::platform::package_transport::MAXIMUM_PACKAGE_CLOSURE {
            return Err(binding_error());
        }
        let validator = crate::platform::witness::contract::validator_contract_digest();
        if package.revision.encode()?.0 != package.binding.package_revision
            || package.transport.encode()?.0 != package.binding.transport
            || package.transport.package_revision != package.binding.package_revision
            || !package.witness.manifest.contract_is_current()
            || package.witness.manifest.validator_contract != validator
            || package.witness.manifest.semantic_root != package.transport.semantic_root
        {
            return Err(binding_error());
        }
        let mut bindings = Vec::with_capacity(dependencies.len());
        for dependency in dependencies {
            let admitted_content = dependency.admitted_content_digest(|| checkpoint(control))?;
            bindings.push(DependencyKey {
                manifest: dependency.manifest_digest.bytes(),
                bundle: dependency.bundle_digest.bytes(),
                admitted_content,
            });
        }
        checkpoint(control)?;
        Ok(Self {
            revision: package.binding.package_revision.bytes(),
            transport: package.binding.transport.bytes(),
            witness: package.witness.manifest_digest.bytes(),
            schema: crate::platform::canonical_schema::SCHEMA_EPOCH,
            validator: validator.bytes(),
            implementation: crate::platform::canonical_schema::VALIDATOR_IMPLEMENTATION_HASH,
            optimization: OptimizationPolicy::DeterministicBaseline,
            dependencies: bindings,
        })
    }
}

#[derive(Debug)]
pub(super) struct PreparedSupplier {
    pub artifact: Arc<LoadedArtifact>,
    /// A builtin artifact was already proved by clean compilation; it is reused even on
    /// the first pool insertion. All other misses perform the ordinary immutable compiler.
    pub reused: bool,
}

pub(super) fn prepare(
    closure: &AdmittedClosure,
    revision: PackageRevisionDigest,
    dependencies: &[LoadedArtifact],
    control: &ExecutionControl,
) -> Result<PreparedSupplier, Diagnostic> {
    let package = closure.packages.get(&revision).ok_or_else(binding_error)?;
    if package.binding.package_revision != revision
        || !closure.container.selections.contains(&package.binding)
    {
        return Err(binding_error());
    }
    let key = Key::admitted(package, dependencies, control)?;
    static POOL: OnceLock<Pool> = OnceLock::new();
    POOL.get_or_init(|| Pool::new(MAXIMUM_ENTRIES, MAXIMUM_RETAINED_BYTES))
        .prepare(key, control, || {
            if let Some(artifact) = embedded_standard(package, closure, dependencies, control)? {
                return Ok(PreparedSupplier {
                    artifact: Arc::new(artifact),
                    reused: true,
                });
            }
            checkpoint(control)?;
            let linked = compile_immutable(package, &closure.container.objects, dependencies)?;
            checkpoint(control)?;
            let (_, artifact) = linked.artifact.into_admitted()?;
            checkpoint(control)?;
            Ok(PreparedSupplier {
                artifact: Arc::new(artifact),
                reused: false,
            })
        })
}

fn embedded_standard(
    package: &ImmutablePackage,
    closure: &AdmittedClosure,
    dependencies: &[LoadedArtifact],
    control: &ExecutionControl,
) -> Result<Option<LoadedArtifact>, Diagnostic> {
    if package.revision.package != crate::platform::builtin_standard::builtin_standard_package()?
        || !dependencies.is_empty()
    {
        return Ok(None);
    }
    let standard = crate::platform::builtin_standard::BuiltinStandard::load()?;
    if package.binding.package_revision != standard.package_revision
        || package.binding.transport != standard.package_transport
    {
        return Ok(None);
    }
    // Identity alone does not certify another physical input. Compare every exact embedded
    // canonical object against the newly admitted closure before sharing its preadmitted code.
    let embedded =
        PackageContainer::decode(standard.transport_bytes(), standard.package_transport)?;
    for (key, bytes) in &embedded.objects {
        checkpoint(control)?;
        if closure.container.objects.get(key) != Some(bytes) {
            return Err(binding_error());
        }
    }
    checkpoint(control)?;
    Ok(Some(standard.artifact.clone()))
}

struct Entry {
    key: Key,
    value: Option<(Arc<LoadedArtifact>, usize)>,
    reserved_bytes: usize,
}

#[derive(Default)]
struct State {
    entries: VecDeque<Entry>,
    retained_bytes: usize,
}

struct Pool {
    state: Mutex<State>,
    changed: Condvar,
    maximum_entries: usize,
    maximum_bytes: usize,
}

impl Pool {
    fn new(maximum_entries: usize, maximum_bytes: usize) -> Self {
        Self {
            state: Mutex::new(State::default()),
            changed: Condvar::new(),
            maximum_entries,
            maximum_bytes,
        }
    }

    fn lock(&self) -> Result<MutexGuard<'_, State>, Diagnostic> {
        self.state.lock().map_err(|_| {
            Diagnostic::new(
                DiagnosticClass::Infrastructure,
                "lifecycle_supplier_cache_lock",
                "immutable supplier cache metadata is unavailable",
            )
        })
    }

    fn prepare(
        &self,
        key: Key,
        control: &ExecutionControl,
        compile: impl FnOnce() -> Result<PreparedSupplier, Diagnostic>,
    ) -> Result<PreparedSupplier, Diagnostic> {
        let reservation = key_retained_bytes(&key).unwrap_or(usize::MAX);
        let slot = loop {
            checkpoint(control)?;
            let mut state = self.lock()?;
            if let Some(entry) = state.entries.iter().find(|entry| entry.key == key) {
                if let Some((artifact, _)) = &entry.value {
                    let artifact = Arc::clone(artifact);
                    drop(state);
                    checkpoint(control)?;
                    return Ok(PreparedSupplier {
                        artifact,
                        reused: true,
                    });
                }
                // Waiting never owns a compiler/store lock. Its own cancellation/deadline is
                // observed independently of the invocation currently preparing this key.
                drop(
                    self.changed
                        .wait_timeout(state, Duration::from_millis(20))
                        .map_err(|_| {
                            Diagnostic::new(
                                DiagnosticClass::Infrastructure,
                                "lifecycle_supplier_cache_lock",
                                "immutable supplier cache wait failed",
                            )
                        })?,
                );
                continue;
            }
            if state.entries.len() >= self.maximum_entries {
                evict_ready(&mut state);
            }
            if reservation > self.maximum_bytes {
                break None;
            }
            while state.retained_bytes > self.maximum_bytes - reservation {
                if !evict_ready(&mut state) {
                    break;
                }
            }
            if state.entries.len() >= self.maximum_entries
                || state.retained_bytes > self.maximum_bytes - reservation
            {
                // Optional cache capacity cannot refuse or serialize unrelated valid code.
                break None;
            }
            state.entries.push_back(Entry {
                key: key.clone(),
                value: None,
                reserved_bytes: reservation,
            });
            state.retained_bytes += reservation;
            break Some(Slot {
                pool: self,
                key: key.clone(),
                published: false,
            });
        };
        checkpoint(control)?;
        let prepared = compile()?;
        checkpoint(control)?;
        if let Some(mut slot) = slot {
            slot.publish(&prepared.artifact, control)?;
        }
        checkpoint(control)?;
        Ok(prepared)
    }
}

struct Slot<'a> {
    pool: &'a Pool,
    key: Key,
    published: bool,
}

impl Slot<'_> {
    fn publish(
        &mut self,
        artifact: &Arc<LoadedArtifact>,
        control: &ExecutionControl,
    ) -> Result<(), Diagnostic> {
        let Some(bytes) = retained_bytes(&self.key, artifact) else {
            return Ok(());
        };
        if bytes > self.pool.maximum_bytes {
            return Ok(());
        }
        let mut state = self.pool.lock()?;
        checkpoint(control)?;
        let Some(reserved) = state
            .entries
            .iter()
            .find(|entry| entry.key == self.key)
            .map(|entry| entry.reserved_bytes)
        else {
            return Err(binding_error());
        };
        let additional = bytes.checked_sub(reserved).ok_or_else(binding_error)?;
        while state.retained_bytes > self.pool.maximum_bytes - additional {
            if !evict_ready(&mut state) {
                return Ok(());
            }
        }
        if let Some(entry) = state.entries.iter_mut().find(|entry| entry.key == self.key) {
            entry.value = Some((Arc::clone(artifact), bytes));
            state.retained_bytes += additional;
            self.published = true;
        }
        Ok(())
    }
}

impl Drop for Slot<'_> {
    fn drop(&mut self) {
        if !self.published
            && let Ok(mut state) = self.pool.state.lock()
            && let Some(index) = state
                .entries
                .iter()
                .position(|entry| entry.key == self.key && entry.value.is_none())
            && let Some(entry) = state.entries.remove(index)
        {
            state.retained_bytes -= entry.reserved_bytes;
        }
        self.pool.changed.notify_all();
    }
}

fn evict_ready(state: &mut State) -> bool {
    let Some(index) = state.entries.iter().position(|entry| entry.value.is_some()) else {
        return false;
    };
    if let Some(Entry {
        value: Some((_, bytes)),
        ..
    }) = state.entries.remove(index)
    {
        state.retained_bytes -= bytes;
    }
    true
}

fn retained_bytes(key: &Key, artifact: &LoadedArtifact) -> Option<usize> {
    // Reserve one entire possible BTree node per retained object. Vec capacities, nested
    // manifest/runtime-owner storage, key storage and generous cache/Arc metadata are included.
    let mut bytes = key_retained_bytes(key)?
        .checked_add(size_of::<LoadedArtifact>())?
        .checked_add(
            artifact
                .manifest
                .packages
                .capacity()
                .checked_mul(size_of::<crate::platform::compiler::ArtifactPackage>())?,
        )?;
    for package in &artifact.manifest.packages {
        bytes = bytes.checked_add(
            package
                .runtime_owners
                .capacity()
                .checked_mul(size_of::<crate::platform::compiler::ArtifactRuntimeOwner>())?,
        )?;
    }
    for value in artifact.objects.values() {
        bytes = bytes.checked_add(value.capacity())?.checked_add(
            256 + 12 * size_of::<(crate::platform::storage::object::ObjectKey, Vec<u8>)>(),
        )?;
    }
    Some(bytes)
}

fn key_retained_bytes(key: &Key) -> Option<usize> {
    1_024_usize.checked_add(
        key.dependencies
            .capacity()
            .checked_mul(size_of::<DependencyKey>())?,
    )
}

fn checkpoint(control: &ExecutionControl) -> Result<(), Diagnostic> {
    control.check().map_err(execution_diagnostic)
}

fn binding_error() -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Corrupt,
        "lifecycle_supplier_cache_binding",
        "immutable supplier compilation does not match its freshly admitted current source binding",
    )
}

#[cfg(test)]
mod tests;
