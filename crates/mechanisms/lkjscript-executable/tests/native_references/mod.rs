#![allow(clippy::panic)]

use lkjscript_executable::{
    ExecutableInstaller, ExecutableLimits, InvocationOutcome, NativeInvocationConfig,
    NativeResourceLimitKind, NativeRoot, NativeRuntimeServices, NativeServiceError,
};
use lkjscript_native::{
    encode, AllocationClass, BackendLimits, EncodingConfig, FunctionId, HeapCallDescriptor,
    HeapOperation, HeapRuntimeSite, ImageContracts, InstallableImage, LayoutIdentity,
    MachinePlanBuilder, NativeReference, NativeValue, ReferenceType, RuntimeCallSlot,
    RuntimeOutcome, Signature, SourceFunctionId, StoreClass, TrapCode, ValueType,
};

#[derive(Clone, Copy)]
enum HeapFailure {
    Trap,
    Resource,
    Host,
}

#[derive(Default)]
struct RecordingServices {
    observed: Vec<Vec<(ReferenceType, u64)>>,
    replacement: Option<u64>,
    failure: Option<NativeServiceError>,
}

impl NativeRuntimeServices for RecordingServices {
    fn collect_references(&mut self, roots: &mut [NativeRoot]) -> Result<(), NativeServiceError> {
        self.observed.push(
            roots
                .iter()
                .map(|root| (root.reference_type(), root.opaque_word()))
                .collect(),
        );
        if let Some(failure) = self.failure {
            return Err(failure);
        }
        if let Some(replacement) = self.replacement {
            for root in roots {
                root.set_opaque_word(replacement);
            }
        }
        Ok(())
    }
}

fn product_ref(word: u64) -> NativeValue {
    NativeValue::Reference(NativeReference::new(
        ReferenceType::Product(LayoutIdentity::product(0)),
        word,
    ))
}

mod active_frames;
mod collection;
mod heap;
mod image;
mod wide_roots;

use image::*;
