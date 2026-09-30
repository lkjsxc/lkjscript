//! One request-local admission for affine expression visits and metadata reads.
use super::*;
use crate::platform::kernel::TypeObject;
use std::cell::Cell;

pub(super) struct Meter<'a> {
    used: &'a Cell<usize>,
    maximum: usize,
}

impl<'a> Meter<'a> {
    pub(super) fn new(used: &'a mut usize, maximum: usize) -> Self {
        Self {
            used: Cell::from_mut(used),
            maximum,
        }
    }

    pub(super) fn step(&self) -> Result<(), Diagnostic> {
        let used = self.used.get();
        if used >= self.maximum {
            return Err(exhaustion());
        }
        // The pre-admission comparison also proves this increment cannot overflow.
        self.used.set(used + 1);
        Ok(())
    }
}

pub(super) fn exhaustion() -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Resource,
        "kernel_affine_work",
        "affine validation exhausted its explicit work budget",
    )
}

/// All affine metadata paths, including imported signatures and recursive type
/// inspection, go through this reader. Exhaustion precedes the delegated read;
/// errors/checkpoints retain the underlying reader's meaning and cancellation.
pub(super) struct Read<'a, R: ?Sized> {
    pub(super) inner: &'a R,
    pub(super) meter: &'a Meter<'a>,
}

impl<R: ExpressionRead + ?Sized> ExpressionRead for Read<'_, R> {
    fn package_id(&self) -> PackageId {
        self.inner.package_id()
    }

    fn owner(&self, owner: OwnerKey) -> Result<Option<OwnerRecord>, Diagnostic> {
        self.meter.step()?;
        self.inner.owner(owner)
    }

    fn type_object(&self, digest: TypeObjectDigest) -> Result<Option<TypeObject>, Diagnostic> {
        self.meter.step()?;
        self.inner.type_object(digest)
    }

    fn package_interface_owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<Option<PackageInterfaceRecord>, Diagnostic> {
        self.meter.step()?;
        self.inner.package_interface_owner(package, owner)
    }

    fn has_dependency(&self, package: PackageId) -> Result<bool, Diagnostic> {
        self.meter.step()?;
        self.inner.has_dependency(package)
    }

    fn validation_checkpoint(&self) -> Result<(), Diagnostic> {
        self.inner.validation_checkpoint()
    }

    fn validation_work(&self) -> Result<(), Diagnostic> {
        self.meter.step()?;
        self.inner.validation_work()
    }
}
