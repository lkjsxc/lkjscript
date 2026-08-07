use super::*;
use crate::island::JitIslandServices;
use lkjscript_executable::NativeStructuralRuntimeServices;
use lkjscript_native::NativeUnique;

mod lifecycle;
use lifecycle::lifecycle_services;

impl NativeStructuralRuntimeServices for JitIslandServices {
    fn publish_structural_static(
        &mut self,
        bytes: &[u8],
        value_type: StructuralTypeIdentity,
        payload: StructuralPayloadKind,
        storage: StructuralStorageRoute,
    ) -> Result<NativeStructuralOwner, NativeServiceError> {
        self.structural
            .publish_static(bytes, value_type, payload, storage)
    }
    fn publish_structural_unique(
        &mut self,
        owner: NativeUnique,
        value_type: StructuralTypeIdentity,
        payload: StructuralPayloadKind,
        storage: StructuralStorageRoute,
    ) -> Result<NativeStructuralOwner, NativeServiceError> {
        let bytes = self.unique.export_owner(owner)?;
        self.structural
            .publish_unique(bytes, value_type, payload, storage)
    }

    fn publish_structural_i64(
        &mut self,
        value: i64,
        value_type: StructuralTypeIdentity,
        storage: StructuralStorageRoute,
    ) -> Result<NativeStructuralOwner, NativeServiceError> {
        self.structural.publish_i64(value, value_type, storage)
    }

    fn publish_structural_formatted_i64(
        &mut self,
        value: i64,
        value_type: StructuralTypeIdentity,
    ) -> Result<NativeStructuralOwner, NativeServiceError> {
        self.structural.publish_formatted_i64(value, value_type)
    }

    fn capture_structural_trap(
        &mut self,
        owner: NativeStructuralOwner,
    ) -> Result<(), NativeServiceError> {
        self.structural.capture_trap(owner)
    }

    fn convert_structural_numeric(
        &mut self,
        input: NativeValue,
        kind: StructuralNumericConversion,
        success: &StructuralAggregateDescriptor,
        failure: &StructuralAggregateDescriptor,
        errors: &[StructuralAggregateDescriptor],
    ) -> Result<NativeStructuralOwner, NativeServiceError> {
        self.structural
            .convert_numeric(input, kind, success, failure, errors)
    }

    lifecycle_services!();

    fn copy_structural_view(
        &mut self,
        view: NativeStructuralView,
    ) -> Result<NativeValue, NativeServiceError> {
        self.structural.copy_view(view)
    }

    fn borrow_structural(
        &mut self,
        owner: NativeStructuralOwner,
        projection: &StructuralProjectionDescriptor,
        start: i64,
        end: i64,
    ) -> Result<NativeStructuralView, NativeServiceError> {
        self.structural.borrow(owner, projection, start, end)
    }

    fn borrow_structural_utf8(
        &mut self,
        owner: NativeStructuralOwner,
        projection: &StructuralProjectionDescriptor,
    ) -> Result<NativeStructuralView, NativeServiceError> {
        self.structural.borrow_utf8(owner, projection)
    }

    fn end_structural_view(
        &mut self,
        view: NativeStructuralView,
    ) -> Result<(), NativeServiceError> {
        self.structural.end_view(view)
    }

    fn create_structural_destination(
        &mut self,
        aggregate: &StructuralAggregateDescriptor,
        storage: StructuralStorageRoute,
    ) -> Result<NativeStructuralDestination, NativeServiceError> {
        self.structural.create_destination(aggregate, storage)
    }

    fn initialize_structural_destination(
        &mut self,
        destination: NativeStructuralDestination,
        value: NativeValue,
        aggregate: &StructuralAggregateDescriptor,
        storage: StructuralStorageRoute,
        field: u64,
    ) -> Result<NativeStructuralDestination, NativeServiceError> {
        self.structural
            .initialize_destination(destination, value, aggregate, storage, field)
    }

    fn finish_structural_destination(
        &mut self,
        destination: NativeStructuralDestination,
        aggregate: &StructuralAggregateDescriptor,
        storage: StructuralStorageRoute,
    ) -> Result<NativeStructuralOwner, NativeServiceError> {
        self.structural
            .finish_destination(destination, aggregate, storage)
    }

    fn abort_structural_destination(
        &mut self,
        destination: NativeStructuralDestination,
    ) -> Result<(), NativeServiceError> {
        self.structural.abort_destination(destination)
    }

    fn structural_tag(&mut self, view: NativeStructuralView) -> Result<i64, NativeServiceError> {
        self.structural.tag(view)
    }

    fn structural_owned_tag(
        &mut self,
        owner: NativeStructuralOwner,
    ) -> Result<i64, NativeServiceError> {
        self.structural.owned_tag(owner)
    }

    fn structural_payload_length(
        &mut self,
        owner: NativeStructuralOwner,
    ) -> Result<i64, NativeServiceError> {
        self.structural.length(owner)
    }

    fn consume_structural_payload(
        &mut self,
        owner: NativeStructuralOwner,
        aggregate: &StructuralAggregateDescriptor,
    ) -> Result<NativeValue, NativeServiceError> {
        self.structural.consume_payload(owner, aggregate)
    }

    fn structural_i64(&mut self, view: NativeStructuralView) -> Result<i64, NativeServiceError> {
        self.structural.i64(view)
    }

    fn structural_payload_bytes_equal(
        &mut self,
        left: NativeStructuralView,
        right: NativeStructuralView,
    ) -> Result<bool, NativeServiceError> {
        self.structural.bytes_equal(left, right)
    }

    fn structural_payload_utf8_valid(
        &mut self,
        view: NativeStructuralView,
    ) -> Result<bool, NativeServiceError> {
        self.structural.utf8_valid(view)
    }
}
