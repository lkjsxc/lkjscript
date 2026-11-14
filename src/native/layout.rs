//! Native storage for checked IR. Aggregates occupy their complete inline extent.
use super::*;

#[derive(Clone)]
pub(super) struct TypeLayout {
    pub bytes: u64,
    pub alignment: u8,
    pub fields: Vec<u64>,
    pub variants: Vec<Vec<u64>>,
    pub scalar: bool,
    pub signed: bool,
    pub cleanup: bool,
}
pub(super) struct Layouts {
    types: Vec<TypeLayout>,
}
fn align(value: u64, alignment: u8) -> Result<u64, Diagnostic> {
    match value.checked_add(alignment as u64 - 1) {
        Some(value) => Ok(value & !(alignment as u64 - 1)),
        None => Err(Diagnostic::new(
            Span::default(),
            "E_EMIT",
            "native layout exceeds the address representation",
        )),
    }
}
impl Layouts {
    pub(super) fn new(program: &Program) -> Result<Self, Diagnostic> {
        let mut types = Vec::new();
        for _ in &program.types {
            types.push(TypeLayout {
                bytes: 0,
                alignment: 1,
                fields: Vec::new(),
                variants: Vec::new(),
                scalar: true,
                signed: false,
                cleanup: false,
            });
        }
        let mut result = Self { types };
        let mut visited = vec![0u8; program.types.len()];
        for index in 0..program.types.len() {
            result.visit(program, index, &mut visited)?;
        }
        for (ty, _) in &program.destructors {
            result.types[*ty].cleanup = true;
        }
        // Indirections have cleanup without consulting their pointee, so the
        // remaining direct aggregate graph is acyclic.
        let mut cleanup = vec![false; program.types.len()];
        for index in 0..program.types.len() {
            result.cleanup(program, index, &mut cleanup);
        }
        Ok(result)
    }
    fn fields(
        &mut self,
        program: &Program,
        fields: &[ir::Field],
        start: u64,
        visited: &mut [u8],
    ) -> Result<(u64, u8, Vec<u64>), Diagnostic> {
        let mut extent = start;
        let mut alignment = if start == 0 { 1 } else { 8 };
        let mut offsets = Vec::new();
        for field in fields {
            self.visit(program, field.ty, visited)?;
            let storage = &self.types[field.ty];
            extent = align(extent, storage.alignment)?;
            offsets.push(extent);
            alignment = alignment.max(storage.alignment);
            extent = match extent.checked_add(storage.bytes) {
                Some(value) => value,
                None => {
                    return Err(Diagnostic::new(
                        Span::default(),
                        "E_EMIT",
                        "native field extent exceeds the address representation",
                    ));
                }
            };
        }
        Ok((align(extent.max(1), alignment)?, alignment, offsets))
    }
    fn visit(
        &mut self,
        program: &Program,
        ty: usize,
        visited: &mut [u8],
    ) -> Result<(), Diagnostic> {
        if visited[ty] == 2 {
            return Ok(());
        }
        if visited[ty] == 1 {
            return Err(Diagnostic::new(
                Span::default(),
                "E_EMIT",
                "recursive inline native layout",
            ));
        }
        visited[ty] = 1;
        let mut value = TypeLayout {
            bytes: 8,
            alignment: 8,
            fields: Vec::new(),
            variants: Vec::new(),
            scalar: true,
            signed: false,
            cleanup: false,
        };
        let kind = program.types[ty].kind.clone();
        match kind {
            TypeKind::Unit => {
                value.bytes = 0;
                value.alignment = 1;
            }
            TypeKind::Bool => {
                value.bytes = 1;
                value.alignment = 1;
            }
            TypeKind::Char => {
                value.bytes = 4;
                value.alignment = 4;
            }
            TypeKind::Int { bits, signed } => {
                value.bytes = (bits / 8) as u64;
                value.alignment = bits / 8;
                value.signed = signed;
            }
            TypeKind::Struct { fields, .. } => {
                let (bytes, alignment, offsets) = self.fields(program, &fields, 0, visited)?;
                value.bytes = bytes;
                value.alignment = alignment;
                value.fields = offsets;
                value.scalar = false;
            }
            TypeKind::Enum { variants, .. } => {
                value.bytes = 8;
                for variant in &variants {
                    let (bytes, alignment, offsets) =
                        self.fields(program, &variant.fields, 8, visited)?;
                    value.bytes = value.bytes.max(bytes);
                    value.alignment = value.alignment.max(alignment);
                    value.variants.push(offsets);
                }
                value.bytes = align(value.bytes, value.alignment)?;
                value.scalar = false;
            }
            TypeKind::Array { inner, len } => {
                self.visit(program, inner, visited)?;
                let stride = self.stride(inner);
                value.bytes = match stride.checked_mul(len as u64) {
                    Some(bytes) => bytes,
                    None => {
                        return Err(Diagnostic::new(
                            Span::default(),
                            "E_EMIT",
                            "native array extent exceeds the address representation",
                        ));
                    }
                };
                value.alignment = self.types[inner].alignment;
                value.scalar = false;
            }
            TypeKind::Vec { .. } => {
                value.bytes = 24;
                value.scalar = false;
                value.cleanup = true;
            }
            TypeKind::Box { .. } | TypeKind::File | TypeKind::Directory | TypeKind::Child => {
                value.cleanup = true;
            }
            TypeKind::Ref { inner, .. } => {
                if matches!(
                    program.types[inner].kind,
                    TypeKind::Slice { .. } | TypeKind::Str
                ) {
                    value.bytes = 16;
                    value.scalar = false;
                }
            }
            TypeKind::Slice { .. } | TypeKind::Str => {
                value.bytes = 16;
                value.scalar = false;
            }
            _ => {}
        }
        self.types[ty] = value;
        visited[ty] = 2;
        Ok(())
    }
    fn cleanup(&mut self, program: &Program, ty: usize, visited: &mut [bool]) -> bool {
        if visited[ty] {
            return self.types[ty].cleanup;
        }
        visited[ty] = true;
        if self.types[ty].cleanup {
            return true;
        }
        let kind = program.types[ty].kind.clone();
        let mut cleanup = false;
        match kind {
            TypeKind::Struct { fields, .. } => {
                for field in &fields {
                    if self.cleanup(program, field.ty, visited) {
                        cleanup = true;
                    }
                }
            }
            TypeKind::Enum { variants, .. } => {
                for variant in &variants {
                    for field in &variant.fields {
                        if self.cleanup(program, field.ty, visited) {
                            cleanup = true;
                        }
                    }
                }
            }
            TypeKind::Array { inner, .. } => cleanup = self.cleanup(program, inner, visited),
            _ => {}
        }
        self.types[ty].cleanup = cleanup;
        cleanup
    }
    pub(super) fn storage(&self, ty: usize) -> &TypeLayout {
        &self.types[ty]
    }
    pub(super) fn stride(&self, ty: usize) -> u64 {
        self.types[ty].bytes
    }
    pub(super) fn field_offset(&self, ty: usize, variant: Option<usize>, field: usize) -> u64 {
        match variant {
            Some(variant) => self.types[ty].variants[variant][field],
            None => self.types[ty].fields[field],
        }
    }
}
