//! Progressive owner and shared kernel payload decoding.
//!
//! All variable fields consume an encoded element before fallible growth. The
//! existing Encode implementations remain the canonical byte authority. Flat
//! owner records contain typed identities rather than recursive expression trees;
//! implementation operands retain their existing recursive profile admission.

use super::super::*;
use crate::platform::packed::linear::{
    LinearDecode, decode_string, decode_vec, decode_vec_len_with, decode_vec_with,
};
use bincode::de::Decoder;
use bincode::error::DecodeError;
use bincode::{Decode, Encode};

macro_rules! linear_record {
    ($name:ident { $($field:ident = $read:expr),* $(,)? }) => {
        impl $name {
            pub(crate) fn decode_linear<Context, D: Decoder<Context = Context>>(
                decoder: &mut D,
            ) -> Result<Self, DecodeError> {
                Ok(Self { $($field: ($read)(decoder)?),* })
            }
        }
    };
}

mod expression;
mod owner;

#[cfg(test)]
mod tests;

/// The wrapper changes allocation behavior, never the OwnerRecord wire schema.
#[derive(Encode)]
pub(super) struct LinearOwner(pub OwnerRecord);

impl<Context> Decode<Context> for LinearOwner {
    fn decode<D: Decoder<Context = Context>>(decoder: &mut D) -> Result<Self, DecodeError> {
        OwnerRecord::decode_linear(decoder).map(Self)
    }
}

impl LinearDecode for LinearOwner {}

impl Name {
    pub(crate) fn decode_linear<Context, D: Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        Self::new(decode_string(decoder)?).map_err(|error| DecodeError::OtherString(error.message))
    }
}

impl ImplementationName {
    pub(crate) fn decode_linear<Context, D: Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        Self::new(decode_string(decoder)?).map_err(|error| DecodeError::OtherString(error.message))
    }
}

linear_record!(EffectRow {
    requirements = decode_vec,
    parameters = decode_vec,
});
linear_record!(RequirementConstraint {
    interface = Decode::decode,
    operations = decode_vec,
});
linear_record!(ReadScopeParameter {
    id = Decode::decode,
    name = Name::decode_linear,
});
linear_record!(ReadViewDeclaration {
    type_parameters = decode_vec,
    scope_parameter = ReadScopeParameter::decode_linear,
    fields = |decoder| decode_vec_with(decoder, ReadViewField::decode_linear),
});
linear_record!(ReadViewField {
    id = Decode::decode,
    name = Name::decode_linear,
    ty = Decode::decode,
    kind = Decode::decode,
});
linear_record!(OwnedContract {
    self_parameter = Decode::decode,
    type_parameters = decode_vec,
    methods = |decoder| decode_vec_with(decoder, OwnedMethod::decode_linear),
    associated_types = |decoder| decode_vec_with(decoder, AssociatedTypeMember::decode_linear),
});
linear_record!(OwnedMethod {
    id = Decode::decode,
    name = Name::decode_linear,
    parameters = decode_vec,
    result = Decode::decode,
    read_scope_parameters = |decoder| decode_vec_with(decoder, ReadScopeParameter::decode_linear),
    read_scope_sources = decode_vec,
    effect = FunctionEffect::decode_linear,
});
linear_record!(OwnedImplementation {
    type_parameters = decode_vec,
    implementation_parameters = |decoder| decode_vec_with(decoder, ImplementationParameter::decode_linear),
    contract = Decode::decode,
    self_type = Decode::decode,
    type_arguments = decode_vec,
    methods = |decoder| decode_vec_with(decoder, OwnedMethodImplementation::decode_linear),
    associated_types = |decoder| decode_vec_with(decoder, AssociatedTypeBinding::decode_linear),
});
linear_record!(AssociatedTypeMember {
    id = Decode::decode,
    name = Name::decode_linear,
    bound = AssociatedTypeBound::decode_linear,
});
linear_record!(AssociatedTypeBinding {
    member = Decode::decode,
    value = AssociatedTypeBindingValue::decode_linear,
});
linear_record!(OwnedMethodImplementation {
    method = Decode::decode,
    function = Decode::decode,
    type_arguments = decode_vec,
    implementations = |decoder| decode_vec_with(decoder, ImplementationOperand::decode_linear),
    read_scope_arguments = decode_vec,
});
linear_record!(ImplementationParameter {
    id = Decode::decode,
    name = Name::decode_linear,
    contract = Decode::decode,
    self_type = Decode::decode,
    type_arguments = decode_vec,
});

impl AssociatedTypeBound {
    pub(crate) fn decode_linear<Context, D: Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        match u32::decode(decoder)? {
            0 => Ok(Self::Owned),
            1 => Ok(Self::ReadViewFamily {
                scope_parameter: ReadScopeParameter::decode_linear(decoder)?,
            }),
            _ => Err(DecodeError::Other("unknown associated type bound tag")),
        }
    }
}

impl AssociatedTypeBindingValue {
    pub(crate) fn decode_linear<Context, D: Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        match u32::decode(decoder)? {
            0 => Ok(Self::Owned {
                ty: Decode::decode(decoder)?,
            }),
            1 => Ok(Self::ReadViewFamily {
                scope_parameter: ReadScopeParameter::decode_linear(decoder)?,
                ty: Decode::decode(decoder)?,
            }),
            _ => Err(DecodeError::Other("unknown associated type binding tag")),
        }
    }
}

impl ImplementationOperand {
    pub(crate) fn decode_linear<Context, D: Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        fn operand<Context, D: Decoder<Context = Context>>(
            decoder: &mut D,
            depth: usize,
            remaining: &mut usize,
        ) -> Result<ImplementationOperand, DecodeError> {
            match u32::decode(decoder)? {
                0 => {
                    let implementation = Decode::decode(decoder)?;
                    let type_count = usize::decode(decoder)?;
                    if type_count > contract::MAXIMUM_CHILDREN {
                        return Err(DecodeError::Other(
                            "implementation argument count exceeds its bound",
                        ));
                    }
                    let type_arguments = decode_vec_len_with(decoder, type_count, Decode::decode)?;
                    let length = usize::decode(decoder)?;
                    if length > contract::MAXIMUM_CHILDREN || length > *remaining {
                        return Err(DecodeError::Other(
                            "implementation prerequisite count exceeds its bound",
                        ));
                    }
                    *remaining -= length;
                    let implementations = decode_vec_len_with(decoder, length, |decoder| {
                        operand(decoder, depth + 1, remaining)
                    })?;
                    Ok(ImplementationOperand::Concrete {
                        implementation,
                        type_arguments,
                        implementations,
                    })
                }
                1 => Ok(ImplementationOperand::Parameter {
                    scope: Decode::decode(decoder)?,
                    parameter: Decode::decode(decoder)?,
                }),
                _ => Err(DecodeError::Other("unknown implementation operand tag")),
            }
        }
        // These guards belong to the retained recursive operand profile. Removing
        // them requires its Encode/Clone/comparison/drop lifecycle to change too.
        let mut remaining = contract::MAXIMUM_CHILDREN - 1;
        operand(decoder, 0, &mut remaining)
    }
}

impl HttpRouteSelector {
    pub(crate) fn decode_linear<Context, D: Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        let selector = match u8::decode(decoder)? {
            0 => Self::Exact {
                path: decode_http_text(
                    decoder,
                    contract::MAXIMUM_HTTP_ROUTE_PATH_BYTES,
                    "HTTP exact route path",
                )?,
            },
            1 => {
                let length = usize::from(u8::decode(decoder)?);
                if length == 0 || length > contract::MAXIMUM_HTTP_PATTERN_SEGMENTS {
                    return Err(DecodeError::OtherString(format!(
                        "HTTP route pattern segment count must be 1 through {} before allocation",
                        contract::MAXIMUM_HTTP_PATTERN_SEGMENTS
                    )));
                }
                Self::Pattern {
                    segments: decode_vec_len_with(
                        decoder,
                        length,
                        HttpRoutePatternSegment::decode_linear,
                    )?,
                }
            }
            tag => {
                return Err(DecodeError::OtherString(format!(
                    "unknown HTTP route selector tag {tag}"
                )));
            }
        };
        selector
            .validate_local()
            .map_err(|error| DecodeError::OtherString(error.message))?;
        Ok(selector)
    }
}

impl HttpRoutePatternSegment {
    fn decode_linear<Context, D: Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        match u8::decode(decoder)? {
            0 => Ok(Self::Literal(decode_http_text(
                decoder,
                contract::MAXIMUM_HTTP_ROUTE_PATH_BYTES,
                "HTTP route pattern literal",
            )?)),
            1 => Name::new(decode_http_text(
                decoder,
                contract::MAXIMUM_NAME_BYTES,
                "HTTP route capture name",
            )?)
            .map(Self::Capture)
            .map_err(|error| DecodeError::OtherString(error.message)),
            tag => Err(DecodeError::OtherString(format!(
                "unknown HTTP route pattern segment tag {tag}"
            ))),
        }
    }
}

fn decode_http_text<Context, D: Decoder<Context = Context>>(
    decoder: &mut D,
    maximum: usize,
    label: &'static str,
) -> Result<String, DecodeError> {
    let length = usize::from(u16::decode(decoder)?);
    if length > maximum {
        return Err(DecodeError::OtherString(format!(
            "{label} exceeds {maximum} bytes before allocation"
        )));
    }
    let bytes = decode_vec_len_with(decoder, length, u8::decode)?;
    String::from_utf8(bytes).map_err(|error| DecodeError::OtherString(error.to_string()))
}
