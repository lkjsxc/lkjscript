use crate::analyze::*;
use lkjscript_core::NumericError;

impl Analyzer {
    pub(in crate::analyze) fn finish_numeric_error(&mut self) {
        if self.edition2 && self.enums.len() > 1 {
            self.enums.rotate_left(1);
        }
    }

    pub(in crate::analyze) fn install_numeric_error(&mut self) -> Result<()> {
        if !self.edition2 {
            return Ok(());
        }
        let id = crate::types::numeric_error_type();
        let Type::Enum { id, name, .. } = id else {
            return Err(Error::msg("compiler NumericError identity is not nominal"));
        };
        self.enum_headers.insert(name.clone(), (id, Vec::new()));
        let errors = [
            NumericError::NonFinite,
            NumericError::OutOfRange,
            NumericError::Fractional,
            NumericError::Inexact,
        ];
        let variants = errors
            .iter()
            .enumerate()
            .map(|(order, error)| {
                Ok(EnumVariant {
                    id: crate::types::numeric_error_variant(*error),
                    name: error.name().into(),
                    source_order: u16::try_from(order)
                        .map_err(|_| Error::msg("NumericError order exceeds u16"))?,
                    fields: Vec::new(),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        self.enums.push(EnumDefinition {
            id,
            name,
            origin: SourceId::new(u32::MAX),
            type_parameters: Vec::new(),
            variants,
            layout: EnumLayoutFacts {
                identity: crate::types::numeric_error_layout(),
                recursive: false,
            },
        });
        Ok(())
    }
}
