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
    pub runtime_control: [u8; 32],
    pub process_outcome_codec: [u8; 32],
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct PreparedProgramDescriptor {
    pub platform_revision: u64,
    pub package_kind: PackageProvenanceKind,
    pub package_content: [u8; 32],
    pub package_root: [u8; 32],
    pub entry: [u8; 32],
    pub module_memory_closure: [u8; 32],
    pub memory_plan: [u8; 32],
    pub witness_closure: [u8; 32],
    pub semantic_ssa: [u8; 32],
    pub native_lowerable_ssa: [u8; 32],
    pub validated_bytecode: [u8; 32],
    pub contracts: PreparedContractDigests,
}

impl PreparedProgramDescriptor {
    pub fn identity(&self) -> Result<PreparedProgramIdentity, PreparedProgramError> {
        self.validate()?;
        let mut out = Encoder::new(DOMAIN);
        out.tag(1);
        out.u64(self.platform_revision);
        out.tag(2);
        out.u8(self.package_kind as u8);
        for (tag, value) in [
            (3, self.package_content),
            (4, self.package_root),
            (5, self.entry),
            (6, self.module_memory_closure),
            (7, self.memory_plan),
            (8, self.witness_closure),
            (9, self.semantic_ssa),
            (10, self.native_lowerable_ssa),
            (11, self.validated_bytecode),
            (12, self.contracts.prepared_program),
            (13, self.contracts.runtime_calls),
            (14, self.contracts.native_layout),
            (15, self.contracts.verified_ssa),
            (16, self.contracts.bytecode),
            (17, self.contracts.runtime_control),
            (18, self.contracts.process_outcome_codec),
        ] {
            out.tag(tag);
            out.fixed(&value);
        }
        PreparedProgramIdentity::new(out.finish())
    }

    fn validate(&self) -> Result<(), PreparedProgramError> {
        if self.platform_revision == 0 {
            return Err(PreparedProgramError::ZeroPlatformRevision);
        }
        for value in [
            self.package_content,
            self.package_root,
            self.entry,
            self.module_memory_closure,
            self.memory_plan,
            self.witness_closure,
            self.semantic_ssa,
            self.native_lowerable_ssa,
            self.validated_bytecode,
            self.contracts.prepared_program,
            self.contracts.runtime_calls,
            self.contracts.native_layout,
            self.contracts.verified_ssa,
            self.contracts.bytecode,
            self.contracts.runtime_control,
            self.contracts.process_outcome_codec,
        ] {
            nonzero(value)?;
        }
        Ok(())
    }
}
