use super::{nonzero, Encoder, PreparedProgramError, PreparedProgramIdentity};

const DOMAIN: &[u8] = b"lkjscript.prepared-program-identity\0canonical-binary";

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum PackageProvenanceKind {
    Locked = 1,
    Development = 2,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct PreparedContractDigests {
    pub prepared_program: [u8; 32],
    pub runtime_calls: [u8; 32],
    pub native_layout: [u8; 32],
    pub verified_ssa: [u8; 32],
    pub bytecode: [u8; 32],
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct PreparedProgramDescriptor {
    pub package_kind: PackageProvenanceKind,
    pub package_content: [u8; 32],
    pub package_root: [u8; 32],
    pub entry: [u8; 32],
    pub module_memory_closure: [u8; 32],
    pub memory_plan: [u8; 32],
    pub witness_closure: [u8; 32],
    pub semantic_ssa: [u8; 32],
    pub native_specialization_ssa: Option<[u8; 32]>,
    pub validated_bytecode: [u8; 32],
    pub contracts: PreparedContractDigests,
}

impl PreparedProgramDescriptor {
    pub fn identity(&self) -> Result<PreparedProgramIdentity, PreparedProgramError> {
        self.validate()?;
        let mut out = Encoder::new(DOMAIN);
        out.tag(1);
        out.u8(self.package_kind as u8);
        for (tag, value) in [
            (2, self.package_content),
            (3, self.package_root),
            (4, self.entry),
            (5, self.module_memory_closure),
            (6, self.memory_plan),
            (7, self.witness_closure),
            (8, self.semantic_ssa),
        ] {
            out.tag(tag);
            out.fixed(&value);
        }
        out.tag(9);
        out.u8(u8::from(self.native_specialization_ssa.is_some()));
        if let Some(value) = self.native_specialization_ssa {
            out.fixed(&value);
        }
        for (tag, value) in [
            (10, self.validated_bytecode),
            (11, self.contracts.prepared_program),
            (12, self.contracts.runtime_calls),
            (13, self.contracts.native_layout),
            (14, self.contracts.verified_ssa),
            (15, self.contracts.bytecode),
        ] {
            out.tag(tag);
            out.fixed(&value);
        }
        PreparedProgramIdentity::new(out.finish())
    }

    fn validate(&self) -> Result<(), PreparedProgramError> {
        for value in [
            self.package_content,
            self.package_root,
            self.entry,
            self.module_memory_closure,
            self.memory_plan,
            self.witness_closure,
            self.semantic_ssa,
            self.validated_bytecode,
            self.contracts.prepared_program,
            self.contracts.runtime_calls,
            self.contracts.native_layout,
            self.contracts.verified_ssa,
            self.contracts.bytecode,
        ] {
            nonzero(value)?;
        }
        if let Some(value) = self.native_specialization_ssa {
            nonzero(value)?;
        }
        Ok(())
    }
}
