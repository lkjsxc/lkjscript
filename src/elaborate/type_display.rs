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
            let mut start = 0;
            while name[start..].starts_with("$std::") {
                start += 6;
            }
            let name = &name[start..];
            if args.is_empty() {
                return name.to_string();
            }
            let mut display = name.to_string();
            display.push('<');
            let mut index = 0;
            while index < args.len() {
                if index != 0 {
                    display.push_str(", ");
                }
                display.push_str(&self.display_type(args[index]));
                index += 1;
            }
            display.push('>');
            return display;
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
            k::TypeKind::Struct { fields, .. } if !self.types[ty].name.starts_with('$') => {
                let mut display = "(".to_string();
                let mut index = 0;
                while index < fields.len() {
                    if index != 0 {
                        display.push_str(", ");
                    }
                    display.push_str(&self.display_type(fields[index].ty));
                    index += 1;
                }
                display.push(')');
                display
            }
            _ => self.types[ty].name.clone(),
        }
    }
}
