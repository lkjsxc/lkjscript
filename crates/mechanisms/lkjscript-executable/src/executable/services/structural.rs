use super::*;

/// Safe structural runtime boundary for invocation-local opaque words.
pub trait NativeStructuralRuntimeServices {
    fn publish_structural_static(
        &mut self,
        _bytes: &[u8],
        _value_type: StructuralTypeIdentity,
        _payload: StructuralPayloadKind,
        _storage: StructuralStorageRoute,
    ) -> Result<NativeStructuralOwner, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }
    fn publish_structural_unique(
        &mut self,
        _owner: NativeUnique,
        _value_type: StructuralTypeIdentity,
        _payload: StructuralPayloadKind,
        _storage: StructuralStorageRoute,
    ) -> Result<NativeStructuralOwner, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }
    fn publish_structural_i64(
        &mut self,
        _value: i64,
        _value_type: StructuralTypeIdentity,
        _storage: StructuralStorageRoute,
    ) -> Result<NativeStructuralOwner, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }
    fn publish_structural_formatted_i64(
        &mut self,
        _value: i64,
        _value_type: StructuralTypeIdentity,
    ) -> Result<NativeStructuralOwner, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }
    fn capture_structural_trap(
        &mut self,
        _owner: NativeStructuralOwner,
    ) -> Result<(), NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }
    fn convert_structural_numeric(
        &mut self,
        _input: NativeValue,
        _kind: StructuralNumericConversion,
        _success: &StructuralAggregateDescriptor,
        _failure: &StructuralAggregateDescriptor,
        _errors: &[StructuralAggregateDescriptor],
    ) -> Result<NativeStructuralOwner, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }
    fn publish_structural_owner(
        &mut self,
        _owner: NativeStructuralOwner,
        _storage: StructuralStorageRoute,
    ) -> Result<NativeStructuralOwner, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }
    fn copy_structural(
        &mut self,
        _owner: NativeStructuralOwner,
    ) -> Result<NativeStructuralOwner, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }
    fn move_structural(
        &mut self,
        _owner: NativeStructuralOwner,
    ) -> Result<NativeStructuralOwner, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }
    fn independent_structural_owner(
        &mut self,
        _witness: u16,
        _key: u64,
    ) -> Result<u64, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }
    fn dispose_structural_owner(
        &mut self,
        _witness: u16,
        _key: u64,
    ) -> Result<(), NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }
    fn compare_structural_values(
        &mut self,
        _witness: u16,
        _left: u64,
        _right: u64,
    ) -> Result<bool, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }
    fn copy_structural_view(
        &mut self,
        _view: NativeStructuralView,
    ) -> Result<NativeValue, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }
    fn borrow_structural(
        &mut self,
        _owner: NativeStructuralOwner,
        _projection: &StructuralProjectionDescriptor,
        _start: i64,
        _end: i64,
    ) -> Result<NativeStructuralView, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }
    fn borrow_structural_utf8(
        &mut self,
        _owner: NativeStructuralOwner,
        _projection: &StructuralProjectionDescriptor,
    ) -> Result<NativeStructuralView, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }
    fn end_structural_view(
        &mut self,
        _view: NativeStructuralView,
    ) -> Result<(), NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }
    fn drop_structural(&mut self, _owner: NativeStructuralOwner) -> Result<(), NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }
    fn create_structural_destination(
        &mut self,
        _aggregate: &StructuralAggregateDescriptor,
        _storage: StructuralStorageRoute,
    ) -> Result<NativeStructuralDestination, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }
    fn initialize_structural_destination(
        &mut self,
        _destination: NativeStructuralDestination,
        _value: NativeValue,
        _aggregate: &StructuralAggregateDescriptor,
        _storage: StructuralStorageRoute,
        _field: u16,
    ) -> Result<NativeStructuralDestination, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }
    fn finish_structural_destination(
        &mut self,
        _destination: NativeStructuralDestination,
        _aggregate: &StructuralAggregateDescriptor,
        _storage: StructuralStorageRoute,
    ) -> Result<NativeStructuralOwner, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }
    fn abort_structural_destination(
        &mut self,
        _destination: NativeStructuralDestination,
    ) -> Result<(), NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }
    fn structural_tag(&mut self, _view: NativeStructuralView) -> Result<i64, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }
    fn structural_owned_tag(
        &mut self,
        _owner: NativeStructuralOwner,
    ) -> Result<i64, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }
    fn structural_payload_length(
        &mut self,
        _owner: NativeStructuralOwner,
    ) -> Result<i64, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }
    fn consume_structural_payload(
        &mut self,
        _owner: NativeStructuralOwner,
        _aggregate: &StructuralAggregateDescriptor,
    ) -> Result<NativeValue, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }
    fn structural_i64(&mut self, _view: NativeStructuralView) -> Result<i64, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }
    fn structural_payload_bytes_equal(
        &mut self,
        _left: NativeStructuralView,
        _right: NativeStructuralView,
    ) -> Result<bool, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }
    fn structural_payload_utf8_valid(
        &mut self,
        _view: NativeStructuralView,
    ) -> Result<bool, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }
}
