use crate::analyze::*;
use lkjscript_core::{NumericError, PreludeEnum, SystemErrorKind, Utf8ErrorKind};

impl Analyzer {
    pub(in crate::analyze) fn install_prelude_enums(&mut self) -> Result<()> {
        self.install_option();
        self.install_result();
        self.install_numeric_error();
        self.install_utf8_error();
        self.install_system_error();
        Ok(())
    }

    fn install_option(&mut self) {
        let parameter = Type::Param("T".into());
        self.push_prelude(
            PreludeEnum::Option,
            vec!["T".into()],
            vec![
                variant(lkjscript_core::OPTION_NONE_ID, "None", Vec::new(), 0),
                variant(
                    lkjscript_core::OPTION_SOME_ID,
                    "Some",
                    vec![field(
                        lkjscript_core::OPTION_VALUE_ID,
                        "value",
                        parameter,
                        false,
                    )],
                    1,
                ),
            ],
        );
    }

    fn install_result(&mut self) {
        self.push_prelude(
            PreludeEnum::Result,
            vec!["T".into(), "E".into()],
            vec![
                variant(
                    lkjscript_core::RESULT_OK_ID,
                    "Ok",
                    vec![field(
                        lkjscript_core::RESULT_OK_VALUE_ID,
                        "value",
                        Type::Param("T".into()),
                        false,
                    )],
                    0,
                ),
                variant(
                    lkjscript_core::RESULT_ERR_ID,
                    "Err",
                    vec![field(
                        lkjscript_core::RESULT_ERR_ERROR_ID,
                        "error",
                        Type::Param("E".into()),
                        false,
                    )],
                    1,
                ),
            ],
        );
    }

    fn install_numeric_error(&mut self) {
        let variants = [
            NumericError::NonFinite,
            NumericError::OutOfRange,
            NumericError::Fractional,
            NumericError::Inexact,
        ]
        .into_iter()
        .enumerate()
        .map(|(order, error)| variant(error.variant_id(), error.name(), Vec::new(), order))
        .collect();
        self.push_prelude(PreludeEnum::NumericError, Vec::new(), variants);
    }

    fn install_utf8_error(&mut self) {
        let variants = Utf8ErrorKind::ALL
            .into_iter()
            .enumerate()
            .map(|(order, error)| {
                let field_id =
                    crate::source::enum_member_identity(error.variant_id(), "field", "offset");
                variant(
                    error.variant_id(),
                    error.name(),
                    vec![field(field_id, "offset", Type::I64, false)],
                    order,
                )
            })
            .collect();
        self.push_prelude(PreludeEnum::Utf8Error, Vec::new(), variants);
    }

    fn install_system_error(&mut self) {
        let variants = SystemErrorKind::ALL
            .into_iter()
            .enumerate()
            .map(|(order, error)| {
                let fields = if error == SystemErrorKind::Utf8 {
                    vec![derived_field(
                        error,
                        "error",
                        crate::types::utf8_error_type(),
                    )]
                } else {
                    vec![
                        derived_field(error, "code", crate::types::option_type(Type::I64)),
                        derived_field(error, "detail", crate::types::option_type(Type::Str)),
                    ]
                };
                variant(error.variant_id(), error.name(), fields, order)
            })
            .collect();
        self.push_prelude(PreludeEnum::SystemError, Vec::new(), variants);
    }

    fn push_prelude(
        &mut self,
        kind: PreludeEnum,
        parameters: Vec<String>,
        variants: Vec<EnumVariant>,
    ) {
        let Type::Enum { id, name, .. } = crate::types::prelude_type(kind, Vec::new()) else {
            unreachable!("prelude type must be nominal")
        };
        self.enum_headers
            .insert(name.clone(), (id, parameters.clone()));
        self.enums.push(EnumDefinition {
            id,
            name,
            origin: SourceId::new(u32::MAX),
            type_parameters: parameters,
            variants,
            layout: EnumLayoutFacts {
                identity: crate::types::prelude_layout(kind),
                recursive: false,
            },
        });
    }
}

fn variant(id: [u8; 32], name: &str, fields: Vec<EnumVariantField>, order: usize) -> EnumVariant {
    EnumVariant {
        id: VariantId::new(id),
        name: name.into(),
        source_order: u16::try_from(order).unwrap_or(u16::MAX),
        fields,
    }
}

fn derived_field(kind: SystemErrorKind, name: &str, ty: Type) -> EnumVariantField {
    let id = crate::source::enum_member_identity(kind.variant_id(), "field", name);
    field(id, name, ty, true)
}

fn field(id: [u8; 32], name: &str, ty: Type, indirect: bool) -> EnumVariantField {
    EnumVariantField {
        id: VariantFieldId::new(id),
        name: name.into(),
        source_order: 0,
        indirect,
        traced: matches!(
            ty,
            Type::Str
                | Type::Symbol
                | Type::Buf
                | Type::Path
                | Type::Product(_)
                | Type::Enum { .. }
                | Type::List(_)
                | Type::Fn { .. }
                | Type::Forall { .. }
        ),
        ty,
    }
}
