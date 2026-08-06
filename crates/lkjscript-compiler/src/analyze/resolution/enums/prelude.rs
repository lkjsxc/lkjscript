use crate::analyze::*;

impl Resolver<'_> {
    pub(in crate::analyze::resolution) fn prelude_operation(
        &self,
        operation: Operation,
        arguments: Vec<Expr>,
    ) -> Option<ExprKind> {
        let argument = arguments.first().cloned().map(Box::new);
        let result = (
            EnumId::new(lkjscript_core::RESULT_ID),
            crate::types::prelude_layout(lkjscript_core::PreludeEnum::Result),
        );
        let option = (
            EnumId::new(lkjscript_core::OPTION_ID),
            crate::types::prelude_layout(lkjscript_core::PreludeEnum::Option),
        );
        match operation {
            Operation::Ok => Some(ExprKind::EnumValue {
                enum_id: result.0,
                variant: VariantId::new(lkjscript_core::RESULT_OK_ID),
                layout: result.1,
                fields: arguments,
            }),
            Operation::Err => Some(ExprKind::EnumValue {
                enum_id: result.0,
                variant: VariantId::new(lkjscript_core::RESULT_ERR_ID),
                layout: result.1,
                fields: arguments,
            }),
            Operation::Some => Some(ExprKind::EnumValue {
                enum_id: option.0,
                variant: VariantId::new(lkjscript_core::OPTION_SOME_ID),
                layout: option.1,
                fields: arguments,
            }),
            Operation::IsOk => Some(ExprKind::EnumIsVariant {
                enum_id: result.0,
                variant: VariantId::new(lkjscript_core::RESULT_OK_ID),
                layout: result.1,
                value: argument.clone()?,
            }),
            Operation::IsSome => Some(ExprKind::EnumIsVariant {
                enum_id: option.0,
                variant: VariantId::new(lkjscript_core::OPTION_SOME_ID),
                layout: option.1,
                value: argument.clone()?,
            }),
            Operation::UnwrapOk => Some(enum_unwrap(
                result,
                lkjscript_core::RESULT_OK_ID,
                lkjscript_core::RESULT_OK_VALUE_ID,
                argument.clone()?,
                "unwrap-ok on Err",
            )),
            Operation::UnwrapErr => Some(enum_unwrap(
                result,
                lkjscript_core::RESULT_ERR_ID,
                lkjscript_core::RESULT_ERR_ERROR_ID,
                argument.clone()?,
                "unwrap-err on Ok",
            )),
            Operation::UnwrapSome => Some(enum_unwrap(
                option,
                lkjscript_core::OPTION_SOME_ID,
                lkjscript_core::OPTION_VALUE_ID,
                argument?,
                "unwrap-some on none",
            )),
            _ => None,
        }
    }
}

fn enum_unwrap(
    identity: (EnumId, RuntimeLayoutId),
    variant: [u8; 32],
    field: [u8; 32],
    value: Box<Expr>,
    trap: &str,
) -> ExprKind {
    ExprKind::EnumUnwrap {
        enum_id: identity.0,
        variant: VariantId::new(variant),
        field: VariantFieldId::new(field),
        field_index: 0,
        layout: identity.1,
        value,
        trap: trap.into(),
    }
}
