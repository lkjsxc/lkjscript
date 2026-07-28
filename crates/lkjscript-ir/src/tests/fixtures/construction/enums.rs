use super::*;

pub(crate) fn enum_metadata() -> EnumMetadata {
    EnumMetadata {
        id: EnumId::new([1; 32]),
        name: "Boxed".into(),
        type_parameters: vec!["t".into()],
        variants: vec![
            EnumVariantMetadata {
                id: VariantId::new([2; 32]),
                name: "a".into(),
                physical_tag: 1,
                fields: vec![EnumFieldMetadata {
                    id: VariantFieldId::new([3; 32]),
                    name: "value".into(),
                    ty: SsaType::TypeParameter("t".into()),
                    indirect: false,
                    traced: false,
                }],
            },
            EnumVariantMetadata {
                id: VariantId::new([4; 32]),
                name: "b".into(),
                physical_tag: 0,
                fields: vec![EnumFieldMetadata {
                    id: VariantFieldId::new([5; 32]),
                    name: "value".into(),
                    ty: SsaType::TypeParameter("t".into()),
                    indirect: false,
                    traced: false,
                }],
            },
        ],
        layout: EnumLayoutFacts {
            identity: RuntimeLayoutId::new([6; 32]),
            recursive: false,
        },
    }
}

pub(crate) fn enum_type() -> SsaType {
    SsaType::Enum {
        id: EnumId::new([1; 32]),
        arguments: vec![SsaType::I64],
    }
}
