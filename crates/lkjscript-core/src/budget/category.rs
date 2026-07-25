#[repr(usize)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ResourceCategory {
    SourceBytes,
    SourceUnits,
    ImportEdges,
    Tokens,
    SchemaNodes,
    TopLevelDeclarations,
    ProductFields,
    ParserWork,
    ValidationWork,
    PathWork,
    TypeNesting,
    TypeWork,
    TraitWork,
    OwnershipExpressions,
    OwnershipRetainedState,
    HirFunctions,
    HirExpressions,
    SsaFunctions,
    SsaBlocks,
    SsaValues,
    SsaEdges,
    SsaFrameStates,
    Diagnostics,
    ProtocolRequestBytes,
    ProtocolResponseBytes,
}

impl ResourceCategory {
    pub const ALL: [Self; 25] = [
        Self::SourceBytes,
        Self::SourceUnits,
        Self::ImportEdges,
        Self::Tokens,
        Self::SchemaNodes,
        Self::TopLevelDeclarations,
        Self::ProductFields,
        Self::ParserWork,
        Self::ValidationWork,
        Self::PathWork,
        Self::TypeNesting,
        Self::TypeWork,
        Self::TraitWork,
        Self::OwnershipExpressions,
        Self::OwnershipRetainedState,
        Self::HirFunctions,
        Self::HirExpressions,
        Self::SsaFunctions,
        Self::SsaBlocks,
        Self::SsaValues,
        Self::SsaEdges,
        Self::SsaFrameStates,
        Self::Diagnostics,
        Self::ProtocolRequestBytes,
        Self::ProtocolResponseBytes,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SourceBytes => "source_bytes",
            Self::SourceUnits => "source_units",
            Self::ImportEdges => "import_edges",
            Self::Tokens => "tokens",
            Self::SchemaNodes => "schema_nodes",
            Self::TopLevelDeclarations => "top_level_declarations",
            Self::ProductFields => "product_fields",
            Self::ParserWork => "parser_work",
            Self::ValidationWork => "validation_work",
            Self::PathWork => "path_work",
            Self::TypeNesting => "type_nesting",
            Self::TypeWork => "type_work",
            Self::TraitWork => "trait_work",
            Self::OwnershipExpressions => "ownership_expressions",
            Self::OwnershipRetainedState => "ownership_retained_state",
            Self::HirFunctions => "hir_functions",
            Self::HirExpressions => "hir_expressions",
            Self::SsaFunctions => "ssa_functions",
            Self::SsaBlocks => "ssa_blocks",
            Self::SsaValues => "ssa_values",
            Self::SsaEdges => "ssa_edges",
            Self::SsaFrameStates => "ssa_frame_states",
            Self::Diagnostics => "diagnostics",
            Self::ProtocolRequestBytes => "protocol_request_bytes",
            Self::ProtocolResponseBytes => "protocol_response_bytes",
        }
    }

    pub const fn unit(self) -> &'static str {
        match self {
            Self::SourceBytes | Self::ProtocolRequestBytes | Self::ProtocolResponseBytes => "bytes",
            Self::ParserWork
            | Self::ValidationWork
            | Self::PathWork
            | Self::TypeWork
            | Self::TraitWork => "work-units",
            _ => "records",
        }
    }

    pub(crate) const fn index(self) -> usize {
        self as usize
    }
}

pub(crate) const RESOURCE_CATEGORY_COUNT: usize = ResourceCategory::ALL.len();
