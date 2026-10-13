//! Current read-view-capability associated bodies and frozen associated-owned bytes.
//! Base and predecessor codecs never discard associated or read-scope inventories.

use super::frozen_read_view_wire::{
    AssociatedTypeBinding33, AssociatedTypeMember33, OwnedContract33, OwnedImplementation33,
};
use super::*;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::semantic_id::{
    AssociatedTypeMemberId, ImplementationParameterId, ModuleId, TypeParameterId,
};
use bincode::de::Decoder;
use bincode::enc::Encoder;
use bincode::error::{DecodeError, EncodeError};
use bincode::{Decode, Encode};

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub(crate) struct OwnedContractBaseWire34 {
    self_parameter: TypeParameterId,
    type_parameters: Vec<TypeParameterId>,
    methods: Vec<OwnedMethod>,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub(crate) struct OwnedImplementationBaseWire34 {
    type_parameters: Vec<TypeParameterId>,
    implementation_parameters: Vec<ImplementationParameter>,
    contract: DeclarationReference,
    self_type: TypeObjectDigest,
    type_arguments: Vec<TypeObjectDigest>,
    methods: Vec<OwnedMethodImplementation>,
}

impl OwnedContractBaseWire34 {
    fn fields(value: &OwnedContract) -> Self {
        Self {
            self_parameter: value.self_parameter,
            type_parameters: value.type_parameters.clone(),
            methods: value.methods.clone(),
        }
    }
    fn into_logical(self, associated_types: Vec<AssociatedTypeMember>) -> OwnedContract {
        OwnedContract {
            self_parameter: self.self_parameter,
            type_parameters: self.type_parameters,
            methods: self.methods,
            associated_types,
        }
    }
}
impl OwnedImplementationBaseWire34 {
    fn fields(value: &OwnedImplementation) -> Self {
        Self {
            type_parameters: value.type_parameters.clone(),
            implementation_parameters: value.implementation_parameters.clone(),
            contract: value.contract,
            self_type: value.self_type,
            type_arguments: value.type_arguments.clone(),
            methods: value.methods.clone(),
        }
    }
    fn into_logical(self, associated_types: Vec<AssociatedTypeBinding>) -> OwnedImplementation {
        OwnedImplementation {
            type_parameters: self.type_parameters,
            implementation_parameters: self.implementation_parameters,
            contract: self.contract,
            self_type: self.self_type,
            type_arguments: self.type_arguments,
            methods: self.methods,
            associated_types,
        }
    }
}

impl Encode for OwnedContract {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        if !self.associated_types.is_empty() {
            return Err(EncodeError::Other(
                "associated contract requires its explicit extension envelope",
            ));
        }
        (&self.self_parameter, &self.type_parameters, &self.methods).encode(encoder)
    }
}
impl<C> Decode<C> for OwnedContract {
    fn decode<D: Decoder<Context = C>>(decoder: &mut D) -> Result<Self, DecodeError> {
        Ok(OwnedContractBaseWire34::decode(decoder)?.into_logical(Vec::new()))
    }
}
bincode::impl_borrow_decode!(OwnedContract);
impl Encode for OwnedImplementation {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        if !self.associated_types.is_empty() {
            return Err(EncodeError::Other(
                "associated implementation requires its explicit extension envelope",
            ));
        }
        (
            &self.type_parameters,
            &self.implementation_parameters,
            &self.contract,
            &self.self_type,
            &self.type_arguments,
            &self.methods,
        )
            .encode(encoder)
    }
}
impl<C> Decode<C> for OwnedImplementation {
    fn decode<D: Decoder<Context = C>>(decoder: &mut D) -> Result<Self, DecodeError> {
        Ok(OwnedImplementationBaseWire34::decode(decoder)?.into_logical(Vec::new()))
    }
}
bincode::impl_borrow_decode!(OwnedImplementation);

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum AssociatedOwnedBody {
    Contract(OwnedContractBaseWire34, Vec<AssociatedTypeMember>),
    Implementation(OwnedImplementationBaseWire34, Vec<AssociatedTypeBinding>),
}

impl AssociatedOwnedBody {
    pub(crate) fn contract(value: &OwnedContract) -> Result<Self, Diagnostic> {
        value.validate_local()?;
        require_nonempty(value.associated_types.len())?;
        Ok(Self::Contract(
            OwnedContractBaseWire34::fields(value),
            value.associated_types.clone(),
        ))
    }
    pub(crate) fn implementation(value: &OwnedImplementation) -> Result<Self, Diagnostic> {
        value.validate_local()?;
        require_nonempty(value.associated_types.len())?;
        Ok(Self::Implementation(
            OwnedImplementationBaseWire34::fields(value),
            value.associated_types.clone(),
        ))
    }
    pub(crate) fn from_payload(payload: &DeclarationPayload) -> Result<Option<Self>, Diagnostic> {
        match payload {
            DeclarationPayload::OwnedContract(c) if !c.associated_types.is_empty() => {
                Self::contract(c).map(Some)
            }
            DeclarationPayload::OwnedImplementation(i) if !i.associated_types.is_empty() => {
                Self::implementation(i).map(Some)
            }
            _ => Ok(None),
        }
    }
    pub(crate) fn tag(&self) -> u8 {
        match self {
            Self::Contract(..) => 1,
            Self::Implementation(..) => 2,
        }
    }
    pub(crate) fn into_payload(self) -> DeclarationPayload {
        match self {
            Self::Contract(base, inventory) => {
                DeclarationPayload::OwnedContract(base.into_logical(inventory))
            }
            Self::Implementation(base, inventory) => {
                DeclarationPayload::OwnedImplementation(base.into_logical(inventory))
            }
        }
    }
    pub(crate) fn encode_data<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        match self {
            Self::Contract(base, inventory) => {
                base.encode(encoder)?;
                inventory.encode(encoder)
            }
            Self::Implementation(base, inventory) => {
                base.encode(encoder)?;
                inventory.encode(encoder)
            }
        }
    }
    pub(crate) fn decode_data<C, D: Decoder<Context = C>>(
        tag: u8,
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        let body = match tag {
            1 => Self::Contract(
                OwnedContractBaseWire34::decode(decoder)?,
                Vec::<AssociatedTypeMember>::decode(decoder)?,
            ),
            2 => Self::Implementation(
                OwnedImplementationBaseWire34::decode(decoder)?,
                Vec::<AssociatedTypeBinding>::decode(decoder)?,
            ),
            _ => return Err(DecodeError::Other("unknown associated owned extension tag")),
        };
        let count = match &body {
            Self::Contract(_, inventory) => inventory.len(),
            Self::Implementation(_, inventory) => inventory.len(),
        };
        require_nonempty(count).map_err(|error| DecodeError::OtherString(error.message))?;
        Ok(body)
    }
}
impl Encode for AssociatedOwnedBody {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        contract::ASSOCIATED_READ_VIEW_CONTRACT_VERSION.encode(encoder)?;
        self.tag().encode(encoder)?;
        self.encode_data(encoder)
    }
}
impl<C> Decode<C> for AssociatedOwnedBody {
    fn decode<D: Decoder<Context = C>>(decoder: &mut D) -> Result<Self, DecodeError> {
        decode_read_view_version(decoder)?;
        Self::decode_data(u8::decode(decoder)?, decoder)
    }
}
bincode::impl_borrow_decode!(AssociatedOwnedBody);

pub(crate) fn decode_version<C, D: Decoder<Context = C>>(
    decoder: &mut D,
) -> Result<(), DecodeError> {
    if u16::decode(decoder)? != contract::OWNED_ASSOCIATED_CONTRACT_VERSION {
        return Err(DecodeError::Other(
            "unknown associated owned contract version",
        ));
    }
    Ok(())
}
pub(crate) fn require_base_inventory(is_empty: bool) -> Result<(), Diagnostic> {
    if !is_empty {
        return Err(Diagnostic::new(
            DiagnosticClass::Semantic,
            "kernel_associated_encoding_generation",
            "associated inventory cannot be represented by a base or predecessor codec",
        ));
    }
    Ok(())
}
fn require_nonempty(count: usize) -> Result<(), Diagnostic> {
    if count == 0 || count > contract::MAXIMUM_CHILDREN {
        return Err(Diagnostic::new(
            DiagnosticClass::Semantic,
            "kernel_associated_inventory",
            "associated extension requires a bounded nonempty inventory",
        ));
    }
    Ok(())
}

pub(crate) struct AssociatedOwnerWire(pub DeclarationRecord);
impl Encode for AssociatedOwnerWire {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        let record = &self.0;
        if record.header.contract_version != contract::GRAPH_CONTRACT_VERSION {
            return Err(EncodeError::Other(
                "read-view associated owner requires the current graph profile",
            ));
        }
        let body = AssociatedOwnedBody::from_payload(&record.payload)
            .map_err(|e| EncodeError::OtherString(e.message))?
            .ok_or(EncodeError::Other("owner has no associated inventory"))?;
        contract::ASSOCIATED_READ_VIEW_CONTRACT_VERSION.encode(encoder)?;
        body.tag().encode(encoder)?;
        record.header.encode(encoder)?;
        record.module.encode(encoder)?;
        record.name.encode(encoder)?;
        record.visibility.encode(encoder)?;
        body.encode_data(encoder)
    }
}
impl<C> Decode<C> for AssociatedOwnerWire {
    fn decode<D: Decoder<Context = C>>(decoder: &mut D) -> Result<Self, DecodeError> {
        decode_read_view_version(decoder)?;
        let tag = u8::decode(decoder)?;
        let header = OwnerHeader::decode(decoder)?;
        if header.contract_version != contract::GRAPH_CONTRACT_VERSION {
            return Err(DecodeError::Other(
                "read-view associated owner requires the current graph profile",
            ));
        }
        let module = ModuleId::decode(decoder)?;
        let name = Name::decode(decoder)?;
        let visibility = DeclarationVisibility::decode(decoder)?;
        let payload = AssociatedOwnedBody::decode_data(tag, decoder)?.into_payload();
        Ok(Self(DeclarationRecord {
            header,
            module,
            name,
            visibility,
            payload,
        }))
    }
}
bincode::impl_borrow_decode!(AssociatedOwnerWire);

pub(crate) fn unsupported_semantics() -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Source,
        "owned_associated_semantics_unsupported",
        "associated owned types have a codec but semantic admission is not yet supported",
    )
}

pub(crate) struct AssociatedTypeWire(pub TypeObject);
impl Encode for AssociatedTypeWire {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        if self.0.contract_version != contract::OWNED_ASSOCIATED_CONTRACT_VERSION {
            return Err(EncodeError::Other(
                "unknown associated owned contract version",
            ));
        }
        self.0.contract_version.encode(encoder)?;
        match &self.0.form {
            TypeForm::AssociatedContractMember { contract, member } => {
                1_u8.encode(encoder)?;
                contract.encode(encoder)?;
                member.encode(encoder)
            }
            TypeForm::AssociatedWitnessProjection {
                scope,
                parameter,
                contract,
                member,
            } => {
                2_u8.encode(encoder)?;
                scope.encode(encoder)?;
                parameter.encode(encoder)?;
                contract.encode(encoder)?;
                member.encode(encoder)
            }
            _ => Err(EncodeError::Other("type has no associated form")),
        }
    }
}
impl<C> Decode<C> for AssociatedTypeWire {
    fn decode<D: Decoder<Context = C>>(decoder: &mut D) -> Result<Self, DecodeError> {
        decode_version(decoder)?;
        let form = match u8::decode(decoder)? {
            1 => TypeForm::AssociatedContractMember {
                contract: DeclarationReference::decode(decoder)?,
                member: AssociatedTypeMemberId::decode(decoder)?,
            },
            2 => TypeForm::AssociatedWitnessProjection {
                scope: DeclarationReference::decode(decoder)?,
                parameter: ImplementationParameterId::decode(decoder)?,
                contract: DeclarationReference::decode(decoder)?,
                member: AssociatedTypeMemberId::decode(decoder)?,
            },
            _ => return Err(DecodeError::Other("unknown associated type tag")),
        };
        Ok(Self(TypeObject {
            contract_version: contract::OWNED_ASSOCIATED_CONTRACT_VERSION,
            form,
        }))
    }
}
bincode::impl_borrow_decode!(AssociatedTypeWire);

pub(crate) fn decode_read_view_version<C, D: Decoder<Context = C>>(
    decoder: &mut D,
) -> Result<(), DecodeError> {
    if u16::decode(decoder)? != contract::ASSOCIATED_READ_VIEW_CONTRACT_VERSION {
        return Err(DecodeError::Other(
            "unknown associated read-view contract version",
        ));
    }
    Ok(())
}

/// The original associated-owned body, including the original method and binding fields.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum FrozenAssociatedOwnedBody33 {
    Contract(OwnedContract33, Vec<AssociatedTypeMember33>),
    Implementation(OwnedImplementation33, Vec<AssociatedTypeBinding33>),
}

impl FrozenAssociatedOwnedBody33 {
    pub(crate) fn contract(value: &OwnedContract) -> Result<Self, Diagnostic> {
        value.validate_local()?;
        require_nonempty(value.associated_types.len())?;
        Ok(Self::Contract(
            OwnedContract33::fields(value)?,
            value
                .associated_types
                .iter()
                .cloned()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
        ))
    }
    pub(crate) fn implementation(value: &OwnedImplementation) -> Result<Self, Diagnostic> {
        value.validate_local()?;
        require_nonempty(value.associated_types.len())?;
        Ok(Self::Implementation(
            OwnedImplementation33::fields(value)?,
            value
                .associated_types
                .iter()
                .cloned()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
        ))
    }
    pub(crate) fn from_payload(payload: &DeclarationPayload) -> Result<Option<Self>, Diagnostic> {
        match payload {
            DeclarationPayload::OwnedContract(value) if !value.associated_types.is_empty() => {
                Self::contract(value).map(Some)
            }
            DeclarationPayload::OwnedImplementation(value)
                if !value.associated_types.is_empty() =>
            {
                Self::implementation(value).map(Some)
            }
            _ => Ok(None),
        }
    }
    pub(crate) fn tag(&self) -> u8 {
        match self {
            Self::Contract(..) => 1,
            Self::Implementation(..) => 2,
        }
    }
    pub(crate) fn into_payload(self) -> DeclarationPayload {
        match self {
            Self::Contract(base, inventory) => DeclarationPayload::OwnedContract(
                base.into_logical(inventory.into_iter().map(Into::into).collect()),
            ),
            Self::Implementation(base, inventory) => DeclarationPayload::OwnedImplementation(
                base.into_logical(inventory.into_iter().map(Into::into).collect()),
            ),
        }
    }
    pub(crate) fn encode_data<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        match self {
            Self::Contract(base, inventory) => {
                base.encode(encoder)?;
                inventory.encode(encoder)
            }
            Self::Implementation(base, inventory) => {
                base.encode(encoder)?;
                inventory.encode(encoder)
            }
        }
    }
    pub(crate) fn decode_data<C, D: Decoder<Context = C>>(
        tag: u8,
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        let body = match tag {
            1 => Self::Contract(
                OwnedContract33::decode(decoder)?,
                Vec::<AssociatedTypeMember33>::decode(decoder)?,
            ),
            2 => Self::Implementation(
                OwnedImplementation33::decode(decoder)?,
                Vec::<AssociatedTypeBinding33>::decode(decoder)?,
            ),
            _ => return Err(DecodeError::Other("unknown associated owned extension tag")),
        };
        let count = match &body {
            Self::Contract(_, inventory) => inventory.len(),
            Self::Implementation(_, inventory) => inventory.len(),
        };
        require_nonempty(count).map_err(|error| DecodeError::OtherString(error.message))?;
        Ok(body)
    }
}

impl Encode for FrozenAssociatedOwnedBody33 {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        contract::OWNED_ASSOCIATED_CONTRACT_VERSION.encode(encoder)?;
        self.tag().encode(encoder)?;
        self.encode_data(encoder)
    }
}
impl<C> Decode<C> for FrozenAssociatedOwnedBody33 {
    fn decode<D: Decoder<Context = C>>(decoder: &mut D) -> Result<Self, DecodeError> {
        decode_version(decoder)?;
        Self::decode_data(u8::decode(decoder)?, decoder)
    }
}
bincode::impl_borrow_decode!(FrozenAssociatedOwnedBody33);

pub(crate) struct FrozenAssociatedOwnerWire33(pub DeclarationRecord);

impl Encode for FrozenAssociatedOwnerWire33 {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        let record = &self.0;
        if record.header.contract_version != 33 {
            return Err(EncodeError::Other(
                "associated owned owner requires the Graph 33 base profile",
            ));
        }
        let body = FrozenAssociatedOwnedBody33::from_payload(&record.payload)
            .map_err(|error| EncodeError::OtherString(error.message))?
            .ok_or(EncodeError::Other("owner has no associated inventory"))?;
        contract::OWNED_ASSOCIATED_CONTRACT_VERSION.encode(encoder)?;
        body.tag().encode(encoder)?;
        record.header.encode(encoder)?;
        record.module.encode(encoder)?;
        record.name.encode(encoder)?;
        record.visibility.encode(encoder)?;
        body.encode_data(encoder)
    }
}
impl<C> Decode<C> for FrozenAssociatedOwnerWire33 {
    fn decode<D: Decoder<Context = C>>(decoder: &mut D) -> Result<Self, DecodeError> {
        decode_version(decoder)?;
        let tag = u8::decode(decoder)?;
        let header = OwnerHeader::decode(decoder)?;
        if header.contract_version != 33 {
            return Err(DecodeError::Other(
                "associated owned owner requires the Graph 33 base profile",
            ));
        }
        let module = ModuleId::decode(decoder)?;
        let name = Name::decode(decoder)?;
        let visibility = DeclarationVisibility::decode(decoder)?;
        let payload = FrozenAssociatedOwnedBody33::decode_data(tag, decoder)?.into_payload();
        Ok(Self(DeclarationRecord {
            header,
            module,
            name,
            visibility,
            payload,
        }))
    }
}
bincode::impl_borrow_decode!(FrozenAssociatedOwnerWire33);

/// Disjoint current-capability envelope for read-view applications and family projections.
pub(crate) struct ReadViewTypeWire(pub TypeObject);

impl Encode for ReadViewTypeWire {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        if self.0.contract_version != contract::ASSOCIATED_READ_VIEW_CONTRACT_VERSION {
            return Err(EncodeError::Other(
                "unknown associated read-view contract version",
            ));
        }
        self.0.contract_version.encode(encoder)?;
        match &self.0.form {
            TypeForm::ReadViewApplied {
                declaration,
                arguments,
                read_scope,
            } => {
                1_u8.encode(encoder)?;
                declaration.encode(encoder)?;
                arguments.encode(encoder)?;
                read_scope.encode(encoder)
            }
            TypeForm::AssociatedReadViewMember {
                contract,
                member,
                read_scope,
            } => {
                2_u8.encode(encoder)?;
                contract.encode(encoder)?;
                member.encode(encoder)?;
                read_scope.encode(encoder)
            }
            TypeForm::AssociatedReadViewProjection {
                scope,
                parameter,
                contract,
                member,
                read_scope,
            } => {
                3_u8.encode(encoder)?;
                scope.encode(encoder)?;
                parameter.encode(encoder)?;
                contract.encode(encoder)?;
                member.encode(encoder)?;
                read_scope.encode(encoder)
            }
            _ => Err(EncodeError::Other("type has no read-view form")),
        }
    }
}
impl<C> Decode<C> for ReadViewTypeWire {
    fn decode<D: Decoder<Context = C>>(decoder: &mut D) -> Result<Self, DecodeError> {
        decode_read_view_version(decoder)?;
        let form = match u8::decode(decoder)? {
            1 => TypeForm::ReadViewApplied {
                declaration: DeclarationReference::decode(decoder)?,
                arguments: Vec::<TypeObjectDigest>::decode(decoder)?,
                read_scope: ReadScopeArgument::decode(decoder)?,
            },
            2 => TypeForm::AssociatedReadViewMember {
                contract: DeclarationReference::decode(decoder)?,
                member: AssociatedTypeMemberId::decode(decoder)?,
                read_scope: ReadScopeArgument::decode(decoder)?,
            },
            3 => TypeForm::AssociatedReadViewProjection {
                scope: DeclarationReference::decode(decoder)?,
                parameter: ImplementationParameterId::decode(decoder)?,
                contract: DeclarationReference::decode(decoder)?,
                member: AssociatedTypeMemberId::decode(decoder)?,
                read_scope: ReadScopeArgument::decode(decoder)?,
            },
            _ => return Err(DecodeError::Other("unknown read-view type tag")),
        };
        Ok(Self(TypeObject {
            contract_version: contract::ASSOCIATED_READ_VIEW_CONTRACT_VERSION,
            form,
        }))
    }
}
bincode::impl_borrow_decode!(ReadViewTypeWire);
