//! The last monomorphic-child wire is retained independently of current Encode.
use super::*;

#[derive(Encode)]
struct Unit20<'a> {
    contract_version: u16,
    graph_contract_version: u16,
    bytecode_contract_version: u16,
    key: CompilationUnitKey,
    source: &'a CompilationSource,
    optimization: OptimizationPolicy,
    tables: &'a super::super::super::unit::CompilationTables,
    payload: Function20<'a>,
}
struct Function20<'a> {
    signature: &'a super::super::super::unit::CompiledSignature,
    code: Code20<'a>,
}
impl Encode for Function20<'_> {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        6_u32.encode(encoder)?;
        self.signature.encode(encoder)?;
        self.code.encode(encoder)
    }
}
#[derive(Encode)]
struct Code20<'a> {
    parameter_count: u32,
    local_count: u32,
    instructions: Vec<Instruction20<'a>>,
}
struct Instruction20<'a>(&'a CompiledInstruction);
impl Encode for Instruction20<'_> {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        if let CompiledInstruction::Parallel {
            left,
            left_arguments,
            right,
            right_arguments,
            result_type,
            ..
        } = self.0
        {
            38_u32.encode(encoder)?;
            left.encode(encoder)?;
            left_arguments.encode(encoder)?;
            right.encode(encoder)?;
            right_arguments.encode(encoder)?;
            result_type.encode(encoder)
        } else {
            assert!(matches!(self.0, CompiledInstruction::Return));
            27_u32.encode(encoder)
        }
    }
}

#[test]
fn parallel_generic_predecessor20_requires_rebuild_before_application_decoding() {
    let (loaded, key, mut unit) = fixture();
    unit.contract_version = 20;
    unit.bytecode_contract_version = 16;
    unit.key =
        CompilationUnitKey::derive_generation(&unit.source, unit.optimization, 20, 16, 21).unwrap();
    let CompilationPayload::Function { signature, code } = &unit.payload else {
        panic!("function");
    };
    let old = Unit20 {
        contract_version: 20,
        graph_contract_version: 21,
        bytecode_contract_version: 16,
        key: unit.key,
        source: &unit.source,
        optimization: unit.optimization,
        tables: &unit.tables,
        payload: Function20 {
            signature,
            code: Code20 {
                parameter_count: code.parameter_count,
                local_count: code.local_count,
                instructions: code.instructions.iter().map(Instruction20).collect(),
            },
        },
    };
    let bytes = crate::platform::packed::encode(
        *b"LKJCUN20",
        "lkjscript.compiler-unit-envelope.v20",
        &old,
        super::super::super::unit::MAXIMUM_COMPILER_UNIT_BYTES,
    )
    .unwrap();
    let object = ObjectKey::for_bytes(ObjectDomain::CompilerUnit, &bytes);
    let error = CompilationUnit::decode(&bytes, object).unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Source);
    assert_eq!(error.code, "compiler_unit_contract");
    let error = load_artifact(&effect_tests::replace_unit_encoded(
        &loaded,
        key,
        &unit,
        bytes,
        vec![],
    ))
    .unwrap_err();
    assert_eq!(error.code, "compiler_unit_contract");
    let mut manifest = loaded.manifest.clone();
    manifest.contract_version = 27;
    assert_eq!(
        manifest.encode().unwrap_err().code,
        "artifact_manifest_contract"
    );
}
