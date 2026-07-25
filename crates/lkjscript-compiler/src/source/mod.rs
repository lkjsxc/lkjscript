//! Semantic Source Foundation V1 for validated Edition 1 loading and identity.
//!
//! This is the deterministic parser/load/identity foundation identified as
//! `lkjscript.semantic-source-foundation` version 1. It is not the complete
//! `lkjscript.semantic-source` schema or agent/edit protocol. Parsing is
//! intentionally private. The only public source representation is
//! [`ValidatedSourceTree`], which can be produced only by this module's
//! validators.

mod format;
mod load;
mod parser;

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use lkjscript_core::{Error, Limits, Result};

pub use load::validate_source_directory_tree;

/// Identity of the incomplete parser/load/identity foundation.
pub const SEMANTIC_SOURCE_FOUNDATION_SCHEMA: &str = "lkjscript.semantic-source-foundation";
/// Version of the incomplete parser/load/identity foundation.
pub const SEMANTIC_SOURCE_FOUNDATION_SCHEMA_VERSION: u32 = 1;
pub const SOURCE_EDITION: u32 = 1;
/// Always-enforced Foundation V1 maximum for exact bytes in one source file.
pub const FOUNDATION_MAX_SOURCE_FILE_BYTES: u64 = 16 * 1024 * 1024;
/// Always-enforced Foundation V1 maximum for exact bytes in a source closure.
pub const FOUNDATION_MAX_AGGREGATE_SOURCE_BYTES: u64 = 256 * 1024 * 1024;
/// Always-enforced Foundation V1 maximum for source units in a source closure.
pub const FOUNDATION_MAX_SOURCE_UNITS: u64 = 65_536;
/// Always-enforced Foundation V1 maximum for entries scanned by a source-tree check.
pub const FOUNDATION_MAX_SOURCE_TREE_ENTRIES: u64 = 65_536;
/// Identity of the incomplete structured source-diagnostic foundation.
pub const SOURCE_DIAGNOSTIC_FOUNDATION_SCHEMA: &str = "lkjscript.source-diagnostic-foundation";
/// Version of the incomplete structured source-diagnostic foundation.
pub const SOURCE_DIAGNOSTIC_FOUNDATION_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SourcePosition {
    byte: u32,
    line: u32,
    column: u32,
}

impl SourcePosition {
    pub const fn byte(self) -> u32 {
        self.byte
    }

    pub const fn line(self) -> u32 {
        self.line
    }

    pub const fn column(self) -> u32 {
        self.column
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SourceSpan {
    start: SourcePosition,
    end: SourcePosition,
}

impl SourceSpan {
    pub const fn start(self) -> SourcePosition {
        self.start
    }

    pub const fn end(self) -> SourcePosition {
        self.end
    }

    pub const fn byte_range(self) -> std::ops::Range<u32> {
        self.start.byte..self.end.byte
    }

    fn zero() -> Self {
        let position = SourcePosition {
            byte: 0,
            line: 1,
            column: 1,
        };
        Self {
            start: position,
            end: position,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SourceOrigin {
    logical_path: String,
    host_containment_path: Option<PathBuf>,
}

impl SourceOrigin {
    pub fn logical_path(&self) -> &str {
        &self.logical_path
    }

    /// Canonical host path after containment checking. In-memory source has no
    /// host path and still has an independent canonical logical origin.
    pub fn host_containment_path(&self) -> Option<&Path> {
        self.host_containment_path.as_deref()
    }

    fn in_memory(path: &str) -> Self {
        Self {
            logical_path: canonical_logical_path(Path::new(path)),
            host_containment_path: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiagnosticSeverity {
    Error,
}

impl DiagnosticSeverity {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiagnosticCategory {
    SourceSyntax,
    Declaration,
    ResourceLimit,
    SourceLoading,
}

impl DiagnosticCategory {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SourceSyntax => "source-syntax",
            Self::Declaration => "declaration",
            Self::ResourceLimit => "resource-limit",
            Self::SourceLoading => "source-loading",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiagnosticCertainty {
    Guaranteed,
}

impl DiagnosticCertainty {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Guaranteed => "guaranteed",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelatedSourceSpan {
    label: String,
    origin: SourceOrigin,
    span: SourceSpan,
}

impl RelatedSourceSpan {
    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn origin(&self) -> &SourceOrigin {
        &self.origin
    }

    pub const fn span(&self) -> SourceSpan {
        self.span
    }
}

/// Structured source-foundation diagnostic. Existing compiler entry points
/// render this record to human text in `lkjscript_core::Error`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceDiagnostic {
    code: &'static str,
    severity: DiagnosticSeverity,
    category: DiagnosticCategory,
    certainty: DiagnosticCertainty,
    message: String,
    origin: Box<SourceOrigin>,
    primary_span: SourceSpan,
    related: Vec<RelatedSourceSpan>,
}

impl SourceDiagnostic {
    pub const fn schema(&self) -> &'static str {
        SOURCE_DIAGNOSTIC_FOUNDATION_SCHEMA
    }

    pub const fn schema_version(&self) -> u32 {
        SOURCE_DIAGNOSTIC_FOUNDATION_SCHEMA_VERSION
    }

    pub const fn code(&self) -> &'static str {
        self.code
    }

    pub const fn severity(&self) -> DiagnosticSeverity {
        self.severity
    }

    pub const fn category(&self) -> DiagnosticCategory {
        self.category
    }

    pub const fn certainty(&self) -> DiagnosticCertainty {
        self.certainty
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn origin(&self) -> &SourceOrigin {
        self.origin.as_ref()
    }

    pub const fn primary_span(&self) -> SourceSpan {
        self.primary_span
    }

    pub fn related_spans(&self) -> &[RelatedSourceSpan] {
        &self.related
    }

    pub fn render_human(&self) -> String {
        let start = self.primary_span.start();
        let mut rendered = format!(
            "{}:{}:{}: {}[{}]: {}",
            self.origin.logical_path(),
            start.line(),
            start.column(),
            self.severity.as_str(),
            self.code,
            self.message
        );
        for related in &self.related {
            let start = related.span.start();
            rendered.push_str(&format!(
                "\n  related {}:{}:{}: {}",
                related.origin.logical_path(),
                start.line(),
                start.column(),
                related.label
            ));
        }
        rendered
    }

    pub fn render_compact_agent(&self) -> String {
        let start = self.primary_span.start();
        let end = self.primary_span.end();
        let mut rendered = format!(
            "schema={SOURCE_DIAGNOSTIC_FOUNDATION_SCHEMA};version={SOURCE_DIAGNOSTIC_FOUNDATION_SCHEMA_VERSION};code={};severity={};category={};certainty={};origin={};span={}:{}:{}-{}:{}:{};message={}",
            self.code,
            self.severity.as_str(),
            self.category.as_str(),
            self.certainty.as_str(),
            escape_compact(self.origin.logical_path()),
            start.byte(),
            start.line(),
            start.column(),
            end.byte(),
            end.line(),
            end.column(),
            escape_compact(&self.message)
        );
        for (index, related) in self.related.iter().enumerate() {
            let start = related.span.start();
            let end = related.span.end();
            rendered.push_str(&format!(
                ";related[{index}].label={};related[{index}].origin={};related[{index}].span={}:{}:{}-{}:{}:{}",
                escape_compact(&related.label),
                escape_compact(related.origin.logical_path()),
                start.byte(),
                start.line(),
                start.column(),
                end.byte(),
                end.line(),
                end.column()
            ));
        }
        rendered
    }

    fn new(
        code: &'static str,
        category: DiagnosticCategory,
        message: impl Into<String>,
        origin: SourceOrigin,
        primary_span: SourceSpan,
    ) -> Self {
        Self {
            code,
            severity: DiagnosticSeverity::Error,
            category,
            certainty: DiagnosticCertainty::Guaranteed,
            message: message.into(),
            origin: Box::new(origin),
            primary_span,
            related: Vec::new(),
        }
    }

    fn with_related(
        mut self,
        label: impl Into<String>,
        origin: SourceOrigin,
        span: SourceSpan,
    ) -> Self {
        self.related.push(RelatedSourceSpan {
            label: label.into(),
            origin,
            span,
        });
        self
    }

    fn generic(origin: SourceOrigin, message: impl Into<String>) -> Self {
        Self::new(
            "LKJ-SRC-INVALID",
            DiagnosticCategory::SourceSyntax,
            message,
            origin,
            SourceSpan::zero(),
        )
    }

    fn loading(origin: SourceOrigin, message: impl Into<String>) -> Self {
        Self::new(
            "LKJ-SRC-LOAD",
            DiagnosticCategory::SourceLoading,
            message,
            origin,
            SourceSpan::zero(),
        )
    }

    pub(crate) fn into_core(self) -> Error {
        Error::msg(self.render_human())
    }
}

impl fmt::Display for SourceDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.render_human())
    }
}

impl std::error::Error for SourceDiagnostic {}

pub type SourceResult<T> = std::result::Result<T, SourceDiagnostic>;

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Expr {
    LitI64(i64),
    LitF64(f64),
    LitBool(bool),
    LitUnit,
    LitStr(String),
    Symbol(String),
    Call { name: String, args: Vec<Expr> },
    List(Vec<Expr>),
}

impl Expr {
    #[allow(dead_code)]
    pub(crate) fn depth(&self) -> u32 {
        match self {
            Self::Call { args, .. } | Self::List(args) => {
                1 + args.iter().map(Self::depth).max().unwrap_or(0)
            }
            _ => 0,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum SyntaxKind {
    I64 { value: i64 },
    F64 { value: f64 },
    Bool { value: bool },
    Unit,
    Str { value: String },
    Symbol { name: String },
    Call { name: String },
}

#[derive(Clone, Debug)]
pub(crate) struct SourceNode {
    pub(crate) kind: SyntaxKind,
    pub(crate) span: SourceSpan,
    pub(crate) leading_trivia: Vec<String>,
    pub(crate) before_close_trivia: Vec<String>,
    pub(crate) children: Vec<SourceNode>,
}

impl SourceNode {
    pub(crate) fn project(&self) -> Expr {
        match &self.kind {
            SyntaxKind::I64 { value } => Expr::LitI64(*value),
            SyntaxKind::F64 { value } => Expr::LitF64(*value),
            SyntaxKind::Bool { value } => Expr::LitBool(*value),
            SyntaxKind::Unit => Expr::LitUnit,
            SyntaxKind::Str { value } => Expr::LitStr(value.clone()),
            SyntaxKind::Symbol { name } => Expr::Symbol(name.clone()),
            SyntaxKind::Call { name } => Expr::Call {
                name: name.clone(),
                args: self.children.iter().map(Self::project).collect(),
            },
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum TokenKind {
    Open(String),
    Close(String),
    Atom(String),
    Str(String),
}

#[derive(Clone, Debug)]
pub(crate) struct Token {
    pub(crate) kind: TokenKind,
    pub(crate) span: SourceSpan,
    pub(crate) leading_trivia: Vec<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct SourceFile {
    pub(crate) path: PathBuf,
    pub(crate) origin: SourceOrigin,
    pub(crate) exact_source_len: u64,
    pub(crate) exact_source_sha256: [u8; 32],
    pub(crate) forms: Vec<Expr>,
    pub(crate) syntax: Vec<SourceNode>,
    pub(crate) tokens: Vec<Token>,
    pub(crate) trailing_trivia: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RevisionId([u8; 32]);

impl RevisionId {
    pub const fn as_bytes(self) -> [u8; 32] {
        self.0
    }

    pub fn to_hex(self) -> String {
        hex(&self.0)
    }
}

impl fmt::Display for RevisionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&hex(&self.0))
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId {
    revision: RevisionId,
    index: u32,
}

impl NodeId {
    pub const fn revision(self) -> RevisionId {
        self.revision
    }

    pub const fn index(self) -> u32 {
        self.index
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum NodeKind {
    I64Literal,
    F64Literal,
    BoolLiteral,
    UnitLiteral,
    StringLiteral,
    Symbol,
    Call,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeSummary {
    id: NodeId,
    kind: NodeKind,
    label: Option<String>,
    origin: SourceOrigin,
    span: SourceSpan,
    parent: Option<NodeId>,
    children: Vec<NodeId>,
}

impl NodeSummary {
    pub const fn id(&self) -> NodeId {
        self.id
    }

    pub const fn kind(&self) -> NodeKind {
        self.kind
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    pub fn origin(&self) -> &SourceOrigin {
        &self.origin
    }

    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    pub const fn parent(&self) -> Option<NodeId> {
        self.parent
    }

    pub fn children(&self) -> &[NodeId] {
        &self.children
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DeclarationKind {
    Main,
    Function,
    Product,
    Trait,
    Implementation,
}

impl DeclarationKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::Function => "function",
            Self::Product => "product",
            Self::Trait => "trait",
            Self::Implementation => "implementation",
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DeclarationKey {
    digest: [u8; 32],
    exact_identity: Vec<u8>,
    canonical_identity: String,
}

impl DeclarationKey {
    pub const fn digest(&self) -> [u8; 32] {
        self.digest
    }

    pub fn canonical_identity(&self) -> &str {
        &self.canonical_identity
    }

    pub fn to_hex(&self) -> String {
        hex(&self.digest)
    }
}

impl fmt::Display for DeclarationKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&hex(&self.digest))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeclarationSummary {
    key: DeclarationKey,
    kind: DeclarationKind,
    name: String,
    origin: SourceOrigin,
    span: SourceSpan,
    node: NodeId,
}

impl DeclarationSummary {
    pub fn key(&self) -> &DeclarationKey {
        &self.key
    }

    pub const fn kind(&self) -> DeclarationKind {
        self.kind
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn origin(&self) -> &SourceOrigin {
        &self.origin
    }

    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    pub const fn node(&self) -> NodeId {
        self.node
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StaleNodeId {
    expected: RevisionId,
    actual: RevisionId,
}

impl StaleNodeId {
    pub const fn expected_revision(&self) -> RevisionId {
        self.expected
    }

    pub const fn actual_revision(&self) -> RevisionId {
        self.actual
    }
}

impl fmt::Display for StaleNodeId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "node belongs to revision {}, expected {}",
            self.actual, self.expected
        )
    }
}

impl std::error::Error for StaleNodeId {}

/// Opaque, immutable Semantic Source Foundation V1 authority.
///
/// Source origins, declarations, and nodes are exposed in deterministic
/// canonical logical-path and stable-key order. Raw forms and the mutable
/// builder remain private. This type does not claim the complete Semantic
/// Source schema, transaction protocol, JSON transport, or typed holes.
#[derive(Clone, Debug)]
pub struct ValidatedSourceTree {
    revision: RevisionId,
    root: PathBuf,
    root_origin: SourceOrigin,
    files: Vec<SourceFile>,
    origins: Vec<SourceOrigin>,
    declarations: Vec<DeclarationSummary>,
    nodes: Vec<NodeSummary>,
}

impl ValidatedSourceTree {
    pub const fn revision(&self) -> RevisionId {
        self.revision
    }

    pub fn root_origin(&self) -> &SourceOrigin {
        &self.root_origin
    }

    pub fn source_origins(&self) -> &[SourceOrigin] {
        &self.origins
    }

    pub fn declarations(&self) -> &[DeclarationSummary] {
        &self.declarations
    }

    pub fn declaration(&self, key: &DeclarationKey) -> Option<&DeclarationSummary> {
        self.declarations
            .iter()
            .find(|declaration| declaration.key() == key)
    }

    pub fn nodes(&self) -> &[NodeSummary] {
        &self.nodes
    }

    pub fn node(&self, id: NodeId) -> std::result::Result<Option<&NodeSummary>, StaleNodeId> {
        if id.revision != self.revision {
            return Err(StaleNodeId {
                expected: self.revision,
                actual: id.revision,
            });
        }
        Ok(self
            .nodes
            .get(id.index as usize)
            .filter(|node| node.id == id))
    }

    /// Format the source unit at `logical_path` from structural nodes.
    pub fn format_source(&self, logical_path: &str) -> Option<String> {
        let origin = validate_logical_source_path(logical_path).ok()?;
        self.files
            .iter()
            .find(|file| file.origin.logical_path == origin.logical_path)
            .map(format::format_file)
    }

    /// Format a single-unit validated tree.
    pub fn format_single_source(&self) -> Option<String> {
        if self.files.len() != 1 {
            return None;
        }
        self.files.first().map(format::format_file)
    }

    pub(crate) fn files(&self) -> &[SourceFile] {
        &self.files
    }

    pub(crate) fn root_path(&self) -> &Path {
        &self.root
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct LoadMetrics {
    pub(crate) source_loading: Duration,
    pub(crate) parsing: Duration,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct SourceFoundationBudget {
    source_units: u64,
    source_bytes: u64,
}

impl SourceFoundationBudget {
    pub(super) fn check_metadata(
        &self,
        origin: &SourceOrigin,
        file_bytes: u64,
    ) -> SourceResult<()> {
        check_foundation_file_bytes(origin, file_bytes)?;
        let units = self.source_units.checked_add(1).ok_or_else(|| {
            foundation_resource_error(
                origin.clone(),
                "source-units",
                u64::MAX,
                FOUNDATION_MAX_SOURCE_UNITS,
            )
        })?;
        if units > FOUNDATION_MAX_SOURCE_UNITS {
            return Err(foundation_resource_error(
                origin.clone(),
                "source-units",
                units,
                FOUNDATION_MAX_SOURCE_UNITS,
            ));
        }
        let bytes = self.source_bytes.checked_add(file_bytes).ok_or_else(|| {
            foundation_resource_error(
                origin.clone(),
                "aggregate-source-bytes",
                u64::MAX,
                FOUNDATION_MAX_AGGREGATE_SOURCE_BYTES,
            )
        })?;
        if bytes > FOUNDATION_MAX_AGGREGATE_SOURCE_BYTES {
            return Err(foundation_resource_error(
                origin.clone(),
                "aggregate-source-bytes",
                bytes,
                FOUNDATION_MAX_AGGREGATE_SOURCE_BYTES,
            ));
        }
        Ok(())
    }

    pub(super) fn remaining_read_allowance(&self, origin: &SourceOrigin) -> SourceResult<u64> {
        let aggregate = FOUNDATION_MAX_AGGREGATE_SOURCE_BYTES
            .checked_sub(self.source_bytes)
            .ok_or_else(|| {
                foundation_resource_error(
                    origin.clone(),
                    "aggregate-source-bytes",
                    self.source_bytes,
                    FOUNDATION_MAX_AGGREGATE_SOURCE_BYTES,
                )
            })?;
        Ok(FOUNDATION_MAX_SOURCE_FILE_BYTES.min(aggregate))
    }

    pub(super) fn record_read(
        &mut self,
        origin: &SourceOrigin,
        file_bytes: u64,
    ) -> SourceResult<()> {
        self.check_metadata(origin, file_bytes)?;
        self.source_units = self.source_units.checked_add(1).ok_or_else(|| {
            foundation_resource_error(
                origin.clone(),
                "source-units",
                u64::MAX,
                FOUNDATION_MAX_SOURCE_UNITS,
            )
        })?;
        self.source_bytes = self.source_bytes.checked_add(file_bytes).ok_or_else(|| {
            foundation_resource_error(
                origin.clone(),
                "aggregate-source-bytes",
                u64::MAX,
                FOUNDATION_MAX_AGGREGATE_SOURCE_BYTES,
            )
        })?;
        Ok(())
    }

    #[cfg(test)]
    const fn with_usage(source_units: u64, source_bytes: u64) -> Self {
        Self {
            source_units,
            source_bytes,
        }
    }
}

fn check_foundation_file_bytes(origin: &SourceOrigin, file_bytes: u64) -> SourceResult<()> {
    if file_bytes > FOUNDATION_MAX_SOURCE_FILE_BYTES {
        return Err(foundation_resource_error(
            origin.clone(),
            "source-file-bytes",
            file_bytes,
            FOUNDATION_MAX_SOURCE_FILE_BYTES,
        ));
    }
    Ok(())
}

fn foundation_resource_error(
    origin: SourceOrigin,
    category: &str,
    attempted: u64,
    limit: u64,
) -> SourceDiagnostic {
    SourceDiagnostic::new(
        "LKJ-SRC-LIMIT",
        DiagnosticCategory::ResourceLimit,
        format!(
            "Semantic Source Foundation V1 resource limit: category={category}; attempted={attempted}; limit={limit}"
        ),
        origin,
        SourceSpan::zero(),
    )
}

/// Validate one canonical relative in-memory Edition 1 source unit.
pub fn validate(
    source: &str,
    logical_path: &str,
    limits: &Limits,
) -> SourceResult<ValidatedSourceTree> {
    let origin = validate_logical_source_path(logical_path)?;
    validate_one_source(source, PathBuf::from(logical_path), origin, limits)
}

fn validate_one_source(
    source: &str,
    path: PathBuf,
    origin: SourceOrigin,
    limits: &Limits,
) -> SourceResult<ValidatedSourceTree> {
    let source_len = u64::try_from(source.len()).map_err(|_| {
        foundation_resource_error(
            origin.clone(),
            "source-file-bytes",
            u64::MAX,
            FOUNDATION_MAX_SOURCE_FILE_BYTES,
        )
    })?;
    let mut budget = SourceFoundationBudget::default();
    budget.check_metadata(&origin, source_len)?;
    budget.record_read(&origin, source_len)?;
    let parsed = parser::parse_file(source, origin.clone(), path.clone(), limits)?;
    finish_tree(path, origin, vec![parsed])
}

/// Load, contain, parse, and validate a complete import closure.
///
/// Files are returned in deterministic dependency-first DFS order; imports in
/// each file are visited in source order. Loading uses an explicit stack, so a
/// legal deep import closure does not consume the native call stack.
pub fn load(path: &Path, limits: &Limits) -> SourceResult<ValidatedSourceTree> {
    load::load_with_metrics(path, limits).map(|(tree, _)| tree)
}

pub(crate) fn load_with_metrics(
    path: &Path,
    limits: &Limits,
) -> SourceResult<(ValidatedSourceTree, LoadMetrics)> {
    load::load_with_metrics(path, limits)
}

pub(crate) fn validate_for_compiler(
    source: &str,
    logical_path: &str,
    limits: &Limits,
) -> Result<ValidatedSourceTree> {
    validate(source, logical_path, limits).map_err(SourceDiagnostic::into_core)
}

pub(crate) fn load_for_compiler(path: &Path, limits: &Limits) -> Result<ValidatedSourceTree> {
    load(path, limits).map_err(SourceDiagnostic::into_core)
}

pub(crate) fn ensure_source_path_for_compiler(path: &Path) -> Result<()> {
    load::ensure_source_path(path).map_err(SourceDiagnostic::into_core)
}

#[cfg(test)]
pub(crate) fn validate_source_set_for_analysis(
    files: &[(&str, &str)],
    root: &str,
    limits: &Limits,
) -> Result<ValidatedSourceTree> {
    let mut parsed = Vec::with_capacity(files.len());
    for (path, source) in files {
        let origin = SourceOrigin::in_memory(path);
        parsed.push(
            parser::parse_file(source, origin, PathBuf::from(path), limits)
                .map_err(SourceDiagnostic::into_core)?,
        );
    }
    let root_origin = SourceOrigin::in_memory(root);
    finish_tree(PathBuf::from(root), root_origin, parsed).map_err(SourceDiagnostic::into_core)
}

pub(crate) fn finish_tree(
    root: PathBuf,
    root_origin: SourceOrigin,
    files: Vec<SourceFile>,
) -> SourceResult<ValidatedSourceTree> {
    let mut canonical = Vec::new();
    append_framed(&mut canonical, SEMANTIC_SOURCE_FOUNDATION_SCHEMA.as_bytes());
    append_framed(
        &mut canonical,
        &SEMANTIC_SOURCE_FOUNDATION_SCHEMA_VERSION.to_be_bytes(),
    );
    append_framed(&mut canonical, &SOURCE_EDITION.to_be_bytes());
    let mut ordered: Vec<usize> = (0..files.len()).collect();
    ordered.sort_by(|left, right| {
        files[*left]
            .origin
            .logical_path
            .cmp(&files[*right].origin.logical_path)
    });
    let mut logical_origins: HashMap<String, SourceOrigin> = HashMap::new();
    for index in &ordered {
        let file = &files[*index];
        if let Some(first_origin) = logical_origins.get(&file.origin.logical_path) {
            return Err(SourceDiagnostic::new(
                "LKJ-SRC-LOAD",
                DiagnosticCategory::SourceLoading,
                format!(
                    "distinct source units have duplicate logical origin {}",
                    file.origin.logical_path
                ),
                file.origin.clone(),
                SourceSpan::zero(),
            )
            .with_related(
                "first source unit",
                first_origin.clone(),
                SourceSpan::zero(),
            ));
        }
        logical_origins.insert(file.origin.logical_path.clone(), file.origin.clone());
    }
    for index in &ordered {
        let file = &files[*index];
        append_framed(&mut canonical, file.origin.logical_path.as_bytes());
        append_framed(&mut canonical, &file.exact_source_len.to_be_bytes());
        append_framed(&mut canonical, &file.exact_source_sha256);
    }
    let revision = RevisionId(lkjscript_core::sha256(&canonical));

    let mut nodes = Vec::new();
    let mut top_ids: HashMap<(usize, usize), NodeId> = HashMap::new();
    for file_index in &ordered {
        let file = &files[*file_index];
        for (form_index, form) in file.syntax.iter().enumerate() {
            let id = flatten_node(form, &file.origin, revision, None, &mut nodes)?;
            top_ids.insert((*file_index, form_index), id);
        }
    }

    let mut declarations = Vec::new();
    let mut exact_keys: HashMap<Vec<u8>, (SourceOrigin, SourceSpan)> = HashMap::new();
    let mut global_names: HashMap<String, (DeclarationKind, SourceOrigin, SourceSpan)> =
        HashMap::new();
    for file_index in &ordered {
        let file = &files[*file_index];
        for (form_index, form) in file.syntax.iter().enumerate() {
            let Some((kind, name)) = declaration_identity(form) else {
                continue;
            };
            if matches!(
                kind,
                DeclarationKind::Function | DeclarationKind::Product | DeclarationKind::Trait
            ) && !parser::is_source_identifier(&name)
            {
                return Err(SourceDiagnostic::new(
                    "LKJ-DECL-NAME",
                    DiagnosticCategory::Declaration,
                    format!(
                        "{} declaration name {name:?} is not a spellable source identifier",
                        kind.as_str()
                    ),
                    file.origin.clone(),
                    form.span,
                ));
            }
            if matches!(
                kind,
                DeclarationKind::Function | DeclarationKind::Product | DeclarationKind::Trait
            ) {
                if let Some((first_kind, first_origin, first_span)) = global_names.get(&name) {
                    let message = if *first_kind == kind {
                        format!("duplicate {} declaration {name}", kind.as_str())
                    } else if matches!(
                        (*first_kind, kind),
                        (DeclarationKind::Trait, DeclarationKind::Product)
                            | (DeclarationKind::Product, DeclarationKind::Trait)
                    ) {
                        format!("product {name} collides with a trait")
                    } else {
                        format!(
                            "duplicate global declaration {name}: {} conflicts with {}",
                            kind.as_str(),
                            first_kind.as_str()
                        )
                    };
                    return Err(SourceDiagnostic::new(
                        "LKJ-DECL-DUPLICATE",
                        DiagnosticCategory::Declaration,
                        message,
                        file.origin.clone(),
                        form.span,
                    )
                    .with_related(
                        "first declaration",
                        first_origin.clone(),
                        *first_span,
                    ));
                }
                global_names.insert(name.clone(), (kind, file.origin.clone(), form.span));
            }
            let exact = declaration_key_bytes(&file.origin.logical_path, kind, &name);
            if let Some((first_origin, first_span)) = exact_keys.get(&exact) {
                return Err(SourceDiagnostic::new(
                    "LKJ-DECL-DUPLICATE",
                    DiagnosticCategory::Declaration,
                    format!("duplicate {} declaration {name}", kind.as_str()),
                    file.origin.clone(),
                    form.span,
                )
                .with_related(
                    "first declaration",
                    first_origin.clone(),
                    *first_span,
                ));
            }
            exact_keys.insert(exact.clone(), (file.origin.clone(), form.span));
            let node = top_ids
                .get(&(*file_index, form_index))
                .copied()
                .ok_or_else(|| {
                    SourceDiagnostic::generic(file.origin.clone(), "missing dense declaration node")
                })?;
            declarations.push(DeclarationSummary {
                key: DeclarationKey {
                    digest: lkjscript_core::sha256(&exact),
                    exact_identity: exact,
                    canonical_identity: declaration_key_human_identity(
                        &file.origin.logical_path,
                        kind,
                        &name,
                    ),
                },
                kind,
                name,
                origin: file.origin.clone(),
                span: form.span,
                node,
            });
        }
    }
    declarations.sort_by(|left, right| left.key.cmp(&right.key));
    let origins = ordered
        .iter()
        .map(|index| files[*index].origin.clone())
        .collect();
    Ok(ValidatedSourceTree {
        revision,
        root,
        root_origin,
        files,
        origins,
        declarations,
        nodes,
    })
}

fn flatten_node(
    node: &SourceNode,
    origin: &SourceOrigin,
    revision: RevisionId,
    parent: Option<NodeId>,
    output: &mut Vec<NodeSummary>,
) -> SourceResult<NodeId> {
    let raw = u32::try_from(output.len()).map_err(|_| {
        SourceDiagnostic::generic(origin.clone(), "too many source nodes for NodeId")
    })?;
    let id = NodeId {
        revision,
        index: raw,
    };
    let (kind, label) = match &node.kind {
        SyntaxKind::I64 { value } => (NodeKind::I64Literal, Some(value.to_string())),
        SyntaxKind::F64 { value } => (NodeKind::F64Literal, Some(format::format_f64(*value))),
        SyntaxKind::Bool { value } => (NodeKind::BoolLiteral, Some(value.to_string())),
        SyntaxKind::Unit => (NodeKind::UnitLiteral, None),
        SyntaxKind::Str { value } => (NodeKind::StringLiteral, Some(value.clone())),
        SyntaxKind::Symbol { name } => (NodeKind::Symbol, Some(name.clone())),
        SyntaxKind::Call { name } => (NodeKind::Call, Some(name.clone())),
    };
    output.push(NodeSummary {
        id,
        kind,
        label,
        origin: origin.clone(),
        span: node.span,
        parent,
        children: Vec::new(),
    });
    let mut children = Vec::with_capacity(node.children.len());
    for child in &node.children {
        children.push(flatten_node(child, origin, revision, Some(id), output)?);
    }
    if let Some(summary) = output.get_mut(raw as usize) {
        summary.children = children;
    }
    Ok(id)
}

fn declaration_identity(node: &SourceNode) -> Option<(DeclarationKind, String)> {
    let SyntaxKind::Call { name } = &node.kind else {
        return None;
    };
    match name.as_str() {
        "main" => Some((DeclarationKind::Main, "$main".into())),
        "def" => declaration_name(node).map(|name| (DeclarationKind::Function, name)),
        "product" => declaration_name(node).map(|name| (DeclarationKind::Product, name)),
        "trait" => declaration_name(node).map(|name| (DeclarationKind::Trait, name)),
        "impl" => Some((
            DeclarationKind::Implementation,
            node.children
                .iter()
                .map(format::format_node_identity)
                .collect::<Vec<_>>()
                .join("|"),
        )),
        _ => None,
    }
}

fn declaration_name(node: &SourceNode) -> Option<String> {
    let first = node.children.first()?;
    let SyntaxKind::Call { name } = &first.kind else {
        return None;
    };
    if name != "name" || first.children.len() != 1 {
        return None;
    }
    match &first.children[0].kind {
        SyntaxKind::Str { value } => Some(value.clone()),
        SyntaxKind::Symbol { name } | SyntaxKind::Call { name } => Some(name.clone()),
        _ => None,
    }
}

fn declaration_key_bytes(logical_path: &str, kind: DeclarationKind, name: &str) -> Vec<u8> {
    let mut exact = Vec::new();
    for field in [
        SEMANTIC_SOURCE_FOUNDATION_SCHEMA.as_bytes(),
        SEMANTIC_SOURCE_FOUNDATION_SCHEMA_VERSION
            .to_be_bytes()
            .as_slice(),
        SOURCE_EDITION.to_be_bytes().as_slice(),
        b"edition1-root".as_slice(),
        logical_path.as_bytes(),
        kind.as_str().as_bytes(),
        name.as_bytes(),
    ] {
        append_framed(&mut exact, field);
    }
    exact
}

fn declaration_key_human_identity(logical_path: &str, kind: DeclarationKind, name: &str) -> String {
    format!(
        "schema={};version={};edition={};package={};origin={};kind={};name={}",
        escape_compact(SEMANTIC_SOURCE_FOUNDATION_SCHEMA),
        SEMANTIC_SOURCE_FOUNDATION_SCHEMA_VERSION,
        SOURCE_EDITION,
        escape_compact("edition1-root"),
        escape_compact(logical_path),
        escape_compact(kind.as_str()),
        escape_compact(name),
    )
}

fn append_framed(output: &mut Vec<u8>, bytes: &[u8]) {
    output.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
    output.extend_from_slice(bytes);
}

fn validate_logical_source_path(logical_path: &str) -> SourceResult<SourceOrigin> {
    let raw_origin = SourceOrigin {
        logical_path: logical_path.to_string(),
        host_containment_path: None,
    };
    let path = Path::new(logical_path);
    let mut pieces = Vec::new();
    let canonical_components = path.components().all(|component| {
        let std::path::Component::Normal(piece) = component else {
            return false;
        };
        let Some(piece) = piece.to_str() else {
            return false;
        };
        if piece.starts_with('.') {
            return false;
        }
        pieces.push(piece);
        true
    });
    if logical_path.is_empty()
        || logical_path.contains('\\')
        || path.is_absolute()
        || path.extension().and_then(|extension| extension.to_str())
            != Some(crate::SOURCE_EXTENSION)
        || !canonical_components
        || pieces.join("/") != logical_path
    {
        return Err(SourceDiagnostic::loading(
            raw_origin,
            format!(
                "logical source path must be a canonical relative non-dot .{} path: {logical_path}",
                crate::SOURCE_EXTENSION
            ),
        ));
    }
    Ok(SourceOrigin {
        logical_path: logical_path.to_string(),
        host_containment_path: None,
    })
}

fn canonical_logical_path(path: &Path) -> String {
    let mut pieces = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::Normal(piece) => {
                if let Some(piece) = piece.to_str() {
                    pieces.push(piece);
                }
            }
            std::path::Component::ParentDir => {
                pieces.pop();
            }
            std::path::Component::CurDir
            | std::path::Component::RootDir
            | std::path::Component::Prefix(_) => {}
        }
    }
    if pieces.is_empty() {
        "source.lkjscript".into()
    } else {
        pieces.join("/")
    }
}

fn escape_compact(text: &str) -> String {
    text.replace('%', "%25")
        .replace(';', "%3B")
        .replace('=', "%3D")
        .replace('\n', "%0A")
        .replace('\r', "%0D")
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
mod tests;
