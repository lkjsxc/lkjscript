//! A selected payload with the same sealed, loan-aware storage as owned products.
use super::owned_product::OwnedProduct;
use super::value::{NormalizedValue, ValueOrigin};
use crate::platform::execution::{ExecutionControl, ExecutionError};
use crate::platform::kernel::TypeObjectDigest;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnedChoice {
    pub(super) storage: OwnedProduct,
    case: u32,
}
impl OwnedChoice {
    pub(super) const ALLOCATION_BYTES: u64 = OwnedProduct::ALLOCATION_BYTES
        + (std::mem::size_of::<NormalizedValue>() + std::mem::size_of::<Self>()
            - std::mem::size_of::<OwnedProduct>()) as u64;
    pub(super) fn create(
        origin: ValueOrigin,
        ty: TypeObjectDigest,
        case: u32,
        payload: NormalizedValue,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<Self, ExecutionError> {
        control.check()?;
        // The selected case is inline; its payload vector and control block are charged.
        reserve(
            (std::mem::size_of::<NormalizedValue>() + std::mem::size_of::<Self>()
                - std::mem::size_of::<OwnedProduct>()) as u64,
        )?;
        control.check()?;
        let storage = OwnedProduct::create(origin, ty, vec![payload], control, reserve)?;
        Ok(Self { storage, case })
    }
    pub(super) fn ty(&self) -> TypeObjectDigest {
        self.storage.ty()
    }
    pub(super) fn validate(
        &self,
        origin: ValueOrigin,
        consume: bool,
    ) -> Result<(), ExecutionError> {
        self.storage.validate(origin, consume)
    }
    pub(super) fn borrow(&self) -> Result<Self, ExecutionError> {
        Ok(Self {
            storage: self.storage.borrow()?,
            case: self.case,
        })
    }
    pub(super) fn establish_admission(&self, program: ValueOrigin) -> Result<(), ExecutionError> {
        self.storage.establish_admission(program)
    }
    pub(super) fn validate_admission(&self, program: ValueOrigin) -> Result<(), ExecutionError> {
        self.storage.validate_admission(program)
    }
    pub(super) fn inherit_admission(&self, program: ValueOrigin) -> Result<(), ExecutionError> {
        self.storage.inherit_admission(program)
    }
    pub(super) fn adopt_scoped_read(
        &mut self,
        source: ValueOrigin,
        destination: ValueOrigin,
    ) -> Result<(), ExecutionError> {
        self.storage.adopt_scoped_read(source, destination)
    }
    pub(super) fn is_borrowed(&self) -> bool {
        self.storage.is_borrowed()
    }
    pub(super) fn owns_live_loans(&self) -> bool {
        self.storage.owns_live_loans()
    }
    pub(super) fn case(&self) -> u32 {
        self.case
    }
    pub(super) fn borrow_payload(
        &self,
        origin: ValueOrigin,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<NormalizedValue, ExecutionError> {
        self.storage.borrow_field(origin, 0, control, reserve)
    }
    pub(super) fn inspect_transfer<R>(
        &self,
        source: ValueOrigin,
        inspect: impl FnOnce(&NormalizedValue) -> Result<R, ExecutionError>,
    ) -> Result<R, ExecutionError> {
        self.storage.inspect_transfer(source, |fields| {
            let [payload] = fields else {
                return Err(ExecutionError::resource(
                    "normalized_choice_token",
                    "owned choice has no unique selected payload",
                ));
            };
            inspect(payload)
        })
    }
    pub(super) fn adopt_transfer(
        &mut self,
        source: ValueOrigin,
        destination: ValueOrigin,
        adopt: impl FnOnce(&mut NormalizedValue) -> Result<(), ExecutionError>,
    ) -> Result<(), ExecutionError> {
        self.storage.adopt_transfer(source, destination, |fields| {
            let [payload] = fields else {
                return Err(ExecutionError::resource(
                    "normalized_choice_token",
                    "owned choice has no unique selected payload",
                ));
            };
            adopt(payload)
        })
    }
    pub(super) fn select(
        self,
        origin: ValueOrigin,
        ty: TypeObjectDigest,
        control: &ExecutionControl,
    ) -> Result<(u32, NormalizedValue), ExecutionError> {
        let mut payload = self.storage.unpack(origin, ty, control)?;
        if payload.len() != 1 {
            return Err(ExecutionError::resource(
                "normalized_choice_token",
                "owned choice has no unique selected payload",
            ));
        }
        let payload = payload.pop().ok_or_else(|| {
            ExecutionError::resource(
                "normalized_choice_token",
                "owned choice selected payload is missing",
            )
        })?;
        Ok((self.case, payload))
    }
}
