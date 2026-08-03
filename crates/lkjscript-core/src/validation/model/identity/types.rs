use super::encoder::Encoder;
use crate::*;

pub(super) fn capability(out: &mut Encoder, value: CapabilityKind) {
    use CapabilityKind::*;
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
pub(super) fn resource(out: &mut Encoder, value: ResourceKind) {
    use ResourceKind::*;
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
pub(super) fn structural_kind(out: &mut Encoder, value: StructuralKind) {
    use StructuralKind::*;
    out.tag(match value {
        Unit => 0,
        Bool => 1,
        I64 => 2,
        F64 => 3,
        String => 4,
        Path => 5,
        Bytes => 6,
        ByteVector => 7,
        Product => 8,
        Enum => 9,
        Static => 10,
    });
}
pub(super) fn structural_type(out: &mut Encoder, value: StructuralType) {
    let StructuralType {
        layout,
        semantic_type,
        kind,
    } = value;
    out.u64(layout.get());
    out.u64(semantic_type.get());
    structural_kind(out, kind);
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
