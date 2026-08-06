use super::encoder::Encoder;
use crate::*;

pub(super) fn ty(out: &mut Encoder, value: &SsaType) {
    let mut pending = vec![value];
    while let Some(value) = pending.pop() {
        match value {
            SsaType::Unit => out.tag(0),
            SsaType::Bool => out.tag(1),
            SsaType::I64 => out.tag(2),
            SsaType::F64 => out.tag(3),
            SsaType::Str => out.tag(4),
            SsaType::Symbol => out.tag(5),
            SsaType::Bytes => out.tag(6),
            SsaType::ByteVector => out.tag(7),
            SsaType::ByteSlice => out.tag(8),
            SsaType::ByteSliceMut => out.tag(9),
            SsaType::Path => out.tag(10),
            SsaType::Capability(kind) => {
                out.tag(11);
                capability(out, *kind);
            }
            SsaType::Resource(kind) => {
                out.tag(12);
                resource(out, *kind);
            }
            SsaType::StructuralDestination(id) => {
                out.tag(13);
                out.u64(id.raw());
            }
            SsaType::Product(id) => {
                out.tag(14);
                out.u64(id.raw());
            }
            SsaType::Enum { id, arguments } => {
                out.tag(15);
                out.fixed(&id.bytes());
                out.len(arguments.len());
                pending.extend(arguments.iter().rev());
            }
            SsaType::List(element) => {
                out.tag(16);
                pending.push(element);
            }
            SsaType::Function(signature) => {
                out.tag(17);
                signature_header(out, signature);
                out.len(signature.parameters.len());
                pending.push(&signature.result);
                pending.extend(signature.parameters.iter().rev());
            }
            SsaType::TypeParameter(name) => {
                out.tag(18);
                out.string(name);
            }
        }
    }
}

fn signature_header(out: &mut Encoder, value: &Signature) {
    out.sequence(&value.type_parameters, |out, value| out.string(value));
    out.sequence(&value.bounds, |out, value| {
        let TraitBound {
            parameter,
            trait_id,
        } = value;
        out.string(parameter);
        out.wide(trait_id.raw());
    });
    out.sequence(&value.memory_witness_parameters, |out, value| {
        let MemoryWitnessParameter {
            parameter,
            operations,
        } = value;
        out.string(parameter);
        out.sequence(operations, |out, value| witness_operation(out, *value));
    });
}

pub(super) fn signature_value(out: &mut Encoder, value: &Signature) {
    signature_header(out, value);
    out.len(value.parameters.len());
    for parameter in &value.parameters {
        ty(out, parameter);
    }
    ty(out, &value.result);
}

pub(super) fn instantiation(out: &mut Encoder, value: &GenericInstantiation) {
    let GenericInstantiation {
        substitutions,
        witnesses,
        memory_witnesses,
    } = value;
    out.sequence(substitutions, |out, value| {
        let TypeSubstitution {
            parameter,
            ty: value_ty,
        } = value;
        out.string(parameter);
        ty(out, value_ty);
    });
    out.sequence(witnesses, |out, value| {
        let TraitWitness {
            trait_id,
            ty: value_ty,
            kind,
        } = value;
        out.wide(trait_id.raw());
        ty(out, value_ty);
        match kind {
            TraitWitnessKind::AutoTrait => out.tag(0),
            TraitWitnessKind::Explicit(id) => {
                out.tag(1);
                out.wide(id.raw());
            }
        }
    });
    out.sequence(memory_witnesses, |out, value| {
        let MemoryWitnessBinding { parameter, witness } = value;
        out.string(parameter);
        out.fixed(&witness.bytes());
    });
}

pub(super) fn capability(out: &mut Encoder, value: lkjscript_contracts::CapabilityKind) {
    use lkjscript_contracts::CapabilityKind::*;
    out.tag(match value {
        Arguments => 0,
        Clock => 1,
        Entropy => 2,
        FileSystem => 3,
        Network => 4,
        Sqlite => 5,
        Stdio => 6,
        Terminal => 7,
    });
}

pub(super) fn resource(out: &mut Encoder, value: lkjscript_contracts::ResourceKind) {
    use lkjscript_contracts::ResourceKind::*;
    out.tag(match value {
        InputStream => 0,
        OutputStream => 1,
        FileReader => 2,
        FileWriter => 3,
        FileAppender => 4,
        Directory => 5,
        TcpListener => 6,
        TcpStream => 7,
        SqliteConnection => 8,
        SqliteStatement => 9,
        TerminalSession => 10,
    });
}

pub(super) fn witness_operation(
    out: &mut Encoder,
    value: lkjscript_contracts::MemoryWitnessOperation,
) {
    use lkjscript_contracts::MemoryWitnessOperation::*;
    out.tag(match value {
        Transport => 0,
        Clone => 1,
        Drop => 2,
        Share => 3,
        Compare => 4,
        Encode => 5,
        Decode => 6,
        ListImport => 7,
        ListExport => 8,
        IndependentOwner => 9,
        Dispose => 10,
    });
}
