//! Progressive allocation for every current owner payload.

use super::*;

linear_record!(ModuleRecord {
    header = Decode::decode,
    name = Name::decode_linear,
});
linear_record!(DeclarationRecord {
    header = Decode::decode,
    module = Decode::decode,
    name = Name::decode_linear,
    visibility = Decode::decode,
    payload = DeclarationPayload::decode_linear,
});
linear_record!(TypeParameterRecord {
    header = Decode::decode,
    declaration = Decode::decode,
    name = Name::decode_linear,
    constraints = Decode::decode,
});
linear_record!(EffectParameterRecord {
    header = Decode::decode,
    declaration = Decode::decode,
    name = Name::decode_linear,
});
linear_record!(FieldRecord {
    header = Decode::decode,
    declaration = Decode::decode,
    name = Name::decode_linear,
    ty = Decode::decode,
});
linear_record!(CaseRecord {
    header = Decode::decode,
    declaration = Decode::decode,
    name = Name::decode_linear,
    payload = Decode::decode,
});
linear_record!(OperationRecord {
    header = Decode::decode,
    declaration = Decode::decode,
    name = Name::decode_linear,
    parameters = decode_vec,
    result = Decode::decode,
    idempotency = Decode::decode,
    external_visibility = Decode::decode,
});
linear_record!(ParameterRecord {
    header = Decode::decode,
    parent = Decode::decode,
    name = Name::decode_linear,
    ty = Decode::decode,
    use_mode = Decode::decode,
    resource_requirement = Decode::decode,
});
linear_record!(BindingRecord {
    header = Decode::decode,
    name = Name::decode_linear,
    kind = Decode::decode,
    value = Decode::decode,
    declared_type = Decode::decode,
});
linear_record!(RequirementRecord {
    header = Decode::decode,
    declaration = Decode::decode,
    name = Name::decode_linear,
    interface = Decode::decode,
    operations = decode_vec,
    limits = |decoder| decode_vec_with(decoder, ResourceLimit::decode_linear),
});
linear_record!(ResourceLimit {
    name = Name::decode_linear,
    maximum = Decode::decode,
    unit = Decode::decode,
});
linear_record!(PortRecord {
    header = Decode::decode,
    declaration = Decode::decode,
    name = Name::decode_linear,
    function_type = Decode::decode,
    implementation = Decode::decode,
});
linear_record!(TargetRecord {
    header = Decode::decode,
    name = Name::decode_linear,
    component = Decode::decode,
    port = Decode::decode,
    runner = Decode::decode,
});
linear_record!(DocumentationRecord {
    header = Decode::decode,
    owner = Decode::decode,
    class = Decode::decode,
    content = DocumentContent::decode_linear,
});
linear_record!(AnnotationRecord {
    header = Decode::decode,
    owner = Decode::decode,
    class = Decode::decode,
    key = Name::decode_linear,
    value = AnnotationValue::decode_linear,
});
linear_record!(HttpRouteRecord {
    header = Decode::decode,
    target = Decode::decode,
    method = decode_string,
    selector = HttpRouteSelector::decode_linear,
    port = Decode::decode,
});
linear_record!(RequirementParameterRecord {
    header = Decode::decode,
    declaration = Decode::decode,
    name = Name::decode_linear,
    constraint = RequirementConstraint::decode_linear,
});
linear_record!(ExternalDeclaration {
    type_parameters = decode_vec,
    parameters = decode_vec,
    result = Decode::decode,
    implementation = ImplementationName::decode_linear,
});
linear_record!(FunctionDeclaration {
    implementation_parameters = |decoder| decode_vec_with(decoder, ImplementationParameter::decode_linear),
    requirement_parameters = decode_vec,
    effect_parameters = decode_vec,
    type_parameters = decode_vec,
    parameters = decode_vec,
    result = Decode::decode,
    read_scope_parameters = |decoder| decode_vec_with(decoder, ReadScopeParameter::decode_linear),
    read_scope_sources = decode_vec,
    effect = FunctionEffect::decode_linear,
    body = Decode::decode,
});

impl OwnerRecord {
    pub(crate) fn decode_linear<Context, D: Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        Ok(match u32::decode(decoder)? {
            0 => Self::Module(ModuleRecord::decode_linear(decoder)?),
            1 => Self::Declaration(DeclarationRecord::decode_linear(decoder)?),
            2 => Self::TypeParameter(TypeParameterRecord::decode_linear(decoder)?),
            3 => Self::EffectParameter(EffectParameterRecord::decode_linear(decoder)?),
            4 => Self::Field(FieldRecord::decode_linear(decoder)?),
            5 => Self::Case(CaseRecord::decode_linear(decoder)?),
            6 => Self::Operation(OperationRecord::decode_linear(decoder)?),
            7 => Self::Parameter(ParameterRecord::decode_linear(decoder)?),
            8 => Self::Binding(BindingRecord::decode_linear(decoder)?),
            9 => Self::Expression(ExpressionRecord::decode_linear(decoder)?),
            10 => Self::Requirement(RequirementRecord::decode_linear(decoder)?),
            11 => Self::Port(PortRecord::decode_linear(decoder)?),
            12 => Self::Target(TargetRecord::decode_linear(decoder)?),
            13 => Self::Documentation(DocumentationRecord::decode_linear(decoder)?),
            14 => Self::Annotation(AnnotationRecord::decode_linear(decoder)?),
            15 => Self::HttpRoute(HttpRouteRecord::decode_linear(decoder)?),
            16 => Self::RequirementParameter(RequirementParameterRecord::decode_linear(decoder)?),
            _ => return Err(DecodeError::Other("unknown owner record tag")),
        })
    }
}

impl DeclarationPayload {
    pub(crate) fn decode_linear<Context, D: Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        Ok(match u32::decode(decoder)? {
            0 => Self::Record {
                type_parameters: decode_vec(decoder)?,
                fields: decode_vec(decoder)?,
            },
            1 => Self::Variant {
                type_parameters: decode_vec(decoder)?,
                cases: decode_vec(decoder)?,
            },
            2 => Self::Interface {
                operations: decode_vec(decoder)?,
            },
            3 => Self::External(ExternalDeclaration::decode_linear(decoder)?),
            4 => Self::Function(FunctionDeclaration::decode_linear(decoder)?),
            5 => Self::Constant {
                ty: Decode::decode(decoder)?,
                value: Decode::decode(decoder)?,
            },
            6 => Self::Component {
                requirements: decode_vec(decoder)?,
                ports: decode_vec(decoder)?,
            },
            7 => Self::Test {
                actual: Decode::decode(decoder)?,
                expected: Decode::decode(decoder)?,
                comparison: Decode::decode(decoder)?,
            },
            8 => Self::OwnedContract(OwnedContract::decode_linear(decoder)?),
            9 => Self::OwnedImplementation(OwnedImplementation::decode_linear(decoder)?),
            10 => Self::OwnedRecord {
                type_parameters: decode_vec(decoder)?,
                fields: decode_vec(decoder)?,
            },
            11 => Self::ReadView(ReadViewDeclaration::decode_linear(decoder)?),
            _ => return Err(DecodeError::Other("unknown declaration payload tag")),
        })
    }
}

impl FunctionEffect {
    pub(crate) fn decode_linear<Context, D: Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        match u32::decode(decoder)? {
            0 => Ok(Self::Pure),
            1 => Ok(Self::Task {
                requirements: decode_vec(decoder)?,
                effect_parameters: decode_vec(decoder)?,
            }),
            _ => Err(DecodeError::Other("unknown function effect tag")),
        }
    }
}

impl DocumentContent {
    pub(crate) fn decode_linear<Context, D: Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        match u32::decode(decoder)? {
            0 => Ok(Self::Inline(decode_string(decoder)?)),
            1 => Ok(Self::Blob {
                digest: Decode::decode(decoder)?,
                bytes: Decode::decode(decoder)?,
            }),
            _ => Err(DecodeError::Other("unknown document content tag")),
        }
    }
}

impl AnnotationValue {
    pub(crate) fn decode_linear<Context, D: Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        match u32::decode(decoder)? {
            0 => Ok(Self::Bool(Decode::decode(decoder)?)),
            1 => Ok(Self::I64(Decode::decode(decoder)?)),
            2 => Ok(Self::Text(decode_string(decoder)?)),
            3 => Ok(Self::Name(Name::decode_linear(decoder)?)),
            _ => Err(DecodeError::Other("unknown annotation value tag")),
        }
    }
}
