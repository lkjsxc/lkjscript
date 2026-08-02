use super::SemanticPrimitiveKind;

pub(super) fn primitive(value: SemanticPrimitiveKind) -> u8 {
    match value {
        SemanticPrimitiveKind::Never => 0,
        SemanticPrimitiveKind::Unit => 1,
        SemanticPrimitiveKind::Bool => 2,
        SemanticPrimitiveKind::I64 => 3,
        SemanticPrimitiveKind::F64 => 4,
        SemanticPrimitiveKind::String => 5,
        SemanticPrimitiveKind::Bytes => 6,
        SemanticPrimitiveKind::Path => 7,
        SemanticPrimitiveKind::ByteVector => 8,
        SemanticPrimitiveKind::ByteSlice => 9,
        SemanticPrimitiveKind::ByteSliceMut => 10,
        SemanticPrimitiveKind::Symbol => 11,
    }
}

pub(super) fn capability(value: crate::CapabilityKind) -> u8 {
    match value {
        crate::CapabilityKind::Arguments => 0,
        crate::CapabilityKind::Clock => 1,
        crate::CapabilityKind::Entropy => 2,
        crate::CapabilityKind::FileSystem => 3,
        crate::CapabilityKind::Network => 4,
        crate::CapabilityKind::Sqlite => 5,
        crate::CapabilityKind::Stdio => 6,
        crate::CapabilityKind::Terminal => 7,
    }
}

pub(super) fn resource(value: crate::ResourceKind) -> u8 {
    match value {
        crate::ResourceKind::InputStream => 0,
        crate::ResourceKind::OutputStream => 1,
        crate::ResourceKind::FileReader => 2,
        crate::ResourceKind::FileWriter => 3,
        crate::ResourceKind::FileAppender => 4,
        crate::ResourceKind::Directory => 5,
        crate::ResourceKind::TcpListener => 6,
        crate::ResourceKind::TcpStream => 7,
        crate::ResourceKind::SqliteConnection => 8,
        crate::ResourceKind::SqliteStatement => 9,
        crate::ResourceKind::TerminalSession => 10,
    }
}
