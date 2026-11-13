//! Source-facing type names are expanded only when producing a diagnostic.
use super::{Compiler, k};
impl Compiler<'_> {
    pub(super) fn check_owned_element(&self, ty: usize, span: super::a::Span) -> super::Result<()> {
        if self.has_view(ty) {
            return Err(super::err(
                span,
                "E_BORROW",
                "views cannot be stored in Box, Vec, or owned arrays; help: store an owned String/value or an owner index, and keep views in locals or view structs",
            ));
        }
        Ok(())
    }
    pub(super) fn display_type(&self, ty: usize) -> String {
        if let Some((name, args)) = self.applications.get(&ty) {
            let name = name.trim_start_matches("$std::");
            return if args.is_empty() {
                name.to_string()
            } else {
                format!(
                    "{name}<{}>",
                    args.iter()
                        .map(|ty| self.display_type(*ty))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
        }
        match &self.types[ty].kind {
            k::TypeKind::Ref { inner, mutable } => format!(
                "&{}{}",
                if *mutable { "mut " } else { "" },
                self.display_type(*inner)
            ),
            k::TypeKind::Vec { inner } => format!("Vec<{}>", self.display_type(*inner)),
            k::TypeKind::Box { inner } => format!("Box<{}>", self.display_type(*inner)),
            k::TypeKind::Slice { inner, .. } => format!("[{}]", self.display_type(*inner)),
            k::TypeKind::Array { inner, len } => format!("[{}; {len}]", self.display_type(*inner)),
            k::TypeKind::Struct { fields, .. } if !self.types[ty].name.starts_with('$') => format!(
                "({})",
                fields
                    .iter()
                    .map(|field| self.display_type(field.ty))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            _ => self.types[ty].name.clone(),
        }
    }
}
