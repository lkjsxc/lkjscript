//! Canonical normalized compiler-unit and typed-bytecode records.

use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    BlobObjectDigest, CaseReference, ComparisonPolicy, DeclarationReference, ExternalVisibility,
    FieldReference, Idempotency, ImplementationName, Name, OperationReference, OwnerKey, OwnerKind,
    PackageId, ParameterUse, PortReference, RequirementReference, ResourceLimit, TypeObjectDigest,
};
use crate::platform::package::RunnerKind;
use crate::platform::semantic_id::{HttpRouteId, ParameterId, TypeParameterId};
use crate::platform::storage::object::{ObjectDomain, ObjectKey};
use crate::platform::witness::SemanticDigest;
use bincode::{Decode, Encode};
use std::collections::BTreeSet;
use std::fmt;

pub const COMPILER_UNIT_CONTRACT_IDENTITY: &str = "lkjscript-compiler-unit-5";
pub const COMPILER_UNIT_CONTRACT_VERSION: u16 = 5;
pub const BYTECODE_CONTRACT_IDENTITY: &str = "lkjscript-bytecode-3";
pub const BYTECODE_CONTRACT_VERSION: u16 = 3;
pub(crate) const COMPILER_UNIT_MAGIC: [u8; 8] = *b"LKJCUN05";
pub(crate) const COMPILER_UNIT_ENVELOPE_DOMAIN: &str = "lkjscript.compiler-unit-envelope.v5";
pub(crate) const COMPILER_UNIT_KEY_DOMAIN: &str = "lkjscript.compiler-unit-key.v5";
pub(crate) const MAXIMUM_COMPILER_UNIT_BYTES: usize = 8 * 1024 * 1024;
pub(crate) const MAXIMUM_COMPILER_UNIT_ITEMS: usize = 1_000_000;

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CompilationUnitKey([u8; 32]);

impl CompilationUnitKey {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }

    pub(crate) fn derive(
        source: &CompilationSource,
        optimization: OptimizationPolicy,
    ) -> Result<Self, Diagnostic> {
        let core = CompilationKeyCore {
            compiler_contract_version: COMPILER_UNIT_CONTRACT_VERSION,
            bytecode_contract_version: BYTECODE_CONTRACT_VERSION,
            graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
            source: source.clone(),
            optimization,
        };
        let bytes = bincode::encode_to_vec(core, bincode::config::standard()).map_err(|error| {
            unit_error(
                DiagnosticClass::Infrastructure,
                "compiler_unit_key_encode",
                format!("failed to encode compiler-unit key: {error}"),
            )
        })?;
        let mut hasher = blake3::Hasher::new_derive_key(COMPILER_UNIT_KEY_DOMAIN);
        hasher.update(&(bytes.len() as u64).to_be_bytes());
        hasher.update(&bytes);
        Ok(Self(*hasher.finalize().as_bytes()))
    }
}

impl fmt::Display for CompilationUnitKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("compiler_unit_key_")?;
        formatter.write_str(&crate::platform::semantic_id::encode_hex(&self.0))
    }
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
struct CompilationKeyCore {
    compiler_contract_version: u16,
    bytecode_contract_version: u16,
    graph_contract_version: u16,
    source: CompilationSource,
    optimization: OptimizationPolicy,
}

/// Summary dimensions that fully bind reusable executable lowering while deliberately excluding
/// mutable presentation and namespace state.
#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct CompilationSource {
    pub package: PackageId,
    pub owner: OwnerKey,
    pub kind: OwnerKind,
    pub semantic_interface: SemanticDigest,
    pub implementation: SemanticDigest,
    pub type_digest: SemanticDigest,
    pub effect: SemanticDigest,
    pub capability: SemanticDigest,
    pub test: Option<SemanticDigest>,
    pub validation_dependencies: SemanticDigest,
}

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, PartialEq)]
pub enum OptimizationPolicy {
    DeterministicBaseline,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct CompilationUnit {
    pub contract_version: u16,
    pub graph_contract_version: u16,
    pub bytecode_contract_version: u16,
    pub key: CompilationUnitKey,
    pub source: CompilationSource,
    pub optimization: OptimizationPolicy,
    pub tables: CompilationTables,
    pub payload: CompilationPayload,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct CompilationTables {
    pub declarations: Vec<DeclarationReference>,
    pub fields: Vec<FieldReference>,
    pub cases: Vec<CaseReference>,
    pub requirements: Vec<RequirementReference>,
    pub operations: Vec<OperationReference>,
    pub ports: Vec<PortReference>,
    pub types: Vec<TypeObjectDigest>,
    pub structural_names: Vec<Name>,
    pub texts: Vec<CompiledText>,
}

#[derive(Clone, Debug, Decode, Encode, Eq, Ord, PartialEq, PartialOrd)]
pub enum CompiledText {
    Inline(String),
    Blob {
        digest: BlobObjectDigest,
        bytes: u64,
    },
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum CompilationPayload {
    Record {
        fields: Vec<CompiledFieldLayout>,
    },
    Variant {
        cases: Vec<CompiledCaseLayout>,
    },
    Interface {
        operations: Vec<CompiledOperationLayout>,
    },
    External {
        signature: CompiledSignature,
        implementation: ImplementationName,
    },
    Function {
        signature: CompiledSignature,
        code: CompiledCode,
    },
    Constant {
        ty: u32,
        code: CompiledCode,
    },
    Component {
        requirements: Vec<CompiledRequirement>,
        ports: Vec<CompiledPort>,
    },
    Test {
        actual: CompiledCode,
        expected: CompiledCode,
        comparison: ComparisonPolicy,
    },
    Target {
        component: u32,
        port: Option<u32>,
        routes: Vec<CompiledHttpRoute>,
        runner: RunnerKind,
    },
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct CompiledHttpRoute {
    pub route: HttpRouteId,
    pub method: String,
    pub selector: crate::platform::kernel::HttpRouteSelector,
    pub port: u32,
    pub capture_parameters: Vec<ParameterId>,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct CompiledSignature {
    pub type_parameters: Vec<TypeParameterId>,
    pub parameters: Vec<CompiledParameter>,
    pub result: u32,
    pub task_requirements: Vec<u32>,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct CompiledParameter {
    pub parameter: ParameterId,
    pub name: Name,
    pub ty: u32,
    pub use_mode: ParameterUse,
    pub resource_requirement: Option<u32>,
}

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, PartialEq)]
pub struct CompiledFieldLayout {
    pub field: u32,
    pub ty: u32,
}

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, PartialEq)]
pub struct CompiledCaseLayout {
    pub case: u32,
    pub payload: Option<u32>,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct CompiledOperationLayout {
    pub operation: u32,
    pub parameters: Vec<CompiledParameter>,
    pub result: u32,
    pub idempotency: Idempotency,
    pub external_visibility: ExternalVisibility,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct CompiledRequirement {
    pub requirement: u32,
    pub interface: u32,
    pub operations: Vec<u32>,
    pub limits: Vec<ResourceLimit>,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct CompiledPort {
    pub port: u32,
    pub function_type: u32,
    pub implementation: CompiledPortImplementation,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum CompiledPortImplementation {
    Function(u32),
    Expression(CompiledCode),
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct CompiledCode {
    pub parameter_count: u32,
    pub local_count: u32,
    pub instructions: Vec<CompiledInstruction>,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum CompiledInstruction {
    Unit,
    Bool(bool),
    I64(i64),
    Text(u32),
    StaticText(u32),
    LoadLocal {
        local: u32,
        use_mode: ParameterUse,
    },
    StoreLocal(u32),
    Drop,
    JumpIfFalse(u32),
    Jump(u32),
    Call {
        function: u32,
        type_arguments: Vec<u32>,
        arguments: u32,
    },
    FunctionValue {
        function: u32,
        type_arguments: Vec<u32>,
    },
    Invoke {
        arguments: u32,
    },
    Record {
        nominal_type: Option<u32>,
        fields: Vec<CompiledFieldSelector>,
    },
    Variant {
        case: u32,
        has_payload: bool,
    },
    Field(CompiledFieldSelector),
    List {
        item_type: u32,
        items: u32,
    },
    Map {
        key_type: u32,
        value_type: u32,
        entries: u32,
    },
    SwitchVariant(Vec<CompiledVariantJump>),
    Perform {
        requirement: u32,
        operation: u32,
        arguments: u32,
    },
    BeginTransaction {
        requirement: u32,
        binding: u32,
    },
    CommitTransaction {
        requirement: u32,
        binding: u32,
    },
    Return,
}

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, PartialEq)]
pub enum CompiledFieldSelector {
    Nominal(u32),
    Structural(u32),
}

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, PartialEq)]
pub struct CompiledVariantJump {
    pub case: u32,
    pub target: u32,
    pub binding_local: Option<u32>,
}

impl CompilationUnit {
    pub fn encode(&self) -> Result<(ObjectKey, Vec<u8>), Diagnostic> {
        self.validate()?;
        let bytes = crate::platform::packed::encode(
            COMPILER_UNIT_MAGIC,
            COMPILER_UNIT_ENVELOPE_DOMAIN,
            self,
            MAXIMUM_COMPILER_UNIT_BYTES,
        )?;
        Ok((
            ObjectKey::for_bytes(ObjectDomain::CompilerUnit, &bytes),
            bytes,
        ))
    }

    pub fn decode(bytes: &[u8], expected: ObjectKey) -> Result<Self, Diagnostic> {
        if expected.domain != ObjectDomain::CompilerUnit
            || ObjectKey::for_bytes(ObjectDomain::CompilerUnit, bytes) != expected
        {
            return Err(unit_error(
                DiagnosticClass::Corrupt,
                "compiler_unit_digest",
                "compiler-unit bytes disagree with their exact object-domain digest",
            ));
        }
        let unit: Self = crate::platform::packed::decode(
            bytes,
            COMPILER_UNIT_MAGIC,
            COMPILER_UNIT_ENVELOPE_DOMAIN,
            MAXIMUM_COMPILER_UNIT_BYTES,
        )?;
        unit.validate()?;
        let (actual, canonical) = unit.encode()?;
        if actual != expected || canonical != bytes {
            return Err(unit_error(
                DiagnosticClass::Corrupt,
                "compiler_unit_canonical",
                "compiler unit does not use its canonical current encoding",
            ));
        }
        Ok(unit)
    }

    pub(crate) fn validate(&self) -> Result<(), Diagnostic> {
        if self.contract_version != COMPILER_UNIT_CONTRACT_VERSION
            || self.bytecode_contract_version != BYTECODE_CONTRACT_VERSION
            || self.graph_contract_version
                != crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION
        {
            return Err(unit_error(
                DiagnosticClass::Source,
                "compiler_unit_contract",
                "compiler unit uses a predecessor or foreign contract",
            ));
        }
        if self.source.package.bytes() == [0; 16] {
            return Err(unit_error(
                DiagnosticClass::Corrupt,
                "compiler_unit_package",
                "compiler unit has the reserved zero package identity",
            ));
        }
        if !self.source.kind.accepts_owner(self.source.owner) {
            return Err(unit_error(
                DiagnosticClass::Corrupt,
                "compiler_unit_owner_kind",
                "compiler-unit source kind disagrees with its stable owner domain",
            ));
        }
        if matches!(self.source.kind, OwnerKind::Test) != self.source.test.is_some() {
            return Err(unit_error(
                DiagnosticClass::Corrupt,
                "compiler_unit_test_digest",
                "compiler-unit source test digest presence disagrees with its owner kind",
            ));
        }
        let expected_key = CompilationUnitKey::derive(&self.source, self.optimization)?;
        if self.key != expected_key {
            return Err(unit_error(
                DiagnosticClass::Corrupt,
                "compiler_unit_key",
                "compiler-unit key disagrees with its exact source summary dimensions",
            ));
        }
        self.tables.validate()?;
        self.payload.validate(&self.source, &self.tables)
    }
}

impl CompilationTables {
    fn validate(&self) -> Result<(), Diagnostic> {
        for (label, length) in [
            ("declarations", self.declarations.len()),
            ("fields", self.fields.len()),
            ("cases", self.cases.len()),
            ("requirements", self.requirements.len()),
            ("operations", self.operations.len()),
            ("ports", self.ports.len()),
            ("types", self.types.len()),
            ("structural names", self.structural_names.len()),
            ("texts", self.texts.len()),
        ] {
            require_item_count(label, length, true)?;
        }
        require_unique("declaration relocation", &self.declarations)?;
        require_unique("field relocation", &self.fields)?;
        require_unique("case relocation", &self.cases)?;
        require_unique("requirement relocation", &self.requirements)?;
        require_unique("operation relocation", &self.operations)?;
        require_unique("port relocation", &self.ports)?;
        require_unique("type relocation", &self.types)?;
        require_unique("structural field name", &self.structural_names)?;
        require_unique("text constant", &self.texts)?;
        for text in &self.texts {
            match text {
                CompiledText::Inline(value)
                    if value.len()
                        <= crate::platform::kernel::contract::MAXIMUM_INLINE_TEXT_BYTES => {}
                CompiledText::Inline(_) => {
                    return Err(unit_corrupt(
                        "compiler_unit_text_length",
                        "compiled inline text exceeds the Graph 10 inline bound",
                    ));
                }
                CompiledText::Blob { bytes, .. }
                    if *bytes > 0
                        && *bytes
                            <= crate::platform::storage::object::ObjectDomain::Blob.maximum_bytes()
                                as u64 => {}
                CompiledText::Blob { .. } => {
                    return Err(unit_corrupt(
                        "compiler_unit_blob_length",
                        "compiled blob text length is outside the object-domain bound",
                    ));
                }
            }
        }
        Ok(())
    }
}

impl CompilationPayload {
    fn validate(
        &self,
        source: &CompilationSource,
        tables: &CompilationTables,
    ) -> Result<(), Diagnostic> {
        match self {
            Self::Record { fields } => {
                require_kind(source, OwnerKind::Record)?;
                require_item_count("compiled record fields", fields.len(), false)?;
                for field in fields {
                    require_index("record field", field.field, tables.fields.len())?;
                    require_index("record field type", field.ty, tables.types.len())?;
                    if tables.fields[field.field as usize].package != source.package {
                        return Err(unit_corrupt(
                            "compiler_unit_field_identity",
                            "compiled field layout uses a foreign package relocation",
                        ));
                    }
                }
            }
            Self::Variant { cases } => {
                require_kind(source, OwnerKind::Variant)?;
                require_item_count("compiled variant cases", cases.len(), false)?;
                for case in cases {
                    require_index("variant case", case.case, tables.cases.len())?;
                    if let Some(payload) = case.payload {
                        require_index("variant case payload", payload, tables.types.len())?;
                    }
                    if tables.cases[case.case as usize].package != source.package {
                        return Err(unit_corrupt(
                            "compiler_unit_case_identity",
                            "compiled case layout uses a foreign package relocation",
                        ));
                    }
                }
            }
            Self::Interface { operations } => {
                require_kind(source, OwnerKind::Interface)?;
                require_item_count("compiled interface operations", operations.len(), false)?;
                for operation in operations {
                    operation.validate(source, tables)?;
                }
            }
            Self::External { signature, .. } => {
                require_kind(source, OwnerKind::External)?;
                signature.validate(tables, source.kind)?;
            }
            Self::Function { signature, code } => {
                if !matches!(
                    source.kind,
                    OwnerKind::PureFunction | OwnerKind::TaskFunction
                ) {
                    return Err(unit_corrupt(
                        "compiler_unit_function_kind",
                        "function payload is bound to another owner kind",
                    ));
                }
                signature.validate(tables, source.kind)?;
                code.validate(tables)?;
                if code.parameter_count as usize != signature.parameters.len() {
                    return Err(unit_corrupt(
                        "compiler_unit_parameter_count",
                        "compiled function local parameters disagree with its signature",
                    ));
                }
                if matches!(source.kind, OwnerKind::PureFunction)
                    && !signature.task_requirements.is_empty()
                {
                    return Err(unit_corrupt(
                        "compiler_unit_pure_requirements",
                        "pure compiled function declares task requirements",
                    ));
                }
            }
            Self::Constant { ty, code } => {
                require_kind(source, OwnerKind::Constant)?;
                require_index("constant type", *ty, tables.types.len())?;
                code.validate(tables)?;
                if code.parameter_count != 0 {
                    return Err(unit_corrupt(
                        "compiler_unit_constant_parameters",
                        "compiled constant has parameters",
                    ));
                }
            }
            Self::Component {
                requirements,
                ports,
            } => {
                require_kind(source, OwnerKind::Component)?;
                require_item_count("compiled component ports", ports.len(), false)?;
                require_item_count("compiled component requirements", requirements.len(), true)?;
                for requirement in requirements {
                    requirement.validate(source, tables)?;
                }
                for port in ports {
                    port.validate(source, tables)?;
                }
            }
            Self::Test {
                actual, expected, ..
            } => {
                require_kind(source, OwnerKind::Test)?;
                actual.validate(tables)?;
                expected.validate(tables)?;
                if actual.parameter_count != 0 || expected.parameter_count != 0 {
                    return Err(unit_corrupt(
                        "compiler_unit_test_parameters",
                        "compiled test entries have parameters",
                    ));
                }
            }
            Self::Target {
                component,
                port,
                routes,
                runner,
            } => {
                require_kind(source, OwnerKind::Target)?;
                require_index("target component", *component, tables.declarations.len())?;
                if (*runner == RunnerKind::Http) == port.is_some() {
                    return Err(unit_corrupt(
                        "compiler_unit_target_port_condition",
                        "HTTP target must omit its universal port and non-HTTP target must contain one",
                    ));
                }
                if let Some(port) = port {
                    require_index("target port", *port, tables.ports.len())?;
                }
                if *runner == RunnerKind::Http {
                    validate_compiled_http_routes(routes, tables.ports.len())?;
                } else if !routes.is_empty() {
                    return Err(unit_corrupt(
                        "compiler_unit_non_http_routes",
                        "non-HTTP target contains HTTP routes",
                    ));
                }
            }
        }
        Ok(())
    }
}

fn validate_compiled_http_routes(
    routes: &[CompiledHttpRoute],
    port_count: usize,
) -> Result<(), Diagnostic> {
    use crate::platform::kernel::contract::{
        MAXIMUM_HTTP_PATTERN_SEGMENTS_PER_TARGET, MAXIMUM_HTTP_ROUTE_KEY_BYTES_PER_TARGET,
        MAXIMUM_HTTP_ROUTES_PER_TARGET,
    };
    use crate::platform::kernel::{
        HttpRouteSelector, http_route_pattern_strictly_more_specific, http_route_patterns_overlap,
        http_route_same_pattern_language, http_route_selector_cmp, validate_http_route_method,
    };
    if routes.is_empty() || routes.len() > MAXIMUM_HTTP_ROUTES_PER_TARGET {
        return Err(unit_corrupt(
            "compiler_unit_http_route_count",
            "compiled HTTP target route count is outside the supported bounds",
        ));
    }
    let mut bytes = 0_usize;
    let mut pattern_segments = 0usize;
    let mut identities = BTreeSet::new();
    for (index, route) in routes.iter().enumerate() {
        validate_http_route_method(&route.method).map_err(|_| {
            unit_corrupt(
                "compiler_unit_http_route_key",
                "compiled HTTP route contains an invalid method",
            )
        })?;
        route.selector.validate_local().map_err(|_| {
            unit_corrupt(
                "compiler_unit_http_route_selector",
                "compiled HTTP route contains an invalid selector",
            )
        })?;
        require_index("HTTP route port", route.port, port_count)?;
        if !identities.insert(route.route) {
            return Err(unit_corrupt(
                "compiler_unit_http_route_identity",
                "compiled HTTP target repeats one route identity",
            ));
        }
        bytes = bytes
            .checked_add(route.method.len())
            .and_then(|value| value.checked_add(route.selector.key_bytes()))
            .ok_or_else(|| {
                unit_corrupt(
                    "compiler_unit_http_route_bytes",
                    "compiled HTTP route-key bytes overflowed",
                )
            })?;
        if index > 0 {
            let previous = &routes[index - 1];
            if previous
                .method
                .as_bytes()
                .cmp(route.method.as_bytes())
                .then_with(|| http_route_selector_cmp(&previous.selector, &route.selector))
                != std::cmp::Ordering::Less
            {
                return Err(unit_corrupt(
                    "compiler_unit_http_route_order",
                    "compiled HTTP routes are not in unique canonical key order",
                ));
            }
        }
        if route.capture_parameters.len() != route.selector.capture_count()
            || route
                .capture_parameters
                .iter()
                .collect::<BTreeSet<_>>()
                .len()
                != route.capture_parameters.len()
        {
            return Err(unit_corrupt(
                "compiler_unit_http_route_capture_parameters",
                "compiled HTTP route capture-parameter identities disagree with its selector",
            ));
        }
        if let HttpRouteSelector::Pattern { segments } = &route.selector {
            pattern_segments = pattern_segments
                .checked_add(segments.len())
                .ok_or_else(|| {
                    unit_corrupt(
                        "compiler_unit_http_route_pattern_segments",
                        "compiled HTTP pattern-segment count overflowed",
                    )
                })?;
            if pattern_segments > MAXIMUM_HTTP_PATTERN_SEGMENTS_PER_TARGET {
                return Err(unit_corrupt(
                    "compiler_unit_http_route_pattern_segments",
                    "compiled HTTP routes exceed the target pattern-segment bound",
                ));
            }
        }
    }
    if bytes > MAXIMUM_HTTP_ROUTE_KEY_BYTES_PER_TARGET {
        return Err(unit_corrupt(
            "compiler_unit_http_route_bytes",
            "compiled HTTP route-key bytes exceed the supported bound",
        ));
    }
    for (index, left) in routes.iter().enumerate() {
        for right in routes.iter().skip(index + 1) {
            if left.port == right.port
                && left.selector.capture_names() != right.selector.capture_names()
            {
                return Err(unit_corrupt(
                    "compiler_unit_http_route_shared_port_signature",
                    "compiled HTTP routes sharing a port disagree on capture names",
                ));
            }
            if left.method != right.method {
                continue;
            }
            match (&left.selector, &right.selector) {
                (
                    HttpRouteSelector::Exact { path: left },
                    HttpRouteSelector::Exact { path: right },
                ) if left == right => {
                    return Err(unit_corrupt(
                        "compiler_unit_http_route_duplicate_language",
                        "compiled HTTP exact routes repeat one match language",
                    ));
                }
                (
                    HttpRouteSelector::Pattern {
                        segments: left_segments,
                    },
                    HttpRouteSelector::Pattern {
                        segments: right_segments,
                    },
                ) if http_route_patterns_overlap(left_segments, right_segments)
                    && !http_route_pattern_strictly_more_specific(
                        left_segments,
                        right_segments,
                    )
                    && !http_route_pattern_strictly_more_specific(
                        right_segments,
                        left_segments,
                    ) =>
                {
                    let message = if http_route_same_pattern_language(left_segments, right_segments)
                    {
                        "compiled HTTP patterns repeat one match language"
                    } else {
                        "compiled HTTP patterns overlap without strict specificity"
                    };
                    return Err(unit_corrupt("compiler_unit_http_route_overlap", message));
                }
                _ => {}
            }
        }
    }
    Ok(())
}

impl CompiledSignature {
    fn validate(&self, tables: &CompilationTables, kind: OwnerKind) -> Result<(), Diagnostic> {
        require_item_count("compiled type parameters", self.type_parameters.len(), true)?;
        require_item_count("compiled parameters", self.parameters.len(), true)?;
        require_unique("compiled type parameter", &self.type_parameters)?;
        require_unique(
            "compiled parameter",
            &self
                .parameters
                .iter()
                .map(|parameter| parameter.parameter)
                .collect::<Vec<_>>(),
        )?;
        for parameter in &self.parameters {
            require_index("parameter type", parameter.ty, tables.types.len())?;
            if let Some(requirement) = parameter.resource_requirement {
                require_index(
                    "parameter resource requirement",
                    requirement,
                    tables.requirements.len(),
                )?;
            }
        }
        require_index("signature result", self.result, tables.types.len())?;
        for requirement in &self.task_requirements {
            require_index(
                "signature requirement",
                *requirement,
                tables.requirements.len(),
            )?;
        }
        require_unique("signature requirement", &self.task_requirements)?;
        let bound = self
            .parameters
            .iter()
            .enumerate()
            .filter_map(|(index, parameter)| {
                parameter
                    .resource_requirement
                    .map(|requirement| (index, parameter, requirement))
            })
            .collect::<Vec<_>>();
        if kind == OwnerKind::External
            && self.parameters.iter().any(|parameter| {
                parameter.use_mode != ParameterUse::Unrestricted
                    || parameter.resource_requirement.is_some()
            })
        {
            return Err(unit_corrupt(
                "compiler_unit_external_resource_parameter",
                "compiled external parameters cannot use or bind affine resources",
            ));
        }
        if bound.len() > 1 {
            return Err(unit_corrupt(
                "compiler_unit_resource_parameter_count",
                "compiled function signature binds more than one resource parameter",
            ));
        }
        if let Some((index, parameter, requirement)) = bound.first().copied()
            && (kind != OwnerKind::TaskFunction
                || index.saturating_add(1) != self.parameters.len()
                || parameter.use_mode != ParameterUse::Consume
                || !self.task_requirements.contains(&requirement))
        {
            return Err(unit_corrupt(
                "compiler_unit_resource_parameter_shape",
                "compiled resource parameter is not one final consume parameter bound to its task requirement",
            ));
        }
        if self.parameters.iter().any(|parameter| {
            parameter.use_mode != ParameterUse::Unrestricted
                && parameter.resource_requirement.is_none()
        }) {
            return Err(unit_corrupt(
                "compiler_unit_function_parameter_use",
                "compiled function parameter use requires an exact resource binding",
            ));
        }
        Ok(())
    }
}

impl CompiledOperationLayout {
    fn validate(
        &self,
        source: &CompilationSource,
        tables: &CompilationTables,
    ) -> Result<(), Diagnostic> {
        require_index(
            "interface operation",
            self.operation,
            tables.operations.len(),
        )?;
        if tables.operations[self.operation as usize].package != source.package {
            return Err(unit_corrupt(
                "compiler_unit_operation_identity",
                "compiled operation layout uses a foreign package relocation",
            ));
        }
        require_item_count("operation parameters", self.parameters.len(), true)?;
        for parameter in &self.parameters {
            require_index("operation parameter type", parameter.ty, tables.types.len())?;
            if parameter.resource_requirement.is_some() {
                return Err(unit_corrupt(
                    "compiler_unit_operation_resource_binding",
                    "compiled operation parameter carries a function resource binding",
                ));
            }
        }
        require_index("operation result", self.result, tables.types.len())
    }
}

impl CompiledRequirement {
    fn validate(
        &self,
        source: &CompilationSource,
        tables: &CompilationTables,
    ) -> Result<(), Diagnostic> {
        require_index(
            "component requirement",
            self.requirement,
            tables.requirements.len(),
        )?;
        if tables.requirements[self.requirement as usize].package != source.package {
            return Err(unit_corrupt(
                "compiler_unit_requirement_identity",
                "compiled requirement uses a foreign package relocation",
            ));
        }
        require_index(
            "requirement interface",
            self.interface,
            tables.declarations.len(),
        )?;
        for operation in &self.operations {
            require_index("requirement operation", *operation, tables.operations.len())?;
        }
        require_unique("requirement operation", &self.operations)
    }
}

impl CompiledPort {
    fn validate(
        &self,
        source: &CompilationSource,
        tables: &CompilationTables,
    ) -> Result<(), Diagnostic> {
        require_index("component port", self.port, tables.ports.len())?;
        if tables.ports[self.port as usize].package != source.package {
            return Err(unit_corrupt(
                "compiler_unit_port_identity",
                "compiled port uses a foreign package relocation",
            ));
        }
        require_index("port function type", self.function_type, tables.types.len())?;
        match &self.implementation {
            CompiledPortImplementation::Function(function) => {
                require_index("port function", *function, tables.declarations.len())
            }
            CompiledPortImplementation::Expression(code) => code.validate(tables),
        }
    }
}

impl CompiledCode {
    fn validate(&self, tables: &CompilationTables) -> Result<(), Diagnostic> {
        let instruction_count = self.instructions.len();
        require_item_count("compiled instructions", instruction_count, false)?;
        if self.local_count < self.parameter_count {
            return Err(unit_corrupt(
                "compiler_unit_local_count",
                "compiled local count is smaller than its parameter count",
            ));
        }
        require_runtime_count("compiled parameters", self.parameter_count)?;
        require_runtime_count("compiled locals", self.local_count)?;
        if !matches!(self.instructions.last(), Some(CompiledInstruction::Return))
            || self.instructions[..instruction_count - 1]
                .iter()
                .any(|instruction| matches!(instruction, CompiledInstruction::Return))
        {
            return Err(unit_corrupt(
                "compiler_unit_return",
                "compiled code must have exactly one terminal return instruction",
            ));
        }
        for instruction in &self.instructions {
            instruction.validate(self, tables)?;
        }
        verify_stack(self)
    }
}

impl CompiledInstruction {
    fn validate(&self, code: &CompiledCode, tables: &CompilationTables) -> Result<(), Diagnostic> {
        match self {
            Self::Text(index) | Self::StaticText(index) => {
                require_index("text constant", *index, tables.texts.len())
            }
            Self::LoadLocal { local, .. } | Self::StoreLocal(local) => {
                require_index("local", *local, code.local_count as usize)
            }
            Self::JumpIfFalse(target) | Self::Jump(target) => {
                require_index("jump target", *target, code.instructions.len())
            }
            Self::Call {
                function,
                type_arguments,
                arguments,
            } => {
                require_runtime_count("call arguments", *arguments)?;
                require_item_count("call type arguments", type_arguments.len(), true)?;
                require_index("function relocation", *function, tables.declarations.len())?;
                for ty in type_arguments {
                    require_index("type argument", *ty, tables.types.len())?;
                }
                Ok(())
            }
            Self::FunctionValue {
                function,
                type_arguments,
            } => {
                require_item_count("function type arguments", type_arguments.len(), true)?;
                require_index("function relocation", *function, tables.declarations.len())?;
                for ty in type_arguments {
                    require_index("type argument", *ty, tables.types.len())?;
                }
                Ok(())
            }
            Self::Record {
                nominal_type,
                fields,
            } => {
                if let Some(declaration) = nominal_type {
                    require_index(
                        "nominal record declaration",
                        *declaration,
                        tables.declarations.len(),
                    )?;
                }
                require_item_count("record expression fields", fields.len(), false)?;
                for field in fields {
                    field.validate(tables)?;
                }
                Ok(())
            }
            Self::Variant { case, .. } => require_index("variant case", *case, tables.cases.len()),
            Self::Field(selector) => selector.validate(tables),
            Self::List { item_type, items } => {
                require_runtime_count("list items", *items)?;
                require_index("list item type", *item_type, tables.types.len())
            }
            Self::Map {
                key_type,
                value_type,
                entries,
            } => {
                require_runtime_count("map entries", *entries)?;
                require_index("map key type", *key_type, tables.types.len())?;
                require_index("map value type", *value_type, tables.types.len())
            }
            Self::SwitchVariant(arms) => {
                require_item_count("variant switch arms", arms.len(), false)?;
                let mut cases = BTreeSet::new();
                for arm in arms {
                    require_index("match case", arm.case, tables.cases.len())?;
                    require_index("match target", arm.target, code.instructions.len())?;
                    if let Some(local) = arm.binding_local {
                        require_index("match payload local", local, code.local_count as usize)?;
                    }
                    if !cases.insert(arm.case) {
                        return Err(unit_corrupt(
                            "compiler_unit_match_case",
                            "compiled variant switch repeats one exact case",
                        ));
                    }
                }
                Ok(())
            }
            Self::Perform {
                requirement,
                operation,
                arguments,
            } => {
                require_runtime_count("capability arguments", *arguments)?;
                require_index(
                    "capability requirement",
                    *requirement,
                    tables.requirements.len(),
                )?;
                require_index("capability operation", *operation, tables.operations.len())
            }
            Self::BeginTransaction {
                requirement,
                binding,
            }
            | Self::CommitTransaction {
                requirement,
                binding,
            } => {
                require_index(
                    "transaction requirement",
                    *requirement,
                    tables.requirements.len(),
                )?;
                require_index("transaction binding", *binding, code.local_count as usize)
            }
            Self::Invoke { arguments } => require_runtime_count("invoke arguments", *arguments),
            Self::Unit | Self::Bool(_) | Self::I64(_) | Self::Drop | Self::Return => Ok(()),
        }
    }
}

impl CompiledFieldSelector {
    fn validate(self, tables: &CompilationTables) -> Result<(), Diagnostic> {
        match self {
            Self::Nominal(field) => require_index("nominal field", field, tables.fields.len()),
            Self::Structural(name) => {
                require_index("structural field name", name, tables.structural_names.len())
            }
        }
    }
}

fn require_kind(source: &CompilationSource, expected: OwnerKind) -> Result<(), Diagnostic> {
    if source.kind != expected {
        return Err(unit_corrupt(
            "compiler_unit_payload_kind",
            format!(
                "compiled payload requires {expected:?}, but source is {:?}",
                source.kind
            ),
        ));
    }
    Ok(())
}

fn require_index(label: &str, index: u32, length: usize) -> Result<(), Diagnostic> {
    if index as usize >= length {
        return Err(unit_corrupt(
            "compiler_unit_index",
            format!("{label} index {index} is outside table length {length}"),
        ));
    }
    Ok(())
}

fn require_item_count(label: &str, count: usize, allow_zero: bool) -> Result<(), Diagnostic> {
    if (!allow_zero && count == 0) || count > MAXIMUM_COMPILER_UNIT_ITEMS {
        return Err(unit_error(
            DiagnosticClass::Resource,
            "compiler_unit_item_count",
            format!("{label} count {count} is outside the compiler-unit bound"),
        ));
    }
    Ok(())
}

fn require_runtime_count(label: &str, count: u32) -> Result<(), Diagnostic> {
    if count as usize > MAXIMUM_COMPILER_UNIT_ITEMS {
        return Err(unit_error(
            DiagnosticClass::Resource,
            "compiler_unit_runtime_count",
            format!("{label} count {count} exceeds the compiler-unit bound"),
        ));
    }
    Ok(())
}

fn verify_stack(code: &CompiledCode) -> Result<(), Diagnostic> {
    let mut pending = vec![(0_usize, 0_usize)];
    let mut depths = vec![None; code.instructions.len()];
    while let Some((instruction_index, depth)) = pending.pop() {
        let slot = depths.get_mut(instruction_index).ok_or_else(|| {
            unit_corrupt(
                "compiler_unit_control_flow",
                "compiled control flow reaches beyond the instruction stream",
            )
        })?;
        if let Some(previous) = *slot {
            if previous != depth {
                return Err(unit_corrupt(
                    "compiler_unit_stack_merge",
                    "compiled control-flow paths merge with different stack depths",
                ));
            }
            continue;
        }
        *slot = Some(depth);
        let instruction = &code.instructions[instruction_index];
        let (consumed, produced) = stack_effect(instruction)?;
        let next_depth = depth
            .checked_sub(consumed)
            .and_then(|depth| depth.checked_add(produced))
            .ok_or_else(|| {
                unit_corrupt(
                    "compiler_unit_stack_underflow",
                    "compiled instruction consumes beneath its operand stack",
                )
            })?;
        if next_depth > MAXIMUM_COMPILER_UNIT_ITEMS {
            return Err(unit_error(
                DiagnosticClass::Resource,
                "compiler_unit_stack_depth",
                "compiled operand stack exceeds the compiler-unit bound",
            ));
        }
        match instruction {
            CompiledInstruction::Return => {
                if depth != 1 {
                    return Err(unit_corrupt(
                        "compiler_unit_return_stack",
                        "compiled return does not consume exactly one result value",
                    ));
                }
            }
            CompiledInstruction::Jump(target) => {
                pending.push((*target as usize, next_depth));
            }
            CompiledInstruction::JumpIfFalse(target) => {
                pending.push((*target as usize, next_depth));
                pending.push((instruction_index + 1, next_depth));
            }
            CompiledInstruction::SwitchVariant(arms) => {
                pending.extend(arms.iter().map(|arm| (arm.target as usize, next_depth)));
            }
            _ => pending.push((instruction_index + 1, next_depth)),
        }
    }
    if depths.iter().any(Option::is_none) {
        return Err(unit_corrupt(
            "compiler_unit_unreachable_instruction",
            "compiled code contains an unreachable instruction",
        ));
    }
    Ok(())
}

fn stack_effect(instruction: &CompiledInstruction) -> Result<(usize, usize), Diagnostic> {
    let count = |value: u32| {
        usize::try_from(value).map_err(|_| {
            unit_error(
                DiagnosticClass::Resource,
                "compiler_unit_stack_count",
                "compiled operand count does not fit this platform",
            )
        })
    };
    Ok(match instruction {
        CompiledInstruction::Unit
        | CompiledInstruction::Bool(_)
        | CompiledInstruction::I64(_)
        | CompiledInstruction::Text(_)
        | CompiledInstruction::StaticText(_)
        | CompiledInstruction::LoadLocal { .. }
        | CompiledInstruction::FunctionValue { .. } => (0, 1),
        CompiledInstruction::StoreLocal(_) | CompiledInstruction::Drop => (1, 0),
        CompiledInstruction::JumpIfFalse(_) => (1, 0),
        CompiledInstruction::Jump(_)
        | CompiledInstruction::BeginTransaction { .. }
        | CompiledInstruction::CommitTransaction { .. } => (0, 0),
        CompiledInstruction::Call { arguments, .. } => (count(*arguments)?, 1),
        CompiledInstruction::Invoke { arguments } => {
            let consumed = count(*arguments)?.checked_add(1).ok_or_else(|| {
                unit_error(
                    DiagnosticClass::Resource,
                    "compiler_unit_stack_count",
                    "invoke operand count overflows",
                )
            })?;
            (consumed, 1)
        }
        CompiledInstruction::Record { fields, .. } => (fields.len(), 1),
        CompiledInstruction::Variant { has_payload, .. } => (usize::from(*has_payload), 1),
        CompiledInstruction::Field(_) => (1, 1),
        CompiledInstruction::List { items, .. } => (count(*items)?, 1),
        CompiledInstruction::Map { entries, .. } => {
            let consumed = count(*entries)?.checked_mul(2).ok_or_else(|| {
                unit_error(
                    DiagnosticClass::Resource,
                    "compiler_unit_stack_count",
                    "map operand count overflows",
                )
            })?;
            (consumed, 1)
        }
        CompiledInstruction::SwitchVariant(_) => (1, 0),
        CompiledInstruction::Perform { arguments, .. } => (count(*arguments)?, 1),
        CompiledInstruction::Return => (1, 0),
    })
}

fn require_unique<T: Ord + Clone>(label: &str, values: &[T]) -> Result<(), Diagnostic> {
    let mut observed = BTreeSet::new();
    for value in values {
        if !observed.insert(value.clone()) {
            return Err(unit_corrupt(
                "compiler_unit_duplicate",
                format!("compiled {label} is duplicated"),
            ));
        }
    }
    Ok(())
}

fn unit_corrupt(code: &'static str, message: impl Into<String>) -> Diagnostic {
    unit_error(DiagnosticClass::Corrupt, code, message)
}

fn unit_error(
    class: DiagnosticClass,
    code: &'static str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}
