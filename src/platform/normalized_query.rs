//! Revision-pinned normalized semantic query model, continuation, execution, and compact output.

use super::contract::{
    MAXIMUM_CLI_RESPONSE_BYTES, MAXIMUM_CLI_RESPONSE_RECORDS, capabilities_snapshot,
};
use super::control::{CompactResponseLimits, CompactResponseWriter};
use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::execution::ExecutionControl;
use super::kernel::{
    EncodedOwnerKey, ExactOwnerKey, Name, NamespaceClass, OwnerKey, OwnerKind, PackageId,
    RelationEdge, RelationEndpoint, RelationKind, owner_namespace,
};
use super::persistent_map::MapRangeControl;
use super::publication::{
    GraphRepository, RepositoryQueryAdmission, RepositoryReadWork, RepositoryRelationQueryRange,
    RepositoryView,
};
use super::semantic_id::{RepositoryId, RevisionId, encode_hex};
use super::witness::{
    NamespaceKey, decode_forward_relation_key, decode_reverse_relation_key, forward_relation_key,
    forward_relation_prefix, reverse_relation_prefix,
};
use base64::Engine;
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

pub const QUERY_CONTRACT_VERSION: u16 = 5;
pub const QUERY_CONTRACT_IDENTITY: &str = "lkjscript-query-5";
pub const DEFAULT_QUERY_ITEMS: u64 = 50;
pub const MAXIMUM_QUERY_ITEMS: u64 = 10_000;
pub const DEFAULT_QUERY_OUTPUT_BYTES: usize = 64 * 1_024;
pub const MINIMUM_QUERY_OUTPUT_BYTES: usize = 1_536;
pub const MAXIMUM_QUERY_OUTPUT_BYTES: usize = 4 * 1_048_576;
pub const MAXIMUM_QUERY_CONTINUATION_BYTES: usize = 320;
const MAXIMUM_QUERY_RESUME_KEY_BYTES: usize = 70;
const MAXIMUM_QUERY_CONTINUATION_DECODED_BYTES: usize = 224;
const QUERY_CONTINUATION_PREFIX: &str = "qcont_";
pub(crate) const QUERY_CONTINUATION_MAGIC_TEXT: &str = "LKJQCT05";
const QUERY_CONTINUATION_MAGIC: [u8; 8] = *b"LKJQCT05";
const QUERY_CONTINUATION_VERSION: u16 = 1;
const QUERY_CONTINUATION_ENVELOPE_VERSION: u16 = 1;
pub(crate) const QUERY_CONTINUATION_INTEGRITY_DOMAIN: &str =
    "lkjscript.normalized-query.continuation-integrity.v1";
pub(crate) const QUERY_SELECTOR_DIGEST_DOMAIN: &str = "lkjscript.normalized-query.selector.v5";
const QUERY_ORDERING_CONTRACT: u8 = 1;
const CONTINUATION_HEADER_BYTES: usize = 18;
const CONTINUATION_CHECKSUM_BYTES: usize = 32;

pub const MINIMUM_CONTEXT_DEPTH: u8 = 1;
pub const MAXIMUM_CONTEXT_DEPTH: u8 = 8;
pub const MAXIMUM_CONTEXT_OWNERS: u64 = 4_096;
pub const MAXIMUM_CONTEXT_RELATIONS: u64 = 16_384;
pub const MAXIMUM_CONTEXT_RELATION_WITNESSES: u64 = 32_768;
const MAXIMUM_CONTEXT_RELATION_RANGES: u64 = MAXIMUM_CONTEXT_OWNERS * 2;
const MAXIMUM_OWNER_MAP_KEY_BYTES: u64 = 17;
const MAXIMUM_RELATION_MAP_KEY_BYTES: u64 = 69;
const MAXIMUM_MAP_LEAF_RECORDS: u64 = (super::persistent_map::MAXIMUM_PAGE_BYTES / 6) as u64;
pub const MAXIMUM_CONTEXT_MAP_PAGES: u64 = MAXIMUM_CONTEXT_OWNERS
    * (MAXIMUM_OWNER_MAP_KEY_BYTES + 1)
    + (MAXIMUM_CONTEXT_RELATION_RANGES + MAXIMUM_CONTEXT_RELATION_WITNESSES)
        * (MAXIMUM_RELATION_MAP_KEY_BYTES + 1);
pub const MAXIMUM_CONTEXT_MAP_BYTES: u64 =
    MAXIMUM_CONTEXT_MAP_PAGES * super::persistent_map::MAXIMUM_PAGE_BYTES as u64;
pub const MAXIMUM_CONTEXT_MAP_ENTRIES: u64 = MAXIMUM_CONTEXT_MAP_PAGES * MAXIMUM_MAP_LEAF_RECORDS;
pub const MAXIMUM_CONTEXT_STORE_OBJECTS: u64 = MAXIMUM_CONTEXT_MAP_PAGES + MAXIMUM_CONTEXT_OWNERS;
pub const MAXIMUM_CONTEXT_STORE_BYTES: u64 =
    MAXIMUM_CONTEXT_MAP_BYTES + MAXIMUM_CONTEXT_OWNERS * 4 * 1_048_576;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct QueryOperationDescriptor {
    pub action: &'static str,
    pub command: &'static str,
    pub usage: &'static str,
    pub positionals: &'static [&'static str],
    pub options: &'static [&'static str],
}

pub(crate) const QUERY_OPERATION_DESCRIPTORS: [QueryOperationDescriptor; 4] = [
    QueryOperationDescriptor {
        action: "owners",
        command: "query.owners",
        usage: "query owners [--kind KIND] [--limit N] [--bytes N] [--continuation TOKEN]",
        positionals: &[],
        options: &["--kind", "--limit", "--bytes", "--continuation"],
    },
    QueryOperationDescriptor {
        action: "find",
        command: "query.find",
        usage: "query find CLASS NAME [--parent OWNER]",
        positionals: &["class", "name"],
        options: &["--parent"],
    },
    QueryOperationDescriptor {
        action: "relations",
        command: "query.relations",
        usage: "query relations OWNER|package --direction incoming|outgoing [--kind KIND] [--limit N] [--bytes N] [--continuation TOKEN]",
        positionals: &["endpoint"],
        options: &[
            "--direction",
            "--kind",
            "--limit",
            "--bytes",
            "--continuation",
        ],
    },
    QueryOperationDescriptor {
        action: "context",
        command: "query.context",
        usage: "query context OWNER --direction incoming|outgoing|both --depth N [--limit N] [--bytes N] [--continuation TOKEN]",
        positionals: &["owner"],
        options: &[
            "--direction",
            "--depth",
            "--limit",
            "--bytes",
            "--continuation",
        ],
    },
];

pub(crate) const QUERY_RESPONSE_FIELDS: [(&str, &str); 43] = [
    ("result", "status"),
    ("result", "command"),
    ("project", "path"),
    ("project", "name"),
    ("project", "repository"),
    ("project", "package"),
    ("revision", "observed"),
    ("query", "operation"),
    ("query", "digest"),
    ("owner", "id"),
    ("owner", "kind"),
    ("owner", "named"),
    ("owner", "name"),
    ("owner", "class"),
    ("owner", "parent"),
    ("owner", "depth"),
    ("relation", "kind"),
    ("relation", "source-package"),
    ("relation", "source-owner"),
    ("relation", "target-package"),
    ("relation", "target-owner"),
    ("summary", "returned"),
    ("summary", "visited"),
    ("summary", "match"),
    ("summary", "truncated"),
    ("summary", "total-owners"),
    ("summary", "total-relations"),
    ("summary", "expanded-owners"),
    ("continuation", "token"),
    ("work", "map-pages-read"),
    ("work", "map-bytes-read"),
    ("work", "map-entries-visited"),
    ("work", "catalog-lookups"),
    ("work", "store-objects-read"),
    ("work", "store-bytes-read"),
    ("work", "canonical-records-decoded"),
    ("work", "witness-records-decoded"),
    ("work", "relation-witnesses-visited"),
    ("work", "expanded-owners"),
    ("work", "selected-owners"),
    ("work", "selected-relations"),
    ("work", "rendered-output-bytes"),
    ("schema", "capabilities"),
];

pub(crate) const QUERY_SELECTOR_FIELDS: [(&str, &str); 11] = [
    ("selector", "operation"),
    ("selector", "owner-kind"),
    ("selector", "namespace-class"),
    ("selector", "namespace-name"),
    ("selector", "parent"),
    ("selector", "endpoint"),
    ("selector", "direction"),
    ("selector", "relation-kind"),
    ("selector", "root"),
    ("selector", "depth"),
    ("selector", "ordering"),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum QueryOperation {
    Owners,
    Find,
    Relations,
    Context,
}

impl QueryOperation {
    pub const fn tag(self) -> u8 {
        match self {
            Self::Owners => 1,
            Self::Find => 2,
            Self::Relations => 3,
            Self::Context => 4,
        }
    }

    pub const fn action(self) -> &'static str {
        match self {
            Self::Owners => "owners",
            Self::Find => "find",
            Self::Relations => "relations",
            Self::Context => "context",
        }
    }

    pub const fn command(self) -> &'static str {
        match self {
            Self::Owners => "query.owners",
            Self::Find => "query.find",
            Self::Relations => "query.relations",
            Self::Context => "query.context",
        }
    }

    fn from_tag(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::Owners),
            2 => Some(Self::Find),
            3 => Some(Self::Relations),
            4 => Some(Self::Context),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ContextDirection {
    Incoming,
    Outgoing,
    Both,
}

impl ContextDirection {
    pub const ALL: [Self; 3] = [Self::Incoming, Self::Outgoing, Self::Both];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Incoming => "incoming",
            Self::Outgoing => "outgoing",
            Self::Both => "both",
        }
    }

    const fn tag(self) -> u8 {
        match self {
            Self::Incoming => 1,
            Self::Outgoing => 2,
            Self::Both => 3,
        }
    }

    fn parse(value: &str) -> Result<Self, Diagnostic> {
        Self::ALL
            .into_iter()
            .find(|direction| direction.name() == value)
            .ok_or_else(|| {
                query_input_error(
                    "query_invalid_context_direction",
                    format!(
                        "context direction '{value}' is invalid; expected incoming, outgoing, or both"
                    ),
                )
            })
    }

    const fn includes_incoming(self) -> bool {
        matches!(self, Self::Incoming | Self::Both)
    }

    const fn includes_outgoing(self) -> bool {
        matches!(self, Self::Outgoing | Self::Both)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum QueryDirection {
    Incoming,
    Outgoing,
}

impl QueryDirection {
    pub const ALL: [Self; 2] = [Self::Incoming, Self::Outgoing];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Incoming => "incoming",
            Self::Outgoing => "outgoing",
        }
    }

    const fn tag(self) -> u8 {
        match self {
            Self::Incoming => 1,
            Self::Outgoing => 2,
        }
    }

    fn parse(value: &str) -> Result<Self, Diagnostic> {
        Self::ALL
            .into_iter()
            .find(|direction| direction.name() == value)
            .ok_or_else(|| {
                query_input_error(
                    "query_invalid_direction",
                    format!(
                        "relation direction '{value}' is invalid; expected incoming or outgoing"
                    ),
                )
            })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum QueryEndpointSelector {
    Package,
    Owner(OwnerKey),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum QuerySelection {
    Owners {
        kind: Option<OwnerKind>,
    },
    Find {
        class: NamespaceClass,
        name: Name,
        parent: Option<OwnerKey>,
    },
    Relations {
        endpoint: QueryEndpointSelector,
        direction: QueryDirection,
        kind: Option<RelationKind>,
    },
    Context {
        root: OwnerKey,
        direction: ContextDirection,
        depth: u8,
    },
}

impl QuerySelection {
    pub const fn operation(&self) -> QueryOperation {
        match self {
            Self::Owners { .. } => QueryOperation::Owners,
            Self::Find { .. } => QueryOperation::Find,
            Self::Relations { .. } => QueryOperation::Relations,
            Self::Context { .. } => QueryOperation::Context,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct QueryPageLimits {
    pub items: u64,
    pub output_bytes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NormalizedQueryRequest {
    pub selection: QuerySelection,
    pub limits: QueryPageLimits,
    pub continuation: Option<String>,
}

pub(crate) fn parse_query_arguments(
    arguments: &[String],
) -> Result<NormalizedQueryRequest, Diagnostic> {
    let action = arguments.first().ok_or_else(|| {
        query_input_error(
            "query_usage",
            "query requires exactly one action: owners, find, relations, or context",
        )
    })?;
    if matches!(
        action.as_str(),
        "callers" | "callees" | "types" | "capabilities" | "impact" | "request"
    ) {
        return Err(Diagnostic::new(
            DiagnosticClass::Source,
            "predecessor_contract",
            format!(
                "query action '{action}' belongs to the removed predecessor query grammar; use owners, find, relations, or context"
            ),
        ));
    }
    let descriptor = QUERY_OPERATION_DESCRIPTORS
        .iter()
        .find(|descriptor| descriptor.action == action)
        .ok_or_else(|| {
            query_input_error(
                "query_unknown_action",
                format!(
                    "unknown query action '{action}'; expected owners, find, relations, or context"
                ),
            )
        })?;
    match descriptor.action {
        "owners" => parse_owners_arguments(&arguments[1..], descriptor),
        "find" => parse_find_arguments(&arguments[1..], descriptor),
        "relations" => parse_relations_arguments(&arguments[1..], descriptor),
        "context" => parse_context_arguments(&arguments[1..], descriptor),
        _ => Err(Diagnostic::new(
            DiagnosticClass::Infrastructure,
            "query_descriptor_action",
            "query descriptor contains an unimplemented action",
        )),
    }
}

fn parse_owners_arguments(
    arguments: &[String],
    descriptor: &QueryOperationDescriptor,
) -> Result<NormalizedQueryRequest, Diagnostic> {
    let options = parse_query_options(arguments, descriptor)?;
    let kind = options
        .get("--kind")
        .map(|value| {
            OwnerKind::parse(value).map_err(|error| {
                query_input_error(
                    "query_invalid_owner_kind",
                    format!("--kind is invalid: {}", error.message),
                )
            })
        })
        .transpose()?;
    let limits = parse_page_limits(&options)?;
    let continuation = parse_continuation_option(&options)?;
    Ok(NormalizedQueryRequest {
        selection: QuerySelection::Owners { kind },
        limits,
        continuation,
    })
}

fn parse_find_arguments(
    arguments: &[String],
    descriptor: &QueryOperationDescriptor,
) -> Result<NormalizedQueryRequest, Diagnostic> {
    let (positionals, option_arguments) = split_positionals(arguments, 2, descriptor)?;
    let options = parse_query_options(option_arguments, descriptor)?;
    let class = NamespaceClass::parse(positionals[0]).map_err(|error| {
        query_input_error(
            "query_invalid_namespace_class",
            format!("namespace class is invalid: {}", error.message),
        )
    })?;
    let name = Name::new(positionals[1].to_owned()).map_err(|error| {
        query_input_error(
            "query_invalid_name",
            format!("namespace name is invalid: {}", error.message),
        )
    })?;
    let parent = options
        .get("--parent")
        .map(|value| parse_owner_identity(value, "--parent"))
        .transpose()?;
    validate_parent_rule(class, parent)?;
    Ok(NormalizedQueryRequest {
        selection: QuerySelection::Find {
            class,
            name,
            parent,
        },
        limits: QueryPageLimits {
            items: 1,
            output_bytes: DEFAULT_QUERY_OUTPUT_BYTES,
        },
        continuation: None,
    })
}

fn parse_relations_arguments(
    arguments: &[String],
    descriptor: &QueryOperationDescriptor,
) -> Result<NormalizedQueryRequest, Diagnostic> {
    let (positionals, option_arguments) = split_positionals(arguments, 1, descriptor)?;
    let options = parse_query_options(option_arguments, descriptor)?;
    let endpoint = if positionals[0] == "package" {
        QueryEndpointSelector::Package
    } else {
        QueryEndpointSelector::Owner(parse_owner_identity(positionals[0], "relation endpoint")?)
    };
    let direction = options
        .get("--direction")
        .ok_or_else(|| {
            query_input_error(
                "query_missing_direction",
                "query relations requires --direction incoming|outgoing",
            )
        })
        .and_then(|value| QueryDirection::parse(value))?;
    let kind = options
        .get("--kind")
        .map(|value| {
            RelationKind::parse(value).map_err(|error| {
                query_input_error(
                    "query_invalid_relation_kind",
                    format!("--kind is invalid: {}", error.message),
                )
            })
        })
        .transpose()?;
    let limits = parse_page_limits(&options)?;
    let continuation = parse_continuation_option(&options)?;
    Ok(NormalizedQueryRequest {
        selection: QuerySelection::Relations {
            endpoint,
            direction,
            kind,
        },
        limits,
        continuation,
    })
}

fn parse_context_arguments(
    arguments: &[String],
    descriptor: &QueryOperationDescriptor,
) -> Result<NormalizedQueryRequest, Diagnostic> {
    let (positionals, option_arguments) = split_positionals(arguments, 1, descriptor)?;
    let options = parse_query_options(option_arguments, descriptor)?;
    let root = parse_owner_identity(positionals[0], "context root")?;
    let direction = options
        .get("--direction")
        .ok_or_else(|| {
            query_input_error(
                "query_missing_context_direction",
                "query context requires --direction incoming|outgoing|both",
            )
        })
        .and_then(|value| ContextDirection::parse(value))?;
    let depth = options
        .get("--depth")
        .ok_or_else(|| {
            query_input_error(
                "query_missing_context_depth",
                format!(
                    "query context requires --depth {MINIMUM_CONTEXT_DEPTH} through {MAXIMUM_CONTEXT_DEPTH}"
                ),
            )
        })
        .and_then(|value| parse_context_depth(value))?;
    let limits = parse_page_limits(&options)?;
    let continuation = parse_continuation_option(&options)?;
    Ok(NormalizedQueryRequest {
        selection: QuerySelection::Context {
            root,
            direction,
            depth,
        },
        limits,
        continuation,
    })
}

fn parse_context_depth(value: &str) -> Result<u8, Diagnostic> {
    let parsed = value.parse::<u8>().map_err(|_| {
        query_input_error(
            "query_invalid_context_depth",
            format!("context depth '{value}' is not a canonical positive integer"),
        )
    })?;
    if !(MINIMUM_CONTEXT_DEPTH..=MAXIMUM_CONTEXT_DEPTH).contains(&parsed)
        || parsed.to_string() != value
    {
        return Err(query_input_error(
            "query_invalid_context_depth",
            format!(
                "context depth must be {MINIMUM_CONTEXT_DEPTH} through {MAXIMUM_CONTEXT_DEPTH}"
            ),
        ));
    }
    Ok(parsed)
}

fn split_positionals<'a>(
    arguments: &'a [String],
    count: usize,
    descriptor: &QueryOperationDescriptor,
) -> Result<(Vec<&'a str>, &'a [String]), Diagnostic> {
    if arguments.len() < count
        || arguments[..count]
            .iter()
            .any(|value| value.starts_with("--"))
    {
        return Err(query_input_error(
            "query_usage",
            format!("invalid query grammar; expected: {}", descriptor.usage),
        ));
    }
    let positionals = arguments[..count]
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    Ok((positionals, &arguments[count..]))
}

fn parse_query_options(
    arguments: &[String],
    descriptor: &QueryOperationDescriptor,
) -> Result<BTreeMap<String, String>, Diagnostic> {
    let mut options = BTreeMap::new();
    let mut index = 0;
    while index < arguments.len() {
        let name = &arguments[index];
        if !name.starts_with("--") {
            return Err(query_input_error(
                "query_unexpected_argument",
                format!(
                    "unexpected query argument '{name}'; expected: {}",
                    descriptor.usage
                ),
            ));
        }
        if !descriptor.options.contains(&name.as_str()) {
            return Err(query_input_error(
                "query_unknown_option",
                format!("unknown option '{name}' for query {}", descriptor.action),
            ));
        }
        let value = arguments.get(index + 1).ok_or_else(|| {
            query_input_error(
                "query_option_value",
                format!("query option '{name}' requires one value"),
            )
        })?;
        if value.starts_with("--") {
            return Err(query_input_error(
                "query_option_value",
                format!("query option '{name}' requires one value"),
            ));
        }
        if options.insert(name.clone(), value.clone()).is_some() {
            return Err(query_input_error(
                "query_duplicate_option",
                format!("query option '{name}' may be supplied only once"),
            ));
        }
        index += 2;
    }
    Ok(options)
}

fn parse_page_limits(options: &BTreeMap<String, String>) -> Result<QueryPageLimits, Diagnostic> {
    let items = options
        .get("--limit")
        .map(|value| parse_bounded_u64(value, "item", MAXIMUM_QUERY_ITEMS))
        .transpose()?
        .unwrap_or(DEFAULT_QUERY_ITEMS);
    let output_bytes = options
        .get("--bytes")
        .map(|value| {
            parse_bounded_usize(
                value,
                "output-byte",
                MINIMUM_QUERY_OUTPUT_BYTES,
                MAXIMUM_QUERY_OUTPUT_BYTES,
            )
        })
        .transpose()?
        .unwrap_or(DEFAULT_QUERY_OUTPUT_BYTES);
    Ok(QueryPageLimits {
        items,
        output_bytes,
    })
}

fn parse_bounded_u64(value: &str, dimension: &str, maximum: u64) -> Result<u64, Diagnostic> {
    let parsed = value.parse::<u64>().map_err(|_| {
        query_input_error(
            "query_invalid_limit",
            format!("query {dimension} limit '{value}' is not a canonical positive integer"),
        )
    })?;
    if parsed == 0 || parsed > maximum || parsed.to_string() != value {
        return Err(query_input_error(
            "query_invalid_limit",
            format!("query {dimension} limit must be 1 through {maximum}"),
        ));
    }
    Ok(parsed)
}

fn parse_bounded_usize(
    value: &str,
    dimension: &str,
    minimum: usize,
    maximum: usize,
) -> Result<usize, Diagnostic> {
    let parsed = value.parse::<usize>().map_err(|_| {
        query_input_error(
            "query_invalid_byte_limit",
            format!("query {dimension} limit '{value}' is not a canonical positive integer"),
        )
    })?;
    if parsed < minimum || parsed > maximum || parsed.to_string() != value {
        return Err(query_input_error(
            "query_invalid_byte_limit",
            format!("query {dimension} limit must be {minimum} through {maximum}"),
        ));
    }
    Ok(parsed)
}

fn parse_continuation_option(
    options: &BTreeMap<String, String>,
) -> Result<Option<String>, Diagnostic> {
    let continuation = options.get("--continuation").cloned();
    if continuation
        .as_ref()
        .is_some_and(|token| token.len() > MAXIMUM_QUERY_CONTINUATION_BYTES)
    {
        return Err(query_input_error(
            "query_continuation_oversized",
            format!("query continuation exceeds {MAXIMUM_QUERY_CONTINUATION_BYTES} encoded bytes"),
        ));
    }
    Ok(continuation)
}

fn parse_owner_identity(value: &str, boundary: &str) -> Result<OwnerKey, Diagnostic> {
    value.parse().map_err(|error: Diagnostic| {
        query_input_error(
            "query_invalid_owner_identity",
            format!("{boundary} is invalid: {}", error.message),
        )
    })
}

fn validate_parent_rule(class: NamespaceClass, parent: Option<OwnerKey>) -> Result<(), Diagnostic> {
    let is_root = matches!(class, NamespaceClass::Module | NamespaceClass::Target);
    if is_root && parent.is_some() {
        return Err(query_input_error(
            "query_parent_forbidden",
            format!(
                "namespace class '{}' is package-root and forbids --parent",
                class.name()
            ),
        ));
    }
    if !is_root && parent.is_none() {
        return Err(query_input_error(
            "query_parent_required",
            format!(
                "namespace class '{}' requires one exact --parent OWNER",
                class.name()
            ),
        ));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct QueryDigest([u8; 32]);

impl QueryDigest {
    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Display for QueryDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("query_")?;
        formatter.write_str(&encode_hex(&self.0))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DecodedContinuation {
    repository: RepositoryId,
    package: PackageId,
    revision: RevisionId,
    operation: QueryOperation,
    selector: QueryDigest,
    resume_key: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct QueryBinding {
    repository: RepositoryId,
    package: PackageId,
    revision: RevisionId,
}

fn query_digest(selection: &QuerySelection) -> Result<QueryDigest, Diagnostic> {
    let mut bytes = Vec::new();
    bytes.push(QUERY_ORDERING_CONTRACT);
    bytes.push(selection.operation().tag());
    match selection {
        QuerySelection::Owners { kind } => push_optional_tag(&mut bytes, kind.map(OwnerKind::tag)),
        QuerySelection::Find {
            class,
            name,
            parent,
        } => {
            bytes.push(class.tag());
            push_length_prefixed(&mut bytes, name.as_str().as_bytes())?;
            match parent {
                None => bytes.push(0),
                Some(parent) => {
                    bytes.push(1);
                    bytes.extend_from_slice(&EncodedOwnerKey::new(*parent).bytes());
                }
            }
        }
        QuerySelection::Relations {
            endpoint,
            direction,
            kind,
        } => {
            match endpoint {
                QueryEndpointSelector::Package => bytes.push(1),
                QueryEndpointSelector::Owner(owner) => {
                    bytes.push(2);
                    bytes.extend_from_slice(&EncodedOwnerKey::new(*owner).bytes());
                }
            }
            bytes.push(direction.tag());
            push_optional_tag(&mut bytes, kind.map(RelationKind::tag));
        }
        QuerySelection::Context {
            root,
            direction,
            depth,
        } => {
            bytes.extend_from_slice(&EncodedOwnerKey::new(*root).bytes());
            bytes.push(direction.tag());
            bytes.push(*depth);
        }
    }
    Ok(QueryDigest(domain_digest(
        QUERY_SELECTOR_DIGEST_DOMAIN,
        &bytes,
    )))
}

fn push_optional_tag(bytes: &mut Vec<u8>, tag: Option<u8>) {
    match tag {
        None => bytes.push(0),
        Some(tag) => {
            bytes.push(1);
            bytes.push(tag);
        }
    }
}

fn push_length_prefixed(bytes: &mut Vec<u8>, value: &[u8]) -> Result<(), Diagnostic> {
    let length = u16::try_from(value.len()).map_err(|_| {
        Diagnostic::new(
            DiagnosticClass::Resource,
            "query_selector_length",
            "query selector field exceeds its canonical length encoding",
        )
    })?;
    bytes.extend_from_slice(&length.to_be_bytes());
    bytes.extend_from_slice(value);
    Ok(())
}

fn encode_continuation(
    binding: QueryBinding,
    operation: QueryOperation,
    selector: QueryDigest,
    resume_key: &[u8],
) -> Result<String, Diagnostic> {
    if resume_key.is_empty() || resume_key.len() > MAXIMUM_QUERY_RESUME_KEY_BYTES {
        return Err(Diagnostic::new(
            DiagnosticClass::Infrastructure,
            "query_continuation_resume_key",
            "query resume key is outside its canonical operation bound",
        ));
    }
    let mut payload = Vec::with_capacity(171);
    payload.extend_from_slice(&QUERY_CONTINUATION_VERSION.to_be_bytes());
    payload.extend_from_slice(&QUERY_CONTRACT_VERSION.to_be_bytes());
    payload.extend_from_slice(&binding.repository.bytes());
    payload.extend_from_slice(&binding.package.bytes());
    payload.extend_from_slice(&binding.revision.bytes());
    payload.push(operation.tag());
    payload.extend_from_slice(&selector.bytes());
    push_length_prefixed(&mut payload, resume_key)?;
    let payload_length = u64::try_from(payload.len()).map_err(|_| {
        Diagnostic::new(
            DiagnosticClass::Resource,
            "query_continuation_length",
            "query continuation payload length cannot be represented",
        )
    })?;
    let mut bytes =
        Vec::with_capacity(CONTINUATION_HEADER_BYTES + payload.len() + CONTINUATION_CHECKSUM_BYTES);
    bytes.extend_from_slice(&QUERY_CONTINUATION_MAGIC);
    bytes.extend_from_slice(&QUERY_CONTINUATION_ENVELOPE_VERSION.to_le_bytes());
    bytes.extend_from_slice(&payload_length.to_le_bytes());
    bytes.extend_from_slice(&payload);
    bytes.extend_from_slice(&domain_digest(QUERY_CONTINUATION_INTEGRITY_DOMAIN, &bytes));
    let token = format!(
        "{QUERY_CONTINUATION_PREFIX}{}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
    );
    if token.len() > MAXIMUM_QUERY_CONTINUATION_BYTES {
        return Err(Diagnostic::new(
            DiagnosticClass::Infrastructure,
            "query_continuation_length",
            "canonical query continuation exceeds its declared textual bound",
        ));
    }
    Ok(token)
}

fn decode_continuation(token: &str) -> Result<DecodedContinuation, Diagnostic> {
    if token.starts_with("cont_") {
        return Err(Diagnostic::new(
            DiagnosticClass::Source,
            "predecessor_contract",
            "the supplied continuation belongs to the removed predecessor query contract",
        ));
    }
    if token.len() > MAXIMUM_QUERY_CONTINUATION_BYTES {
        return Err(query_input_error(
            "query_continuation_oversized",
            format!("query continuation exceeds {MAXIMUM_QUERY_CONTINUATION_BYTES} encoded bytes"),
        ));
    }
    let encoded = token
        .strip_prefix(QUERY_CONTINUATION_PREFIX)
        .ok_or_else(|| {
            query_input_error(
                "query_continuation_malformed",
                "query continuation has an unknown textual domain",
            )
        })?;
    if encoded.is_empty() || encoded.contains('=') {
        return Err(query_input_error(
            "query_continuation_noncanonical",
            "query continuation must use canonical unpadded URL-safe base64",
        ));
    }
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|_| {
            query_input_error(
                "query_continuation_malformed",
                "query continuation contains malformed URL-safe base64",
            )
        })?;
    if bytes.len() > MAXIMUM_QUERY_CONTINUATION_DECODED_BYTES {
        return Err(query_input_error(
            "query_continuation_oversized",
            "decoded query continuation exceeds its strict canonical bound",
        ));
    }
    if base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&bytes) != encoded {
        return Err(query_input_error(
            "query_continuation_noncanonical",
            "query continuation base64 does not reproduce its canonical text",
        ));
    }
    if bytes.len() < CONTINUATION_HEADER_BYTES + CONTINUATION_CHECKSUM_BYTES {
        return Err(query_input_error(
            "query_continuation_malformed",
            "query continuation is truncated",
        ));
    }
    if bytes[..8] != QUERY_CONTINUATION_MAGIC {
        return Err(query_input_error(
            "query_continuation_contract",
            "query continuation uses an unknown contract identity",
        ));
    }
    let envelope_version = u16::from_le_bytes([bytes[8], bytes[9]]);
    if envelope_version != QUERY_CONTINUATION_ENVELOPE_VERSION {
        return Err(query_input_error(
            "query_continuation_contract",
            "query continuation uses a foreign envelope version",
        ));
    }
    let payload_length = usize::try_from(u64::from_le_bytes(bytes[10..18].try_into().map_err(
        |_| {
            query_input_error(
                "query_continuation_malformed",
                "query continuation has a malformed length field",
            )
        },
    )?))
    .map_err(|_| {
        query_input_error(
            "query_continuation_oversized",
            "query continuation payload length cannot be represented",
        )
    })?;
    let expected_length = CONTINUATION_HEADER_BYTES
        .checked_add(payload_length)
        .and_then(|length| length.checked_add(CONTINUATION_CHECKSUM_BYTES))
        .ok_or_else(|| {
            query_input_error(
                "query_continuation_oversized",
                "query continuation length overflowed",
            )
        })?;
    if expected_length != bytes.len() {
        return Err(query_input_error(
            "query_continuation_malformed",
            "query continuation length does not match its canonical envelope",
        ));
    }
    let checksum_start = CONTINUATION_HEADER_BYTES + payload_length;
    if bytes[checksum_start..]
        != domain_digest(
            QUERY_CONTINUATION_INTEGRITY_DOMAIN,
            &bytes[..checksum_start],
        )
    {
        return Err(query_input_error(
            "query_continuation_integrity",
            "query continuation integrity digest does not match its canonical payload",
        ));
    }
    let payload = &bytes[CONTINUATION_HEADER_BYTES..checksum_start];
    let mut decoder = ContinuationDecoder::new(payload);
    if decoder.u16()? != QUERY_CONTINUATION_VERSION || decoder.u16()? != QUERY_CONTRACT_VERSION {
        return Err(query_input_error(
            "query_continuation_contract",
            "query continuation uses a foreign continuation or query contract version",
        ));
    }
    let repository = RepositoryId::from_bytes(decoder.array_16()?).ok_or_else(|| {
        query_input_error(
            "query_continuation_reserved_identity",
            "query continuation contains the reserved repository identity",
        )
    })?;
    let package = PackageId::from_bytes(decoder.array_16()?).ok_or_else(|| {
        query_input_error(
            "query_continuation_reserved_identity",
            "query continuation contains the reserved package identity",
        )
    })?;
    let revision_bytes = decoder.array_32()?;
    if revision_bytes == [0; 32] {
        return Err(query_input_error(
            "query_continuation_reserved_identity",
            "query continuation contains the reserved revision identity",
        ));
    }
    let revision = RevisionId::from_digest(revision_bytes);
    let operation = QueryOperation::from_tag(decoder.u8()?).ok_or_else(|| {
        query_input_error(
            "query_continuation_operation",
            "query continuation contains an unknown operation tag",
        )
    })?;
    let selector = QueryDigest(decoder.array_32()?);
    let resume_length = usize::from(decoder.u16()?);
    if resume_length == 0 || resume_length > MAXIMUM_QUERY_RESUME_KEY_BYTES {
        return Err(query_input_error(
            "query_continuation_resume_key",
            "query continuation resume key is outside its canonical operation bound",
        ));
    }
    let resume_key = decoder.take(resume_length)?.to_vec();
    decoder.finish()?;
    Ok(DecodedContinuation {
        repository,
        package,
        revision,
        operation,
        selector,
        resume_key,
    })
}

fn bind_continuation(
    request: &NormalizedQueryRequest,
    binding: QueryBinding,
    selector: QueryDigest,
) -> Result<Option<Vec<u8>>, Diagnostic> {
    let Some(token) = request.continuation.as_deref() else {
        return Ok(None);
    };
    let continuation = decode_continuation(token)?;
    if continuation.repository != binding.repository || continuation.package != binding.package {
        return Err(query_input_error(
            "query_continuation_foreign",
            "query continuation belongs to a foreign repository or package",
        ));
    }
    if continuation.revision != binding.revision {
        return Err(query_input_error(
            "query_continuation_stale",
            format!(
                "query continuation observes revision '{}', but current HEAD is '{}'",
                continuation.revision, binding.revision
            ),
        ));
    }
    if continuation.operation != request.selection.operation() || continuation.selector != selector
    {
        return Err(query_input_error(
            "query_continuation_mismatch",
            "query continuation does not match the normalized semantic selector",
        ));
    }
    validate_resume_key(
        &request.selection,
        binding.package,
        &continuation.resume_key,
    )?;
    Ok(Some(continuation.resume_key))
}

fn validate_resume_key(
    selection: &QuerySelection,
    package: PackageId,
    key: &[u8],
) -> Result<(), Diagnostic> {
    match selection {
        QuerySelection::Owners { .. } => {
            EncodedOwnerKey::decode(key).map_err(|_| {
                query_input_error(
                    "query_continuation_resume_key",
                    "owner continuation contains an invalid logical owner key",
                )
            })?;
        }
        QuerySelection::Relations {
            endpoint,
            direction,
            kind,
        } => {
            let edge = match direction {
                QueryDirection::Incoming => decode_reverse_relation_key(key),
                QueryDirection::Outgoing => decode_forward_relation_key(key),
            }
            .map_err(|_| {
                query_input_error(
                    "query_continuation_resume_key",
                    "relation continuation contains an invalid logical relation key",
                )
            })?;
            let expected_endpoint = match endpoint {
                QueryEndpointSelector::Package => RelationEndpoint::Package(package),
                QueryEndpointSelector::Owner(owner) => RelationEndpoint::Owner(ExactOwnerKey {
                    package,
                    owner: *owner,
                }),
            };
            let observed_endpoint = match direction {
                QueryDirection::Incoming => edge.target,
                QueryDirection::Outgoing => edge.source,
            };
            let prefix = match direction {
                QueryDirection::Incoming => reverse_relation_prefix(expected_endpoint, *kind),
                QueryDirection::Outgoing => forward_relation_prefix(expected_endpoint, *kind),
            };
            if observed_endpoint != expected_endpoint
                || kind.is_some_and(|expected| edge.kind != expected)
                || !key.starts_with(&prefix)
            {
                return Err(query_input_error(
                    "query_continuation_resume_key",
                    "relation continuation resume key disagrees with its selector prefix",
                ));
            }
        }
        QuerySelection::Context { depth, .. } => {
            validate_context_resume_key(key, *depth)?;
        }
        QuerySelection::Find { .. } => {
            return Err(query_input_error(
                "query_continuation_mismatch",
                "exact namespace lookup does not accept continuations",
            ));
        }
    }
    Ok(())
}

const CONTEXT_OWNER_SECTION: u8 = 1;
const CONTEXT_RELATION_SECTION: u8 = 2;

fn context_owner_resume_key(depth: u8, owner: OwnerKey) -> Vec<u8> {
    let mut key = Vec::with_capacity(19);
    key.push(CONTEXT_OWNER_SECTION);
    key.push(depth);
    key.extend_from_slice(&EncodedOwnerKey::new(owner).bytes());
    key
}

fn context_relation_resume_key(edge: RelationEdge) -> Vec<u8> {
    let relation = forward_relation_key(edge);
    let mut key = Vec::with_capacity(relation.len().saturating_add(1));
    key.push(CONTEXT_RELATION_SECTION);
    key.extend_from_slice(&relation);
    key
}

fn validate_context_resume_key(key: &[u8], maximum_depth: u8) -> Result<(), Diagnostic> {
    let Some(section) = key.first().copied() else {
        return Err(query_input_error(
            "query_continuation_resume_key",
            "context continuation has an empty resume key",
        ));
    };
    match section {
        CONTEXT_OWNER_SECTION => {
            if key.len() != 19 {
                return Err(query_input_error(
                    "query_continuation_resume_key",
                    "context owner continuation has an invalid logical key length",
                ));
            }
            let depth = key[1];
            if depth > maximum_depth {
                return Err(query_input_error(
                    "query_continuation_resume_key",
                    "context owner continuation depth exceeds its selector",
                ));
            }
            EncodedOwnerKey::decode(&key[2..]).map_err(|_| {
                query_input_error(
                    "query_continuation_resume_key",
                    "context owner continuation contains an invalid owner key",
                )
            })?;
        }
        CONTEXT_RELATION_SECTION => {
            decode_forward_relation_key(&key[1..]).map_err(|_| {
                query_input_error(
                    "query_continuation_resume_key",
                    "context relation continuation contains an invalid canonical edge key",
                )
            })?;
        }
        _ => {
            return Err(query_input_error(
                "query_continuation_resume_key",
                "context continuation contains an unknown output section",
            ));
        }
    }
    Ok(())
}

struct ContinuationDecoder<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> ContinuationDecoder<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], Diagnostic> {
        let end = self.position.checked_add(length).ok_or_else(|| {
            query_input_error(
                "query_continuation_malformed",
                "query continuation decoder position overflowed",
            )
        })?;
        let value = self.bytes.get(self.position..end).ok_or_else(|| {
            query_input_error(
                "query_continuation_malformed",
                "query continuation payload is truncated",
            )
        })?;
        self.position = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, Diagnostic> {
        self.take(1).map(|bytes| bytes[0])
    }

    fn u16(&mut self) -> Result<u16, Diagnostic> {
        self.take(2)
            .map(|bytes| u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn array_16(&mut self) -> Result<[u8; 16], Diagnostic> {
        self.take(16)?.try_into().map_err(|_| {
            query_input_error(
                "query_continuation_malformed",
                "query continuation identity has the wrong width",
            )
        })
    }

    fn array_32(&mut self) -> Result<[u8; 32], Diagnostic> {
        self.take(32)?.try_into().map_err(|_| {
            query_input_error(
                "query_continuation_malformed",
                "query continuation digest has the wrong width",
            )
        })
    }

    fn finish(self) -> Result<(), Diagnostic> {
        if self.position != self.bytes.len() {
            return Err(query_input_error(
                "query_continuation_trailing",
                "query continuation payload contains trailing bytes",
            ));
        }
        Ok(())
    }
}

fn domain_digest(domain: &str, bytes: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key(domain);
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
    *hasher.finalize().as_bytes()
}

fn query_input_error(code: &'static str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Source, code, message)
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct OwnerProjection {
    owner: OwnerKey,
    kind: OwnerKind,
    namespace: Option<OwnerNamespaceProjection>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct OwnerNamespaceProjection {
    name: String,
    class: NamespaceClass,
    parent: Option<OwnerKey>,
}

impl OwnerProjection {
    fn from_record(owner: OwnerKey, record: &super::kernel::OwnerRecord) -> Self {
        let namespace = owner_namespace(record).map(|namespace| OwnerNamespaceProjection {
            name: namespace.name.as_str().to_owned(),
            class: namespace.class,
            parent: namespace.parent,
        });
        Self {
            owner,
            kind: record.kind(),
            namespace,
        }
    }

    fn fields(&self) -> Vec<(&'static str, String)> {
        let mut fields = vec![
            ("id", self.owner.to_string()),
            ("kind", self.kind.name().to_owned()),
            (
                "named",
                if self.namespace.is_some() {
                    "true"
                } else {
                    "false"
                }
                .to_owned(),
            ),
        ];
        if let Some(namespace) = &self.namespace {
            fields.push(("name", namespace.name.clone()));
            fields.push(("class", namespace.class.name().to_owned()));
            fields.push((
                "parent",
                namespace
                    .parent
                    .map_or_else(|| "package".to_owned(), |parent| parent.to_string()),
            ));
        }
        fields
    }

    fn fields_with_depth(&self, depth: u8) -> Vec<(&'static str, String)> {
        let mut fields = self.fields();
        fields.push(("depth", depth.to_string()));
        fields
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ContextOwnerProjection {
    depth: u8,
    projection: OwnerProjection,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct ContextSummary {
    total_owners: u64,
    total_relations: u64,
    expanded_owners: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct ContextWork {
    relation_witnesses_visited: u64,
    expanded_owners: u64,
    selected_owners: u64,
    selected_relations: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum QueryItems {
    Owners(Vec<OwnerProjection>),
    Relations(Vec<RelationEdge>),
    Context {
        owners: Vec<ContextOwnerProjection>,
        relations: Vec<RelationEdge>,
    },
}

#[derive(Clone, Debug)]
struct QueryExecution {
    operation: QueryOperation,
    selector: QueryDigest,
    items: QueryItems,
    returned: u64,
    visited: u64,
    matched: Option<bool>,
    truncated: Option<bool>,
    context_summary: Option<ContextSummary>,
    context_work: Option<ContextWork>,
    continuation: Option<String>,
    work: RepositoryReadWork,
}

#[derive(Clone, Debug)]
struct QueryRenderContext<'a> {
    repository_root: &'a Path,
    project_name: &'a str,
    repository: RepositoryId,
    package: PackageId,
    revision: RevisionId,
    capabilities_digest: &'a str,
}

pub(crate) fn execute_normalized_query(
    repository: &GraphRepository,
    view: &RepositoryView,
    request: &NormalizedQueryRequest,
) -> Result<Vec<u8>, Diagnostic> {
    let control = ExecutionControl::uncancelled();
    let mut cancellation = || check_query_control(&control);
    execute_normalized_query_with_cancellation(repository, view, request, &mut cancellation)
}

fn execute_normalized_query_with_cancellation(
    repository: &GraphRepository,
    view: &RepositoryView,
    request: &NormalizedQueryRequest,
    cancellation: &mut dyn FnMut() -> Result<(), Diagnostic>,
) -> Result<Vec<u8>, Diagnostic> {
    cancellation()?;
    let snapshot = capabilities_snapshot().map_err(|message| {
        Diagnostic::new(
            DiagnosticClass::Infrastructure,
            "capabilities_projection",
            message,
        )
    })?;
    let binding = QueryBinding {
        repository: view.current().head.repository_id,
        package: view.package(),
        revision: view.revision(),
    };
    let selector = query_digest(&request.selection)?;
    let resume_key = bind_continuation(request, binding, selector)?;
    let context = QueryRenderContext {
        repository_root: repository.root(),
        project_name: view.current().semantic_root.package_name.as_str(),
        repository: binding.repository,
        package: binding.package,
        revision: binding.revision,
        capabilities_digest: &snapshot.digest,
    };
    let (fixed_bytes, fixed_records) = if request.selection.operation() == QueryOperation::Context {
        (0, 0)
    } else {
        fixed_response_reserve(
            &context,
            request.selection.operation(),
            selector,
            request.limits.output_bytes,
        )
        .map_err(classify_query_output_diagnostic)?
    };
    let item_byte_budget = request
        .limits
        .output_bytes
        .checked_sub(fixed_bytes)
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticClass::Resource,
                "query_output_envelope_too_large",
                format!(
                    "query fixed response envelope requires at least {fixed_bytes} bytes; increase --bytes"
                ),
            )
        })?;
    let item_record_budget = MAXIMUM_CLI_RESPONSE_RECORDS
        .checked_sub(fixed_records)
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticClass::Infrastructure,
                "query_output_record_configuration",
                "query fixed response records exceed the compact response record bound",
            )
        })?;
    let execution = match &request.selection {
        QuerySelection::Owners { kind } => execute_owners(
            view,
            request,
            binding,
            selector,
            resume_key.as_deref(),
            *kind,
            item_byte_budget,
            item_record_budget,
        ),
        QuerySelection::Find {
            class,
            name,
            parent,
        } => execute_find(
            view,
            request,
            selector,
            *class,
            name,
            *parent,
            item_byte_budget,
        ),
        QuerySelection::Relations {
            endpoint,
            direction,
            kind,
        } => execute_relations(
            view,
            request,
            binding,
            selector,
            resume_key.as_deref(),
            *endpoint,
            *direction,
            *kind,
            item_byte_budget,
            item_record_budget,
        ),
        QuerySelection::Context {
            root,
            direction,
            depth,
        } => execute_context(
            view,
            context_query_admission(),
            request,
            &context,
            binding,
            selector,
            resume_key.as_deref(),
            *root,
            *direction,
            *depth,
            cancellation,
        ),
    }
    .map_err(classify_query_read_diagnostic)?;
    render_query_response(&context, &execution, request.limits.output_bytes)
}

fn check_query_control(control: &ExecutionControl) -> Result<(), Diagnostic> {
    control.check().map_err(|error| {
        Diagnostic::new(
            DiagnosticClass::Cancelled,
            "query_cancelled",
            format!("normalized context query: {}", error.message),
        )
    })
}

fn classify_query_read_diagnostic(mut diagnostic: Diagnostic) -> Diagnostic {
    let replacement = match diagnostic.code.as_str() {
        "persistent_map_admission_pages_read" => Some("query_admission_map_pages"),
        "persistent_map_admission_bytes_read" => Some("query_admission_map_bytes"),
        "persistent_map_admission_entries_visited" => Some("query_admission_map_entries"),
        "object_read_catalog_lookups_exhausted" => Some("query_admission_catalog_lookups"),
        "object_read_objects_exhausted" => Some("query_admission_store_objects"),
        "object_read_bytes_exhausted" => Some("query_admission_store_bytes"),
        "persistent_map_page_missing" => Some("query_required_map_page_missing"),
        "publication_read_object_missing" => Some("query_required_object_missing"),
        _ => None,
    };
    if let Some(code) = replacement {
        diagnostic.code = code.to_owned();
        diagnostic.message = format!("normalized query read boundary: {}", diagnostic.message);
    }
    diagnostic
}

fn classify_query_output_diagnostic(mut diagnostic: Diagnostic) -> Diagnostic {
    if diagnostic.code == "control_response_byte_budget" {
        diagnostic.code = "query_output_envelope_too_large".to_owned();
        diagnostic.message = format!("query fixed response envelope: {}", diagnostic.message);
    }
    diagnostic
}

#[allow(
    clippy::too_many_arguments,
    reason = "the query page boundary keeps selector, logical, byte, and record dimensions explicit"
)]
fn execute_owners(
    view: &RepositoryView,
    request: &NormalizedQueryRequest,
    binding: QueryBinding,
    selector: QueryDigest,
    resume_key: Option<&[u8]>,
    kind: Option<OwnerKind>,
    item_byte_budget: usize,
    item_record_budget: usize,
) -> Result<QueryExecution, Diagnostic> {
    let scan_quantum = owner_scan_quantum(request.limits.items)?;
    let logical_item_limit = request
        .limits
        .items
        .min(u64::try_from(item_record_budget).map_err(|_| {
            Diagnostic::new(
                DiagnosticClass::Resource,
                "query_record_limit",
                "compact response record capacity cannot be represented as a query item count",
            )
        })?);
    if logical_item_limit == 0 {
        return Err(Diagnostic::new(
            DiagnosticClass::Resource,
            "query_record_limit",
            "compact response has no capacity for a query result record",
        ));
    }
    let admission = query_admission(scan_quantum, logical_item_limit)?;
    let mut owners = Vec::new();
    let mut item_bytes = 0_usize;
    let read = view.visit_query_owners(
        resume_key,
        kind,
        scan_quantum,
        logical_item_limit,
        admission,
        |owner, record| {
            let projection = OwnerProjection::from_record(owner, record);
            let record_bytes = compact_record_bytes("owner", &projection.fields())?;
            let required = item_bytes.checked_add(record_bytes).ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticClass::Resource,
                    "query_output_byte_overflow",
                    "owner result byte accounting overflowed",
                )
            })?;
            if required > item_byte_budget {
                if owners.is_empty() {
                    return Err(Diagnostic::new(
                        DiagnosticClass::Resource,
                        "query_output_item_too_large",
                        "one canonical owner projection cannot fit the requested --bytes limit",
                    ));
                }
                return Ok(MapRangeControl::StopBefore);
            }
            item_bytes = required;
            owners.push(projection);
            Ok(MapRangeControl::Continue)
        },
    )?;
    if read.revision != binding.revision {
        return Err(query_corrupt(
            "query_revision_binding",
            "owner range did not retain its pinned accepted revision",
        ));
    }
    let continuation = continuation_for_range(
        read.value.has_more,
        read.value.last_visited_key.as_deref(),
        binding,
        QueryOperation::Owners,
        selector,
    )?;
    Ok(QueryExecution {
        operation: QueryOperation::Owners,
        selector,
        items: QueryItems::Owners(owners),
        returned: read.value.returned,
        visited: read.value.visited,
        matched: None,
        truncated: Some(read.value.has_more),
        context_summary: None,
        context_work: None,
        continuation,
        work: read.work,
    })
}

fn execute_find(
    view: &RepositoryView,
    request: &NormalizedQueryRequest,
    selector: QueryDigest,
    class: NamespaceClass,
    name: &Name,
    parent: Option<OwnerKey>,
    item_byte_budget: usize,
) -> Result<QueryExecution, Diagnostic> {
    if request.continuation.is_some() {
        return Err(query_input_error(
            "query_continuation_mismatch",
            "query find does not accept a continuation",
        ));
    }
    let total_admission = find_admission();
    let mut work = RepositoryReadWork::default();
    if let Some(parent) = parent {
        validate_parent_identity_class(class, parent)?;
        let parent_read = view.query_owner(parent, remaining_admission(total_admission, &work)?)?;
        add_query_work(&mut work, parent_read.work)?;
        let parent_record = parent_read.value.ok_or_else(|| {
            Diagnostic::new(
                DiagnosticClass::Semantic,
                "query_parent_not_found",
                format!(
                    "namespace parent '{parent}' is not live at revision '{}'",
                    view.revision()
                ),
            )
        })?;
        validate_live_parent(class, parent, &parent_record)?;
    }
    let namespace = NamespaceKey {
        parent,
        class,
        name: name.clone(),
    };
    let namespace_read =
        view.query_namespace(&namespace, remaining_admission(total_admission, &work)?)?;
    add_query_work(&mut work, namespace_read.work)?;
    let Some(owner) = namespace_read.value else {
        return Ok(QueryExecution {
            operation: QueryOperation::Find,
            selector,
            items: QueryItems::Owners(Vec::new()),
            returned: 0,
            visited: 0,
            matched: Some(false),
            truncated: None,
            context_summary: None,
            context_work: None,
            continuation: None,
            work,
        });
    };
    let owner_read = view.query_owner(owner, remaining_admission(total_admission, &work)?)?;
    add_query_work(&mut work, owner_read.work)?;
    let record = owner_read.value.ok_or_else(|| {
        query_corrupt(
            "query_namespace_owner_missing",
            "namespace witness references an owner absent from canonical authority",
        )
    })?;
    let projection = exact_namespace_projection(owner, &record, class, name, parent)?;
    if compact_record_bytes("owner", &projection.fields())? > item_byte_budget {
        return Err(Diagnostic::new(
            DiagnosticClass::Resource,
            "query_output_item_too_large",
            "the exact owner projection cannot fit the bounded query response",
        ));
    }
    Ok(QueryExecution {
        operation: QueryOperation::Find,
        selector,
        items: QueryItems::Owners(vec![projection]),
        returned: 1,
        visited: 1,
        matched: Some(true),
        truncated: None,
        context_summary: None,
        context_work: None,
        continuation: None,
        work,
    })
}

#[allow(
    clippy::too_many_arguments,
    reason = "the relation query boundary keeps selector, logical, byte, and record dimensions explicit"
)]
fn execute_relations(
    view: &RepositoryView,
    request: &NormalizedQueryRequest,
    binding: QueryBinding,
    selector: QueryDigest,
    resume_key: Option<&[u8]>,
    endpoint_selector: QueryEndpointSelector,
    direction: QueryDirection,
    kind: Option<RelationKind>,
    item_byte_budget: usize,
    item_record_budget: usize,
) -> Result<QueryExecution, Diagnostic> {
    let total_admission = query_admission(request.limits.items, request.limits.items)?;
    let mut work = RepositoryReadWork::default();
    let endpoint = match endpoint_selector {
        QueryEndpointSelector::Package => RelationEndpoint::Package(binding.package),
        QueryEndpointSelector::Owner(owner) => {
            let owner_read =
                view.query_owner(owner, remaining_admission(total_admission, &work)?)?;
            add_query_work(&mut work, owner_read.work)?;
            if owner_read.value.is_none() {
                return Err(Diagnostic::new(
                    DiagnosticClass::Semantic,
                    "query_owner_not_found",
                    format!(
                        "relation endpoint owner '{owner}' is not live at revision '{}'",
                        binding.revision
                    ),
                ));
            }
            RelationEndpoint::Owner(ExactOwnerKey {
                package: binding.package,
                owner,
            })
        }
    };
    let logical_item_limit = request
        .limits
        .items
        .min(u64::try_from(item_record_budget).map_err(|_| {
            Diagnostic::new(
                DiagnosticClass::Resource,
                "query_record_limit",
                "compact response record capacity cannot be represented as a relation count",
            )
        })?);
    if logical_item_limit == 0 {
        return Err(Diagnostic::new(
            DiagnosticClass::Resource,
            "query_record_limit",
            "compact response has no capacity for a relation result record",
        ));
    }
    let mut relations = Vec::new();
    let mut item_bytes = 0_usize;
    let relation_read = view.visit_query_relations(
        RepositoryRelationQueryRange {
            endpoint,
            kind,
            incoming: direction == QueryDirection::Incoming,
            exclusive_lower_bound: resume_key,
            maximum_scan: request.limits.items,
            maximum_items: logical_item_limit,
        },
        remaining_admission(total_admission, &work)?,
        |edge| {
            let record_bytes = compact_record_bytes("relation", &relation_fields(edge))?;
            let required = item_bytes.checked_add(record_bytes).ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticClass::Resource,
                    "query_output_byte_overflow",
                    "relation result byte accounting overflowed",
                )
            })?;
            if required > item_byte_budget {
                if relations.is_empty() {
                    return Err(Diagnostic::new(
                        DiagnosticClass::Resource,
                        "query_output_item_too_large",
                        "one relation projection cannot fit the requested --bytes limit",
                    ));
                }
                return Ok(MapRangeControl::StopBefore);
            }
            item_bytes = required;
            relations.push(edge);
            Ok(MapRangeControl::Continue)
        },
    )?;
    add_query_work(&mut work, relation_read.work)?;
    if relation_read.revision != binding.revision {
        return Err(query_corrupt(
            "query_revision_binding",
            "relation range did not retain its pinned accepted revision",
        ));
    }
    let continuation = continuation_for_range(
        relation_read.value.has_more,
        relation_read.value.last_visited_key.as_deref(),
        binding,
        QueryOperation::Relations,
        selector,
    )?;
    Ok(QueryExecution {
        operation: QueryOperation::Relations,
        selector,
        items: QueryItems::Relations(relations),
        returned: relation_read.value.returned,
        visited: relation_read.value.visited,
        matched: None,
        truncated: Some(relation_read.value.has_more),
        context_summary: None,
        context_work: None,
        continuation,
        work,
    })
}

#[derive(Clone, Debug)]
enum ContextLogicalItem {
    Owner(ContextOwnerProjection),
    Relation(RelationEdge),
}

#[allow(
    clippy::too_many_arguments,
    reason = "the context boundary keeps selector, traversal, paging, and cancellation dimensions explicit"
)]
fn execute_context(
    view: &RepositoryView,
    total_admission: RepositoryQueryAdmission,
    request: &NormalizedQueryRequest,
    render_context: &QueryRenderContext<'_>,
    binding: QueryBinding,
    selector: QueryDigest,
    resume_key: Option<&[u8]>,
    root: OwnerKey,
    direction: ContextDirection,
    maximum_depth: u8,
    cancellation: &mut dyn FnMut() -> Result<(), Diagnostic>,
) -> Result<QueryExecution, Diagnostic> {
    let mut work = RepositoryReadWork::default();
    let root_projection =
        read_context_owner(view, root, binding, total_admission, &mut work, true)?;
    let mut selected_owners = BTreeMap::new();
    selected_owners.insert(root, (0_u8, root_projection));
    let mut selected_relations = BTreeMap::<Vec<u8>, RelationEdge>::new();
    let mut frontier = BTreeMap::from([(EncodedOwnerKey::new(root).bytes(), root)]);
    let mut expanded_owners = 0_u64;
    let mut relation_witnesses = 0_u64;
    let mut current_depth = 0_u8;

    while current_depth < maximum_depth && !frontier.is_empty() {
        let next_depth = current_depth.checked_add(1).ok_or_else(|| {
            Diagnostic::new(
                DiagnosticClass::Resource,
                "query_context_depth_overflow",
                "context traversal depth accounting overflowed",
            )
        })?;
        let mut next_frontier = BTreeMap::new();
        for owner in frontier.values().copied() {
            cancellation()?;
            expanded_owners = checked_context_count(
                expanded_owners,
                MAXIMUM_CONTEXT_OWNERS,
                "query_context_owner_limit",
                "expanded owners",
            )?;
            let mut neighbor_candidates = BTreeMap::new();
            if direction.includes_incoming() {
                visit_context_relation_range(
                    view,
                    binding,
                    owner,
                    true,
                    total_admission,
                    &mut work,
                    &mut relation_witnesses,
                    &mut selected_relations,
                    &mut neighbor_candidates,
                    cancellation,
                )?;
            }
            if direction.includes_outgoing() {
                visit_context_relation_range(
                    view,
                    binding,
                    owner,
                    false,
                    total_admission,
                    &mut work,
                    &mut relation_witnesses,
                    &mut selected_relations,
                    &mut neighbor_candidates,
                    cancellation,
                )?;
            }
            for (encoded, neighbor) in neighbor_candidates {
                if selected_owners.contains_key(&neighbor) {
                    continue;
                }
                ensure_context_capacity(
                    selected_owners.len(),
                    MAXIMUM_CONTEXT_OWNERS,
                    "query_context_owner_limit",
                    "unique local owners",
                )?;
                let projection =
                    read_context_owner(view, neighbor, binding, total_admission, &mut work, false)?;
                selected_owners.insert(neighbor, (next_depth, projection));
                next_frontier.insert(encoded, neighbor);
            }
        }
        frontier = next_frontier;
        current_depth = next_depth;
    }
    cancellation()?;

    let mut owners = selected_owners
        .into_iter()
        .map(|(owner, (depth, projection))| {
            (
                (depth, EncodedOwnerKey::new(owner).bytes()),
                ContextOwnerProjection { depth, projection },
            )
        })
        .collect::<Vec<_>>();
    owners.sort_by_key(|(key, _projection)| *key);
    let owners = owners
        .into_iter()
        .map(|(_key, projection)| projection)
        .collect::<Vec<_>>();
    let relations = selected_relations.into_values().collect::<Vec<_>>();
    let total_owners = u64::try_from(owners.len())
        .map_err(|_| query_work_overflow("selected context owner count"))?;
    let total_relations = u64::try_from(relations.len())
        .map_err(|_| query_work_overflow("selected context relation count"))?;

    let mut stream = Vec::with_capacity(owners.len().saturating_add(relations.len()));
    for owner in &owners {
        stream.push((
            context_owner_resume_key(owner.depth, owner.projection.owner),
            ContextLogicalItem::Owner(owner.clone()),
        ));
    }
    for relation in &relations {
        stream.push((
            context_relation_resume_key(*relation),
            ContextLogicalItem::Relation(*relation),
        ));
    }
    let context_summary = ContextSummary {
        total_owners,
        total_relations,
        expanded_owners,
    };
    let context_work = ContextWork {
        relation_witnesses_visited: relation_witnesses,
        expanded_owners,
        selected_owners: total_owners,
        selected_relations: total_relations,
    };
    let continuation_key = (stream.len() > 1)
        .then(|| stream.iter().max_by_key(|(key, _item)| key.len()))
        .flatten()
        .map(|(key, _item)| key.as_slice());
    let (fixed_bytes, fixed_records) = context_response_reserve(
        render_context,
        binding,
        selector,
        context_summary,
        context_work,
        &work,
        continuation_key,
        request.limits.output_bytes,
    )
    .map_err(classify_query_output_diagnostic)?;
    let item_byte_budget = request
        .limits
        .output_bytes
        .checked_sub(fixed_bytes)
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticClass::Resource,
                "query_output_envelope_too_large",
                format!(
                    "context fixed response envelope requires at least {fixed_bytes} bytes; increase --bytes"
                ),
            )
        })?;
    let item_record_budget = MAXIMUM_CLI_RESPONSE_RECORDS
        .checked_sub(fixed_records)
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticClass::Infrastructure,
                "query_output_record_configuration",
                "context fixed response records exceed the compact response record bound",
            )
        })?;
    let start = match resume_key {
        None => 0,
        Some(key) => stream
            .iter()
            .position(|(candidate, _item)| candidate == key)
            .and_then(|position| position.checked_add(1))
            .ok_or_else(|| {
                query_input_error(
                    "query_continuation_resume_key",
                    "context continuation does not identify an item in the complete selected neighborhood",
                )
            })?,
    };
    if start >= stream.len() {
        return Err(query_input_error(
            "query_continuation_resume_key",
            "context continuation resumes after the terminal selected item",
        ));
    }
    let logical_item_limit = request
        .limits
        .items
        .min(u64::try_from(item_record_budget).map_err(|_| {
            Diagnostic::new(
                DiagnosticClass::Resource,
                "query_record_limit",
                "compact response record capacity cannot be represented as a context item count",
            )
        })?);
    if logical_item_limit == 0 {
        return Err(Diagnostic::new(
            DiagnosticClass::Resource,
            "query_record_limit",
            "compact response has no capacity for a context result record",
        ));
    }
    let mut page_owners = Vec::new();
    let mut page_relations = Vec::new();
    let mut item_bytes = 0_usize;
    let mut returned = 0_u64;
    let mut last_key = None;
    for (key, item) in &stream[start..] {
        if returned >= logical_item_limit {
            break;
        }
        cancellation()?;
        let record_bytes = match item {
            ContextLogicalItem::Owner(owner) => {
                compact_record_bytes("owner", &owner.projection.fields_with_depth(owner.depth))?
            }
            ContextLogicalItem::Relation(edge) => {
                compact_record_bytes("relation", &relation_fields(*edge))?
            }
        };
        let required = item_bytes.checked_add(record_bytes).ok_or_else(|| {
            Diagnostic::new(
                DiagnosticClass::Resource,
                "query_output_byte_overflow",
                "context result byte accounting overflowed",
            )
        })?;
        if required > item_byte_budget {
            if returned == 0 {
                return Err(Diagnostic::new(
                    DiagnosticClass::Resource,
                    "query_output_item_too_large",
                    "one canonical context projection cannot fit the requested --bytes limit",
                ));
            }
            break;
        }
        item_bytes = required;
        match item {
            ContextLogicalItem::Owner(owner) => page_owners.push(owner.clone()),
            ContextLogicalItem::Relation(edge) => page_relations.push(*edge),
        }
        returned = returned
            .checked_add(1)
            .ok_or_else(|| query_work_overflow("returned context items"))?;
        last_key = Some(key.as_slice());
    }
    let end = start
        .checked_add(
            usize::try_from(returned)
                .map_err(|_| query_work_overflow("returned context item index"))?,
        )
        .ok_or_else(|| query_work_overflow("context page end"))?;
    let truncated = end < stream.len();
    let continuation = if truncated {
        let key = last_key.ok_or_else(|| {
            Diagnostic::new(
                DiagnosticClass::Resource,
                "query_output_item_too_large",
                "context query cannot advance because its first logical result does not fit the output bound",
            )
        })?;
        Some(encode_continuation(
            binding,
            QueryOperation::Context,
            selector,
            key,
        )?)
    } else {
        None
    };
    Ok(QueryExecution {
        operation: QueryOperation::Context,
        selector,
        items: QueryItems::Context {
            owners: page_owners,
            relations: page_relations,
        },
        returned,
        visited: relation_witnesses,
        matched: None,
        truncated: Some(truncated),
        context_summary: Some(context_summary),
        context_work: Some(context_work),
        continuation,
        work,
    })
}

fn read_context_owner(
    view: &RepositoryView,
    owner: OwnerKey,
    binding: QueryBinding,
    total_admission: RepositoryQueryAdmission,
    work: &mut RepositoryReadWork,
    root: bool,
) -> Result<OwnerProjection, Diagnostic> {
    let read = view.query_owner(owner, remaining_admission(total_admission, work)?)?;
    if read.revision != binding.revision {
        return Err(query_corrupt(
            "query_revision_binding",
            "context owner read did not retain its pinned accepted revision",
        ));
    }
    add_query_work(work, read.work)?;
    let record = read.value.ok_or_else(|| {
        if root {
            Diagnostic::new(
                DiagnosticClass::Semantic,
                "query_owner_not_found",
                format!(
                    "context root owner '{owner}' is not live at revision '{}'",
                    binding.revision
                ),
            )
        } else {
            query_corrupt(
                "query_context_owner_missing",
                format!(
                    "a selected local relation endpoint '{owner}' is absent from canonical authority"
                ),
            )
        }
    })?;
    if record.owner() != owner {
        return Err(query_corrupt(
            "query_context_owner_binding",
            "context owner key disagrees with its canonical owner header",
        ));
    }
    Ok(OwnerProjection::from_record(owner, &record))
}

#[allow(
    clippy::too_many_arguments,
    reason = "one relation-range step keeps exact admission, logical sets, and cancellation explicit"
)]
fn visit_context_relation_range(
    view: &RepositoryView,
    binding: QueryBinding,
    owner: OwnerKey,
    incoming: bool,
    total_admission: RepositoryQueryAdmission,
    work: &mut RepositoryReadWork,
    relation_witnesses: &mut u64,
    selected_relations: &mut BTreeMap<Vec<u8>, RelationEdge>,
    neighbor_candidates: &mut BTreeMap<[u8; 17], OwnerKey>,
    cancellation: &mut dyn FnMut() -> Result<(), Diagnostic>,
) -> Result<(), Diagnostic> {
    let remaining_witnesses = MAXIMUM_CONTEXT_RELATION_WITNESSES
        .checked_sub(*relation_witnesses)
        .ok_or_else(|| query_work_overflow("remaining context relation witnesses"))?;
    let endpoint = RelationEndpoint::Owner(ExactOwnerKey {
        package: binding.package,
        owner,
    });
    let read = view.visit_query_relations(
        RepositoryRelationQueryRange {
            endpoint,
            kind: None,
            incoming,
            exclusive_lower_bound: None,
            maximum_scan: remaining_witnesses,
            maximum_items: MAXIMUM_CONTEXT_RELATION_WITNESSES,
        },
        remaining_admission(total_admission, work)?,
        |edge| {
            cancellation()?;
            let key = forward_relation_key(edge);
            if !selected_relations.contains_key(&key) {
                ensure_context_capacity(
                    selected_relations.len(),
                    MAXIMUM_CONTEXT_RELATIONS,
                    "query_context_relation_limit",
                    "unique selected relations",
                )?;
            }
            selected_relations.insert(key, edge);
            let neighbor = if incoming { edge.source } else { edge.target };
            if let RelationEndpoint::Owner(exact) = neighbor
                && exact.package == binding.package
            {
                neighbor_candidates.insert(EncodedOwnerKey::new(exact.owner).bytes(), exact.owner);
            }
            Ok(MapRangeControl::Continue)
        },
    )?;
    if read.revision != binding.revision {
        return Err(query_corrupt(
            "query_revision_binding",
            "context relation range did not retain its pinned accepted revision",
        ));
    }
    add_query_work(work, read.work)?;
    *relation_witnesses = relation_witnesses
        .checked_add(read.value.visited)
        .ok_or_else(|| query_work_overflow("context relation witnesses"))?;
    if *relation_witnesses > MAXIMUM_CONTEXT_RELATION_WITNESSES || read.value.has_more {
        return Err(Diagnostic::new(
            DiagnosticClass::Resource,
            "query_context_witness_limit",
            format!(
                "context traversal exceeds {MAXIMUM_CONTEXT_RELATION_WITNESSES} relation-witness entries"
            ),
        ));
    }
    Ok(())
}

fn checked_context_count(
    value: u64,
    maximum: u64,
    code: &'static str,
    dimension: &'static str,
) -> Result<u64, Diagnostic> {
    let next = value
        .checked_add(1)
        .ok_or_else(|| query_work_overflow(dimension))?;
    if next > maximum {
        return Err(Diagnostic::new(
            DiagnosticClass::Resource,
            code,
            format!("context traversal exceeds {maximum} {dimension}"),
        ));
    }
    Ok(next)
}

fn ensure_context_capacity(
    current: usize,
    maximum: u64,
    code: &'static str,
    dimension: &'static str,
) -> Result<(), Diagnostic> {
    let current = u64::try_from(current).map_err(|_| query_work_overflow(dimension))?;
    if current >= maximum {
        return Err(Diagnostic::new(
            DiagnosticClass::Resource,
            code,
            format!("context traversal exceeds {maximum} {dimension}"),
        ));
    }
    Ok(())
}

const fn context_query_admission() -> RepositoryQueryAdmission {
    RepositoryQueryAdmission {
        map_pages: MAXIMUM_CONTEXT_MAP_PAGES,
        map_bytes: MAXIMUM_CONTEXT_MAP_BYTES,
        map_entries: MAXIMUM_CONTEXT_MAP_ENTRIES,
        catalog_lookups: MAXIMUM_CONTEXT_STORE_OBJECTS,
        store_objects: MAXIMUM_CONTEXT_STORE_OBJECTS,
        store_bytes: MAXIMUM_CONTEXT_STORE_BYTES,
        canonical_records: MAXIMUM_CONTEXT_OWNERS,
        witness_records: MAXIMUM_CONTEXT_RELATION_WITNESSES,
    }
}

fn continuation_for_range(
    has_more: bool,
    last_visited_key: Option<&[u8]>,
    binding: QueryBinding,
    operation: QueryOperation,
    selector: QueryDigest,
) -> Result<Option<String>, Diagnostic> {
    if !has_more {
        return Ok(None);
    }
    let key = last_visited_key.ok_or_else(|| {
        Diagnostic::new(
            DiagnosticClass::Resource,
            "query_output_item_too_large",
            "query cannot advance because its first logical result does not fit the output bound",
        )
    })?;
    encode_continuation(binding, operation, selector, key).map(Some)
}

fn owner_scan_quantum(items: u64) -> Result<u64, Diagnostic> {
    items
        .checked_mul(4)
        .map(|value| value.clamp(256, MAXIMUM_QUERY_ITEMS))
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticClass::Resource,
                "query_scan_quantum",
                "owner scan quantum overflowed its logical item request",
            )
        })
}

fn query_admission(
    scan_quantum: u64,
    item_limit: u64,
) -> Result<RepositoryQueryAdmission, Diagnostic> {
    let map_entries = scan_quantum.checked_add(4_096).ok_or_else(|| {
        Diagnostic::new(
            DiagnosticClass::Resource,
            "query_admission_map_entries",
            "query map-entry admission overflowed",
        )
    })?;
    let catalog_lookups = item_limit
        .checked_add(2_064)
        .ok_or_else(|| query_work_overflow("catalog lookup admission"))?;
    Ok(RepositoryQueryAdmission {
        map_pages: 2_048,
        map_bytes: 64 * 1_048_576,
        map_entries,
        catalog_lookups,
        store_objects: catalog_lookups,
        store_bytes: 128 * 1_048_576,
        canonical_records: item_limit
            .checked_add(4)
            .ok_or_else(|| query_work_overflow("canonical record admission"))?,
        witness_records: scan_quantum
            .checked_add(4)
            .ok_or_else(|| query_work_overflow("witness record admission"))?,
    })
}

const fn find_admission() -> RepositoryQueryAdmission {
    RepositoryQueryAdmission {
        map_pages: 128,
        map_bytes: 4 * 1_048_576,
        map_entries: 4_096,
        catalog_lookups: 256,
        store_objects: 256,
        store_bytes: 16 * 1_048_576,
        canonical_records: 4,
        witness_records: 4,
    }
}

fn remaining_admission(
    total: RepositoryQueryAdmission,
    work: &RepositoryReadWork,
) -> Result<RepositoryQueryAdmission, Diagnostic> {
    Ok(RepositoryQueryAdmission {
        map_pages: remaining_dimension(total.map_pages, work.map.pages_read, "map pages")?,
        map_bytes: remaining_dimension(total.map_bytes, work.map.bytes_read, "map bytes")?,
        map_entries: remaining_dimension(
            total.map_entries,
            work.map.entries_visited,
            "map entries",
        )?,
        catalog_lookups: remaining_dimension(
            total.catalog_lookups,
            work.store.catalog_lookups,
            "catalog lookups",
        )?,
        store_objects: remaining_dimension(
            total.store_objects,
            work.store.objects_read,
            "store objects",
        )?,
        store_bytes: remaining_dimension(total.store_bytes, work.store.bytes_read, "store bytes")?,
        canonical_records: remaining_dimension(
            total.canonical_records,
            work.canonical_records_decoded,
            "canonical records",
        )?,
        witness_records: remaining_dimension(
            total.witness_records,
            work.witness_records_decoded,
            "witness records",
        )?,
    })
}

fn remaining_dimension(maximum: u64, used: u64, dimension: &str) -> Result<u64, Diagnostic> {
    maximum.checked_sub(used).ok_or_else(|| {
        Diagnostic::new(
            DiagnosticClass::Resource,
            "query_admission_exhausted",
            format!("query {dimension} work exceeded its owning admission"),
        )
    })
}

fn add_query_work(
    total: &mut RepositoryReadWork,
    additional: RepositoryReadWork,
) -> Result<(), Diagnostic> {
    macro_rules! add {
        ($field:expr, $additional:expr, $dimension:literal) => {{
            $field = $field
                .checked_add($additional)
                .ok_or_else(|| query_work_overflow($dimension))?;
        }};
    }
    add!(
        total.map.pages_read,
        additional.map.pages_read,
        "map pages read"
    );
    add!(
        total.map.pages_decoded,
        additional.map.pages_decoded,
        "map pages decoded"
    );
    add!(
        total.map.pages_encoded,
        additional.map.pages_encoded,
        "map pages encoded"
    );
    add!(
        total.map.pages_written,
        additional.map.pages_written,
        "map pages written"
    );
    add!(
        total.map.pages_reused,
        additional.map.pages_reused,
        "map pages reused"
    );
    add!(
        total.map.bytes_read,
        additional.map.bytes_read,
        "map bytes read"
    );
    add!(
        total.map.bytes_encoded,
        additional.map.bytes_encoded,
        "map bytes encoded"
    );
    add!(
        total.map.bytes_written,
        additional.map.bytes_written,
        "map bytes written"
    );
    add!(
        total.map.key_comparisons,
        additional.map.key_comparisons,
        "map key comparisons"
    );
    add!(
        total.map.entries_visited,
        additional.map.entries_visited,
        "map entries visited"
    );
    add!(
        total.map.differences_emitted,
        additional.map.differences_emitted,
        "map differences emitted"
    );
    add!(
        total.map.subtrees_skipped,
        additional.map.subtrees_skipped,
        "map subtrees skipped"
    );
    add!(
        total.map.entries_skipped,
        additional.map.entries_skipped,
        "map entries skipped"
    );
    add!(
        total.store.catalog_lookups,
        additional.store.catalog_lookups,
        "catalog lookups"
    );
    add!(
        total.store.packs_opened,
        additional.store.packs_opened,
        "packs opened"
    );
    add!(
        total.store.objects_read,
        additional.store.objects_read,
        "store objects read"
    );
    add!(
        total.store.objects_staged,
        additional.store.objects_staged,
        "store objects staged"
    );
    add!(
        total.store.objects_reused,
        additional.store.objects_reused,
        "store objects reused"
    );
    add!(
        total.store.bytes_read,
        additional.store.bytes_read,
        "store bytes read"
    );
    add!(
        total.store.bytes_staged,
        additional.store.bytes_staged,
        "store bytes staged"
    );
    add!(
        total.store.pages_staged,
        additional.store.pages_staged,
        "store pages staged"
    );
    add!(
        total.store.packs_sealed,
        additional.store.packs_sealed,
        "store packs sealed"
    );
    add!(
        total.canonical_records_decoded,
        additional.canonical_records_decoded,
        "canonical records decoded"
    );
    add!(
        total.witness_records_decoded,
        additional.witness_records_decoded,
        "witness records decoded"
    );
    add!(
        total.items_returned,
        additional.items_returned,
        "repository items returned"
    );
    Ok(())
}

fn query_work_overflow(dimension: &'static str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Resource,
        "query_work_overflow",
        format!("query {dimension} accounting overflowed"),
    )
}

fn validate_parent_identity_class(
    class: NamespaceClass,
    parent: OwnerKey,
) -> Result<(), Diagnostic> {
    let valid = match class {
        NamespaceClass::Declaration => matches!(parent, OwnerKey::Module(_)),
        NamespaceClass::Parameter => {
            matches!(parent, OwnerKey::Declaration(_) | OwnerKey::Operation(_))
        }
        NamespaceClass::TypeParameter
        | NamespaceClass::Field
        | NamespaceClass::Case
        | NamespaceClass::Operation
        | NamespaceClass::Requirement
        | NamespaceClass::Port => matches!(parent, OwnerKey::Declaration(_)),
        NamespaceClass::Module | NamespaceClass::Target => false,
    };
    if !valid {
        return Err(query_input_error(
            "query_invalid_parent_domain",
            format!(
                "owner '{parent}' cannot be a parent for namespace class '{}'",
                class.name()
            ),
        ));
    }
    Ok(())
}

fn validate_live_parent(
    class: NamespaceClass,
    parent: OwnerKey,
    record: &super::kernel::OwnerRecord,
) -> Result<(), Diagnostic> {
    if record.owner() != parent {
        return Err(query_corrupt(
            "query_parent_owner_disagreement",
            "live parent record disagrees with its canonical owner key",
        ));
    }
    validate_parent_identity_class(class, parent)
}

fn exact_namespace_projection(
    owner: OwnerKey,
    record: &super::kernel::OwnerRecord,
    class: NamespaceClass,
    name: &Name,
    parent: Option<OwnerKey>,
) -> Result<OwnerProjection, Diagnostic> {
    if record.owner() != owner {
        return Err(query_corrupt(
            "query_namespace_owner_disagreement",
            "namespace witness owner key disagrees with the live canonical owner header",
        ));
    }
    let observed = owner_namespace(record).ok_or_else(|| {
        query_corrupt(
            "query_namespace_owner_disagreement",
            "namespace witness references a canonical owner without namespace facts",
        )
    })?;
    if observed.class != class || observed.parent != parent || observed.name != name {
        return Err(query_corrupt(
            "query_namespace_owner_disagreement",
            "namespace witness and live canonical owner disagree on class, parent, or name",
        ));
    }
    Ok(OwnerProjection::from_record(owner, record))
}

fn relation_fields(edge: RelationEdge) -> Vec<(&'static str, String)> {
    let (source_package, source_owner) = endpoint_fields(edge.source);
    let (target_package, target_owner) = endpoint_fields(edge.target);
    let mut fields = vec![
        ("kind", edge.kind.name().to_owned()),
        ("source-package", source_package),
    ];
    if let Some(source_owner) = source_owner {
        fields.push(("source-owner", source_owner));
    }
    fields.push(("target-package", target_package));
    if let Some(target_owner) = target_owner {
        fields.push(("target-owner", target_owner));
    }
    fields
}

fn endpoint_fields(endpoint: RelationEndpoint) -> (String, Option<String>) {
    match endpoint {
        RelationEndpoint::Package(package) => (package.to_string(), None),
        RelationEndpoint::Owner(owner) => {
            (owner.package.to_string(), Some(owner.owner.to_string()))
        }
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "the context envelope reserve binds exact selector, totals, work, continuation, and output dimensions"
)]
fn context_response_reserve(
    context: &QueryRenderContext<'_>,
    binding: QueryBinding,
    selector: QueryDigest,
    summary: ContextSummary,
    context_work: ContextWork,
    work: &RepositoryReadWork,
    continuation_key: Option<&[u8]>,
    output_limit: usize,
) -> Result<(usize, usize), Diagnostic> {
    let mut writer = CompactResponseWriter::new(CompactResponseLimits {
        maximum_bytes: output_limit,
        maximum_records: MAXIMUM_CLI_RESPONSE_RECORDS,
    })?;
    append_response_header(&mut writer, context, QueryOperation::Context, selector)?;
    append_fields(
        &mut writer,
        "summary",
        &[
            ("returned", request_maximum_returned_text()),
            ("truncated", "false".to_owned()),
            ("total-owners", summary.total_owners.to_string()),
            ("total-relations", summary.total_relations.to_string()),
            ("expanded-owners", summary.expanded_owners.to_string()),
        ],
    )?;
    if let Some(key) = continuation_key {
        append_fields(
            &mut writer,
            "continuation",
            &[(
                "token",
                encode_continuation(binding, QueryOperation::Context, selector, key)?,
            )],
        )?;
    }
    append_work_record(
        &mut writer,
        work,
        Some(context_work),
        MAXIMUM_QUERY_OUTPUT_BYTES,
    )?;
    append_fields(
        &mut writer,
        "schema",
        &[("capabilities", context.capabilities_digest.to_owned())],
    )?;
    Ok((writer.byte_count(), writer.record_count()))
}

fn request_maximum_returned_text() -> String {
    MAXIMUM_QUERY_ITEMS.to_string()
}

fn fixed_response_reserve(
    context: &QueryRenderContext<'_>,
    operation: QueryOperation,
    selector: QueryDigest,
    output_limit: usize,
) -> Result<(usize, usize), Diagnostic> {
    let mut writer = CompactResponseWriter::new(CompactResponseLimits {
        maximum_bytes: output_limit,
        maximum_records: MAXIMUM_CLI_RESPONSE_RECORDS,
    })?;
    append_response_header(&mut writer, context, operation, selector)?;
    let maximum = u64::MAX.to_string();
    match operation {
        QueryOperation::Find => append_fields(
            &mut writer,
            "summary",
            &[
                ("returned", maximum.clone()),
                ("visited", maximum.clone()),
                ("match", "false".to_owned()),
            ],
        )?,
        QueryOperation::Owners | QueryOperation::Relations => {
            append_fields(
                &mut writer,
                "summary",
                &[
                    ("returned", maximum.clone()),
                    ("visited", maximum.clone()),
                    ("truncated", "false".to_owned()),
                ],
            )?;
            append_fields(
                &mut writer,
                "continuation",
                &[("token", "x".repeat(MAXIMUM_QUERY_CONTINUATION_BYTES))],
            )?;
        }
        QueryOperation::Context => {
            append_fields(
                &mut writer,
                "summary",
                &[
                    ("returned", maximum.clone()),
                    ("truncated", "false".to_owned()),
                    ("total-owners", maximum.clone()),
                    ("total-relations", maximum.clone()),
                    ("expanded-owners", maximum.clone()),
                ],
            )?;
            append_fields(
                &mut writer,
                "continuation",
                &[("token", "x".repeat(MAXIMUM_QUERY_CONTINUATION_BYTES))],
            )?;
        }
    }
    let mut maximum_map_work = super::persistent_map::MapWork::default();
    maximum_map_work.pages_read = u64::MAX;
    maximum_map_work.bytes_read = u64::MAX;
    maximum_map_work.entries_visited = u64::MAX;
    let maximum_store_work = super::storage::object::StoreWork {
        catalog_lookups: u64::MAX,
        objects_read: u64::MAX,
        bytes_read: u64::MAX,
        ..super::storage::object::StoreWork::default()
    };
    append_work_record(
        &mut writer,
        &RepositoryReadWork {
            map: maximum_map_work,
            store: maximum_store_work,
            canonical_records_decoded: u64::MAX,
            witness_records_decoded: u64::MAX,
            items_returned: u64::MAX,
        },
        (operation == QueryOperation::Context).then_some(ContextWork {
            relation_witnesses_visited: u64::MAX,
            expanded_owners: u64::MAX,
            selected_owners: u64::MAX,
            selected_relations: u64::MAX,
        }),
        MAXIMUM_QUERY_OUTPUT_BYTES,
    )?;
    append_fields(
        &mut writer,
        "schema",
        &[("capabilities", context.capabilities_digest.to_owned())],
    )?;
    Ok((writer.byte_count(), writer.record_count()))
}

fn render_query_response(
    context: &QueryRenderContext<'_>,
    execution: &QueryExecution,
    output_limit: usize,
) -> Result<Vec<u8>, Diagnostic> {
    let mut rendered_bytes = 0_usize;
    for _ in 0..4 {
        let bytes = render_query_response_once(context, execution, output_limit, rendered_bytes)?;
        if bytes.len() == rendered_bytes {
            return Ok(bytes);
        }
        rendered_bytes = bytes.len();
    }
    Err(Diagnostic::new(
        DiagnosticClass::Infrastructure,
        "query_output_size_convergence",
        "compact query response byte count did not converge",
    ))
}

fn render_query_response_once(
    context: &QueryRenderContext<'_>,
    execution: &QueryExecution,
    output_limit: usize,
    rendered_bytes: usize,
) -> Result<Vec<u8>, Diagnostic> {
    let mut writer = CompactResponseWriter::new(CompactResponseLimits {
        maximum_bytes: output_limit.min(MAXIMUM_CLI_RESPONSE_BYTES),
        maximum_records: MAXIMUM_CLI_RESPONSE_RECORDS,
    })?;
    append_response_header(
        &mut writer,
        context,
        execution.operation,
        execution.selector,
    )?;
    match &execution.items {
        QueryItems::Owners(owners) => {
            for owner in owners {
                append_fields(&mut writer, "owner", &owner.fields())?;
            }
        }
        QueryItems::Relations(relations) => {
            for relation in relations {
                append_fields(&mut writer, "relation", &relation_fields(*relation))?;
            }
        }
        QueryItems::Context { owners, relations } => {
            for owner in owners {
                append_fields(
                    &mut writer,
                    "owner",
                    &owner.projection.fields_with_depth(owner.depth),
                )?;
            }
            for relation in relations {
                append_fields(&mut writer, "relation", &relation_fields(*relation))?;
            }
        }
    }
    let mut summary = vec![("returned", execution.returned.to_string())];
    if execution.context_summary.is_none() {
        summary.push(("visited", execution.visited.to_string()));
    }
    if let Some(matched) = execution.matched {
        summary.push(("match", matched.to_string()));
    }
    if let Some(truncated) = execution.truncated {
        summary.push(("truncated", truncated.to_string()));
    }
    if let Some(context) = execution.context_summary {
        summary.push(("total-owners", context.total_owners.to_string()));
        summary.push(("total-relations", context.total_relations.to_string()));
        summary.push(("expanded-owners", context.expanded_owners.to_string()));
    }
    append_fields(&mut writer, "summary", &summary)?;
    if let Some(continuation) = &execution.continuation {
        append_fields(
            &mut writer,
            "continuation",
            &[("token", continuation.clone())],
        )?;
    }
    append_work_record(
        &mut writer,
        &execution.work,
        execution.context_work,
        rendered_bytes,
    )?;
    append_fields(
        &mut writer,
        "schema",
        &[("capabilities", context.capabilities_digest.to_owned())],
    )?;
    Ok(writer.finish())
}

fn append_response_header(
    writer: &mut CompactResponseWriter,
    context: &QueryRenderContext<'_>,
    operation: QueryOperation,
    selector: QueryDigest,
) -> Result<(), Diagnostic> {
    append_fields(
        writer,
        "result",
        &[
            ("status", "success".to_owned()),
            ("command", operation.command().to_owned()),
        ],
    )?;
    append_fields(
        writer,
        "project",
        &[
            ("path", context.repository_root.display().to_string()),
            ("name", context.project_name.to_owned()),
            ("repository", context.repository.to_string()),
            ("package", context.package.to_string()),
        ],
    )?;
    append_fields(
        writer,
        "revision",
        &[("observed", context.revision.to_string())],
    )?;
    append_fields(
        writer,
        "query",
        &[
            ("operation", operation.action().to_owned()),
            ("digest", selector.to_string()),
        ],
    )
}

fn append_work_record(
    writer: &mut CompactResponseWriter,
    work: &RepositoryReadWork,
    context: Option<ContextWork>,
    rendered_bytes: usize,
) -> Result<(), Diagnostic> {
    let mut fields = vec![
        ("map-pages-read", work.map.pages_read.to_string()),
        ("map-bytes-read", work.map.bytes_read.to_string()),
        ("map-entries-visited", work.map.entries_visited.to_string()),
        ("catalog-lookups", work.store.catalog_lookups.to_string()),
        ("store-objects-read", work.store.objects_read.to_string()),
        ("store-bytes-read", work.store.bytes_read.to_string()),
        (
            "canonical-records-decoded",
            work.canonical_records_decoded.to_string(),
        ),
        (
            "witness-records-decoded",
            work.witness_records_decoded.to_string(),
        ),
    ];
    if let Some(context) = context {
        fields.push((
            "relation-witnesses-visited",
            context.relation_witnesses_visited.to_string(),
        ));
        fields.push(("expanded-owners", context.expanded_owners.to_string()));
        fields.push(("selected-owners", context.selected_owners.to_string()));
        fields.push(("selected-relations", context.selected_relations.to_string()));
    }
    fields.push(("rendered-output-bytes", rendered_bytes.to_string()));
    append_fields(writer, "work", &fields)
}

fn compact_record_bytes(
    operation: &str,
    fields: &[(&'static str, String)],
) -> Result<usize, Diagnostic> {
    let mut writer = CompactResponseWriter::new(CompactResponseLimits {
        maximum_bytes: MAXIMUM_QUERY_OUTPUT_BYTES,
        maximum_records: 1,
    })?;
    append_fields(&mut writer, operation, fields)?;
    Ok(writer.byte_count())
}

fn append_fields(
    writer: &mut CompactResponseWriter,
    operation: &str,
    fields: &[(&str, String)],
) -> Result<(), Diagnostic> {
    for (name, _value) in fields {
        if !QUERY_RESPONSE_FIELDS.contains(&(operation, *name)) {
            return Err(Diagnostic::new(
                DiagnosticClass::Infrastructure,
                "query_response_field_inventory",
                format!(
                    "query renderer field '{operation}.{name}' is absent from its executable inventory"
                ),
            ));
        }
    }
    let borrowed = fields
        .iter()
        .map(|(name, value)| (*name, value.as_str()))
        .collect::<Vec<_>>();
    writer.append_record(operation, &borrowed)
}

fn query_corrupt(code: &'static str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Corrupt, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::control::{CompactRecord, parse_records};
    use crate::platform::kernel::{
        DeclarationPayload, DeclarationRecord, DeclarationReference, DeclarationVisibility,
        ExternalDeclaration, FieldRecord, ImplementationName, KernelSnapshot, ModuleRecord,
        OwnerHeader, OwnerRecord, StructuralTypeField, TypeForm, TypeObjectInterner, encode_owner,
        extract_relations,
    };
    use crate::platform::semantic_id::{DeclarationId, FieldId, ModuleId};
    use crate::platform::storage::object::{ObjectDomain, ObjectKey};
    use crate::platform::witness::{forward_relation_key, reverse_relation_key};
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs;
    use std::io::{Read, Seek, SeekFrom, Write};
    use std::time::{Duration, Instant};

    fn field<'a>(record: &'a CompactRecord, name: &str) -> Option<&'a str> {
        record
            .fields
            .iter()
            .find(|field| field.name == name)
            .map(|field| field.value.as_str())
    }

    fn records(bytes: &[u8]) -> Vec<CompactRecord> {
        parse_records("<normalized-query-test>", bytes).expect("compact query response")
    }

    fn raw_continuation(token: &str) -> Vec<u8> {
        base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(
                token
                    .strip_prefix(QUERY_CONTINUATION_PREFIX)
                    .expect("query continuation prefix"),
            )
            .expect("query continuation base64")
    }

    fn sealed_continuation(mut bytes_without_checksum: Vec<u8>) -> String {
        bytes_without_checksum.extend_from_slice(&domain_digest(
            QUERY_CONTINUATION_INTEGRITY_DOMAIN,
            &bytes_without_checksum,
        ));
        format!(
            "{QUERY_CONTINUATION_PREFIX}{}",
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes_without_checksum)
        )
    }

    fn fixture() -> (tempfile::TempDir, GraphRepository, KernelSnapshot) {
        let temporary = tempfile::tempdir().expect("query fixture parent");
        let destination = temporary.path().join("project");
        let snapshot = crate::platform::kernel::tests::witness_snapshot();
        let created = GraphRepository::create(&destination, &snapshot, None)
            .expect("normalized query repository");
        (temporary, created.repository, snapshot)
    }

    fn named_owner(snapshot: &KernelSnapshot, name: &str) -> OwnerKey {
        snapshot
            .owners
            .iter()
            .find_map(|(owner, record)| {
                record
                    .name()
                    .is_some_and(|candidate| candidate.as_str() == name)
                    .then_some(*owner)
            })
            .expect("named fixture owner")
    }

    fn relation_from_record(record: &CompactRecord) -> RelationEdge {
        let endpoint = |package_name: &str, owner_name: &str| {
            let package = field(record, package_name)
                .expect("relation package")
                .parse::<PackageId>()
                .expect("typed package");
            match field(record, owner_name) {
                Some(owner) => RelationEndpoint::Owner(ExactOwnerKey {
                    package,
                    owner: owner.parse().expect("typed relation owner"),
                }),
                None => RelationEndpoint::Package(package),
            }
        };
        RelationEdge {
            source: endpoint("source-package", "source-owner"),
            kind: RelationKind::parse(field(record, "kind").expect("relation kind"))
                .expect("canonical relation kind"),
            target: endpoint("target-package", "target-owner"),
        }
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct OracleContext {
        owners: Vec<(OwnerKey, u8)>,
        relations: Vec<RelationEdge>,
        expanded_owners: u64,
        relation_witnesses: u64,
    }

    fn full_context_oracle(
        snapshot: &KernelSnapshot,
        root: OwnerKey,
        direction: ContextDirection,
        maximum_depth: u8,
    ) -> OracleContext {
        assert!(snapshot.owners.contains_key(&root));
        let all_relations = extract_relations(
            snapshot.root.package_id,
            &snapshot.owners,
            &snapshot.types,
            &snapshot.dependencies,
        )
        .expect("complete context relation oracle");
        let mut distances = BTreeMap::from([(root, 0_u8)]);
        let mut selected = BTreeMap::new();
        let mut frontier = BTreeMap::from([(EncodedOwnerKey::new(root).bytes(), root)]);
        let mut expanded_owners = 0_u64;
        let mut relation_witnesses = 0_u64;
        let mut current_depth = 0_u8;
        while current_depth < maximum_depth && !frontier.is_empty() {
            let next_depth = current_depth + 1;
            let mut next = BTreeMap::new();
            for owner in frontier.values().copied() {
                expanded_owners += 1;
                let endpoint = RelationEndpoint::Owner(ExactOwnerKey {
                    package: snapshot.root.package_id,
                    owner,
                });
                for incoming in [true, false] {
                    if (incoming && !direction.includes_incoming())
                        || (!incoming && !direction.includes_outgoing())
                    {
                        continue;
                    }
                    let mut observed = all_relations
                        .iter()
                        .copied()
                        .filter(|edge| {
                            if incoming {
                                edge.target == endpoint
                            } else {
                                edge.source == endpoint
                            }
                        })
                        .map(|edge| {
                            let key = if incoming {
                                reverse_relation_key(edge)
                            } else {
                                forward_relation_key(edge)
                            };
                            (key, edge)
                        })
                        .collect::<Vec<_>>();
                    observed.sort_by(|left, right| left.0.cmp(&right.0));
                    for (_witness_key, edge) in observed {
                        relation_witnesses += 1;
                        selected.insert(forward_relation_key(edge), edge);
                        let neighbor = if incoming { edge.source } else { edge.target };
                        let RelationEndpoint::Owner(exact) = neighbor else {
                            continue;
                        };
                        if exact.package != snapshot.root.package_id
                            || !snapshot.owners.contains_key(&exact.owner)
                            || distances.contains_key(&exact.owner)
                        {
                            continue;
                        }
                        distances.insert(exact.owner, next_depth);
                        next.insert(EncodedOwnerKey::new(exact.owner).bytes(), exact.owner);
                    }
                }
            }
            frontier = next;
            current_depth = next_depth;
        }
        let mut owners = distances.into_iter().collect::<Vec<_>>();
        owners.sort_by_key(|(owner, depth)| (*depth, EncodedOwnerKey::new(*owner).bytes()));
        OracleContext {
            owners,
            relations: selected.into_values().collect(),
            expanded_owners,
            relation_witnesses,
        }
    }

    fn collect_context_pages(
        repository: &GraphRepository,
        view: &RepositoryView,
        root: OwnerKey,
        direction: ContextDirection,
        depth: u8,
    ) -> (OracleContext, Vec<Vec<CompactRecord>>) {
        let mut request = NormalizedQueryRequest {
            selection: QuerySelection::Context {
                root,
                direction,
                depth,
            },
            limits: QueryPageLimits {
                items: 1,
                output_bytes: MINIMUM_QUERY_OUTPUT_BYTES,
            },
            continuation: None,
        };
        let mut owners = Vec::new();
        let mut relations = Vec::new();
        let mut pages = Vec::new();
        let mut expected_summary = None;
        let mut expected_work = None;
        let mut relation_section = false;
        for page_index in 0..10_000_u64 {
            request.limits.items = if page_index.is_multiple_of(2) { 1 } else { 7 };
            request.limits.output_bytes = if page_index.is_multiple_of(3) {
                MINIMUM_QUERY_OUTPUT_BYTES
            } else {
                4_096
            };
            let bytes =
                execute_normalized_query(repository, view, &request).expect("context query page");
            let page = records(&bytes);
            let summary = page
                .iter()
                .find(|record| record.operation == "summary")
                .expect("context summary");
            let work = page
                .iter()
                .find(|record| record.operation == "work")
                .expect("context work");
            let summary_identity = (
                field(summary, "total-owners")
                    .expect("total owners")
                    .to_owned(),
                field(summary, "total-relations")
                    .expect("total relations")
                    .to_owned(),
                field(summary, "expanded-owners")
                    .expect("expanded owners")
                    .to_owned(),
            );
            let work_identity = (
                field(work, "relation-witnesses-visited")
                    .expect("relation witness work")
                    .to_owned(),
                field(work, "selected-owners")
                    .expect("selected owner work")
                    .to_owned(),
                field(work, "selected-relations")
                    .expect("selected relation work")
                    .to_owned(),
            );
            assert_eq!(
                field(work, "rendered-output-bytes")
                    .expect("rendered output work")
                    .parse::<usize>()
                    .expect("rendered output number"),
                bytes.len()
            );
            if let Some(expected) = &expected_summary {
                assert_eq!(&summary_identity, expected);
            } else {
                expected_summary = Some(summary_identity);
            }
            if let Some(expected) = &expected_work {
                assert_eq!(&work_identity, expected);
            } else {
                expected_work = Some(work_identity);
            }
            for record in &page {
                match record.operation.as_str() {
                    "owner" => {
                        assert!(!relation_section, "owners must precede relations");
                        owners.push((
                            field(record, "id")
                                .expect("context owner id")
                                .parse::<OwnerKey>()
                                .expect("typed context owner"),
                            field(record, "depth")
                                .expect("context owner depth")
                                .parse::<u8>()
                                .expect("typed context depth"),
                        ));
                    }
                    "relation" => {
                        relation_section = true;
                        relations.push(relation_from_record(record));
                    }
                    _ => {}
                }
            }
            request.continuation = page
                .iter()
                .find(|record| record.operation == "continuation")
                .and_then(|record| field(record, "token"))
                .map(str::to_owned);
            pages.push(page);
            if request.continuation.is_none() {
                let summary = expected_summary.expect("context summary identity");
                let work = expected_work.expect("context work identity");
                return (
                    OracleContext {
                        owners,
                        relations,
                        expanded_owners: summary.2.parse().expect("expanded owner summary"),
                        relation_witnesses: work.0.parse().expect("relation witness work"),
                    },
                    pages,
                );
            }
        }
        panic!("context pagination did not terminate");
    }

    fn maximum_context_snapshot(named_relation_count: usize) -> (KernelSnapshot, OwnerKey) {
        const TARGETS: usize = 2_044;
        const SOURCES: usize = 7;
        assert!(named_relation_count <= TARGETS * SOURCES);
        let mut snapshot = crate::platform::kernel::tests::witness_snapshot();
        let package = snapshot.root.package_id;
        let module_id = ModuleId::migrate(b"maximum-context", 0);
        let module = OwnerKey::Module(module_id);
        let mut owners = BTreeMap::from([(
            module,
            OwnerRecord::Module(ModuleRecord {
                header: OwnerHeader::new(module, OwnerKind::Module),
                name: Name::new("maximum_context").expect("maximum context module name"),
            }),
        )]);
        let mut interner = TypeObjectInterner::default();
        let unit = interner
            .intern(TypeForm::Unit)
            .expect("maximum context unit type");
        let mut targets = Vec::with_capacity(TARGETS);
        for ordinal in 0..TARGETS {
            let declaration_id = DeclarationId::migrate(b"maximum-context-target", ordinal as u64);
            let declaration = OwnerKey::Declaration(declaration_id);
            let field_id = FieldId::migrate(b"maximum-context-field", ordinal as u64);
            let field = OwnerKey::Field(field_id);
            owners.insert(
                declaration,
                OwnerRecord::Declaration(DeclarationRecord {
                    header: OwnerHeader::new(declaration, OwnerKind::Record),
                    module: module_id,
                    name: Name::new(format!("Target_{ordinal:04}"))
                        .expect("maximum context target name"),
                    visibility: DeclarationVisibility::Private,
                    payload: DeclarationPayload::Record {
                        fields: vec![field_id],
                    },
                }),
            );
            owners.insert(
                field,
                OwnerRecord::Field(FieldRecord {
                    header: OwnerHeader::new(field, OwnerKind::Field),
                    declaration: declaration_id,
                    name: Name::new("value").expect("maximum context field name"),
                    ty: unit,
                }),
            );
            targets.push(declaration_id);
        }
        let named_types = targets
            .iter()
            .map(|declaration| {
                interner
                    .intern(TypeForm::Named {
                        declaration: DeclarationReference {
                            package,
                            declaration: *declaration,
                        },
                    })
                    .expect("maximum context named type")
            })
            .collect::<Vec<_>>();
        let quotient = named_relation_count / SOURCES;
        let remainder = named_relation_count % SOURCES;
        for source_ordinal in 0..SOURCES {
            let selected = quotient + usize::from(source_ordinal < remainder);
            assert!(selected <= TARGETS);
            let offset = (source_ordinal * 271) % TARGETS;
            let fields = (0..selected)
                .map(|field_ordinal| StructuralTypeField {
                    name: Name::new(format!("target_{field_ordinal:04}"))
                        .expect("maximum context structural field name"),
                    ty: named_types[(offset + field_ordinal) % TARGETS],
                })
                .collect::<Vec<_>>();
            let result = interner
                .intern(TypeForm::StructuralRecord { fields })
                .expect("maximum context structural result");
            let declaration_id =
                DeclarationId::migrate(b"maximum-context-source", source_ordinal as u64);
            let declaration = OwnerKey::Declaration(declaration_id);
            owners.insert(
                declaration,
                OwnerRecord::Declaration(DeclarationRecord {
                    header: OwnerHeader::new(declaration, OwnerKind::External),
                    module: module_id,
                    name: Name::new(format!("source_{source_ordinal:02}"))
                        .expect("maximum context source name"),
                    visibility: DeclarationVisibility::Private,
                    payload: DeclarationPayload::External(ExternalDeclaration {
                        type_parameters: Vec::new(),
                        parameters: Vec::new(),
                        result,
                        implementation: ImplementationName::new("maximum_context_source")
                            .expect("maximum context implementation"),
                    }),
                }),
            );
        }
        assert_eq!(owners.len() as u64, MAXIMUM_CONTEXT_OWNERS);
        snapshot.owners = owners;
        snapshot.types = interner.into_objects();
        snapshot.dependencies.clear();
        snapshot.dependency_interfaces.clear();
        snapshot.dependency_types.clear();
        snapshot.blobs.clear();
        snapshot.retirements.clear();
        let owner_root = snapshot.root.owners;
        snapshot.root.owners = super::super::persistent_map::MapRoot::from_parts(
            owner_root.page(),
            MAXIMUM_CONTEXT_OWNERS,
            owner_root.content(),
        );
        (snapshot, module)
    }

    fn owner_limit_context_snapshot(neighbor_count: usize) -> (KernelSnapshot, OwnerKey) {
        let mut snapshot = crate::platform::kernel::tests::witness_snapshot();
        let module_id = ModuleId::migrate(b"owner-limit-context", 0);
        let module = OwnerKey::Module(module_id);
        let mut owners = BTreeMap::from([(
            module,
            OwnerRecord::Module(ModuleRecord {
                header: OwnerHeader::new(module, OwnerKind::Module),
                name: Name::new("owner_limit_context").expect("owner-limit module name"),
            }),
        )]);
        let mut interner = TypeObjectInterner::default();
        let unit = interner
            .intern(TypeForm::Unit)
            .expect("owner-limit unit type");
        for ordinal in 0..neighbor_count {
            let declaration_id = DeclarationId::migrate(b"owner-limit-context", ordinal as u64 + 1);
            let declaration = OwnerKey::Declaration(declaration_id);
            owners.insert(
                declaration,
                OwnerRecord::Declaration(DeclarationRecord {
                    header: OwnerHeader::new(declaration, OwnerKind::External),
                    module: module_id,
                    name: Name::new(format!("Neighbor_{ordinal:04}"))
                        .expect("owner-limit declaration name"),
                    visibility: DeclarationVisibility::Private,
                    payload: DeclarationPayload::External(ExternalDeclaration {
                        type_parameters: Vec::new(),
                        parameters: Vec::new(),
                        result: unit,
                        implementation: ImplementationName::new("owner_limit_context")
                            .expect("owner-limit implementation name"),
                    }),
                }),
            );
        }
        snapshot.owners = owners;
        snapshot.types = interner.into_objects();
        snapshot.dependencies.clear();
        snapshot.dependency_interfaces.clear();
        snapshot.dependency_types.clear();
        snapshot.blobs.clear();
        snapshot.retirements.clear();
        let owner_root = snapshot.root.owners;
        snapshot.root.owners = super::super::persistent_map::MapRoot::from_parts(
            owner_root.page(),
            u64::try_from(snapshot.owners.len()).expect("owner-limit owner count"),
            owner_root.content(),
        );
        (snapshot, module)
    }

    fn file_inventory(root: &Path) -> BTreeMap<String, [u8; 32]> {
        fn visit(root: &Path, current: &Path, output: &mut BTreeMap<String, [u8; 32]>) {
            let mut entries = fs::read_dir(current)
                .expect("query inventory directory")
                .collect::<Result<Vec<_>, _>>()
                .expect("query inventory entries");
            entries.sort_by_key(|entry| entry.file_name());
            for entry in entries {
                let path = entry.path();
                let metadata = entry.metadata().expect("query inventory metadata");
                if metadata.is_dir() {
                    visit(root, &path, output);
                } else if metadata.is_file() {
                    let relative = path
                        .strip_prefix(root)
                        .expect("inventory path under root")
                        .to_string_lossy()
                        .into_owned();
                    let bytes = fs::read(&path).expect("query inventory file");
                    output.insert(relative, *blake3::hash(&bytes).as_bytes());
                }
            }
        }
        let mut output = BTreeMap::new();
        visit(root, root, &mut output);
        output
    }

    fn corrupt_repository_object(repository: &GraphRepository, key: ObjectKey) {
        let store = repository
            .object_store()
            .expect("query corruption object store");
        let location = store
            .catalog_location(key)
            .expect("catalog lookup")
            .expect("corrupt object location");
        let pack = repository
            .root()
            .join("packs")
            .join(location.pack.file_name());
        drop(store);
        let mut file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(pack)
            .expect("open isolated corrupt query pack");
        file.seek(SeekFrom::Start(location.offset))
            .expect("seek selected query object");
        let mut byte = [0_u8; 1];
        file.read_exact(&mut byte)
            .expect("read selected query byte");
        byte[0] ^= 0x80;
        file.seek(SeekFrom::Start(location.offset))
            .expect("seek selected query byte again");
        file.write_all(&byte).expect("corrupt selected query byte");
        file.sync_all().expect("sync isolated query corruption");
    }

    fn measured_scale_query(
        label: &str,
        repository: &GraphRepository,
        view: &RepositoryView,
        request: &NormalizedQueryRequest,
    ) -> (Vec<CompactRecord>, Duration) {
        let started = Instant::now();
        let bytes = execute_normalized_query(repository, view, request).expect("scale query");
        let elapsed = started.elapsed();
        let output = records(&bytes);
        let summary = output
            .iter()
            .find(|record| record.operation == "summary")
            .expect("scale summary");
        let work = output
            .iter()
            .find(|record| record.operation == "work")
            .expect("scale work");
        let number = |record: &CompactRecord, name: &str| {
            field(record, name)
                .expect("scale numeric field")
                .parse::<u64>()
                .expect("scale numeric value")
        };
        assert_eq!(number(work, "rendered-output-bytes"), bytes.len() as u64);
        println!(
            "query-scale scenario={label} wall-micros={} output-bytes={} records={} returned={} continuation={} map-pages-read={} map-bytes-read={} map-entries-visited={} catalog-lookups={} store-objects-read={} store-bytes-read={} canonical-records-decoded={} witness-records-decoded={}",
            elapsed.as_micros(),
            bytes.len(),
            output.len(),
            number(summary, "returned"),
            output
                .iter()
                .any(|record| record.operation == "continuation"),
            number(work, "map-pages-read"),
            number(work, "map-bytes-read"),
            number(work, "map-entries-visited"),
            number(work, "catalog-lookups"),
            number(work, "store-objects-read"),
            number(work, "store-bytes-read"),
            number(work, "canonical-records-decoded"),
            number(work, "witness-records-decoded"),
        );
        (output, elapsed)
    }

    fn process_peak_rss_kib() -> Option<u64> {
        let status = fs::read_to_string("/proc/self/status").ok()?;
        status.lines().find_map(|line| {
            let value = line.strip_prefix("VmHWM:")?.trim();
            value.strip_suffix("kB")?.trim().parse::<u64>().ok()
        })
    }

    #[test]
    fn query_parser_is_descriptor_closed_and_rejects_predecessor_grammar() {
        for descriptor in QUERY_OPERATION_DESCRIPTORS {
            assert!(matches!(
                descriptor.action,
                "owners" | "find" | "relations" | "context"
            ));
        }
        for kind in OwnerKind::ALL {
            assert_eq!(
                OwnerKind::parse(kind.name()).expect("owner-kind name"),
                kind
            );
        }
        for class in NamespaceClass::ALL {
            assert_eq!(
                NamespaceClass::parse(class.name()).expect("namespace-class name"),
                class
            );
        }
        for kind in RelationKind::ALL {
            assert_eq!(
                RelationKind::parse(kind.name()).expect("relation-kind name"),
                kind
            );
        }
        let owners = parse_query_arguments(&[
            "owners".to_owned(),
            "--kind".to_owned(),
            "pure_function".to_owned(),
            "--limit".to_owned(),
            "7".to_owned(),
            "--bytes".to_owned(),
            "4096".to_owned(),
        ])
        .expect("owners grammar");
        assert_eq!(owners.limits.items, 7);
        assert_eq!(owners.limits.output_bytes, 4096);
        assert!(matches!(
            owners.selection,
            QuerySelection::Owners {
                kind: Some(OwnerKind::PureFunction)
            }
        ));

        let context_root = "mod_00000000000000000000000000000001";
        let context = parse_query_arguments(&[
            "context".to_owned(),
            context_root.to_owned(),
            "--direction".to_owned(),
            "both".to_owned(),
            "--depth".to_owned(),
            "8".to_owned(),
            "--limit".to_owned(),
            "7".to_owned(),
            "--bytes".to_owned(),
            "4096".to_owned(),
        ])
        .expect("context grammar");
        assert!(matches!(
            context.selection,
            QuerySelection::Context {
                direction: ContextDirection::Both,
                depth: 8,
                ..
            }
        ));
        for arguments in [
            vec!["context", context_root, "--direction", "both"],
            vec!["context", context_root, "--depth", "1"],
            vec![
                "context",
                context_root,
                "--direction",
                "both",
                "--depth",
                "0",
            ],
            vec![
                "context",
                context_root,
                "--direction",
                "both",
                "--depth",
                "09",
            ],
            vec![
                "context",
                context_root,
                "--direction",
                "both",
                "--depth",
                "1",
                "--kind",
                "function_call",
            ],
        ] {
            assert!(
                parse_query_arguments(
                    &arguments.into_iter().map(str::to_owned).collect::<Vec<_>>()
                )
                .is_err()
            );
        }

        for action in [
            "callers",
            "callees",
            "types",
            "capabilities",
            "impact",
            "request",
        ] {
            assert_eq!(
                parse_query_arguments(&[action.to_owned()])
                    .expect_err("predecessor action")
                    .code,
                "predecessor_contract"
            );
        }
        assert_eq!(
            parse_query_arguments(&["owners".to_owned(), "--work".to_owned(), "1".to_owned(),])
                .expect_err("scalar query work option")
                .code,
            "query_unknown_option"
        );
        assert_eq!(
            parse_query_arguments(&[
                "find".to_owned(),
                "module".to_owned(),
                "root".to_owned(),
                "--parent".to_owned(),
                "mod_00000000000000000000000000000001".to_owned(),
            ])
            .expect_err("root namespace parent")
            .code,
            "query_parent_forbidden"
        );
        assert_eq!(
            parse_query_arguments(&[
                "owners".to_owned(),
                "--bytes".to_owned(),
                (MINIMUM_QUERY_OUTPUT_BYTES - 1).to_string(),
            ])
            .expect_err("undersized fixed query envelope")
            .code,
            "query_invalid_byte_limit"
        );
    }

    #[test]
    fn continuation_is_canonical_revision_and_selector_bound() {
        let repository = RepositoryId::migrate(b"query-continuation", 1);
        let package = PackageId::migrate(b"query-continuation", 2);
        let revision = RevisionId::from_digest([3; 32]);
        let owner = OwnerKey::Module(crate::platform::semantic_id::ModuleId::migrate(
            b"query-continuation",
            4,
        ));
        let request = NormalizedQueryRequest {
            selection: QuerySelection::Owners { kind: None },
            limits: QueryPageLimits {
                items: 1,
                output_bytes: 4096,
            },
            continuation: None,
        };
        let binding = QueryBinding {
            repository,
            package,
            revision,
        };
        let selector = query_digest(&request.selection).expect("query digest");
        let token = encode_continuation(
            binding,
            QueryOperation::Owners,
            selector,
            &EncodedOwnerKey::new(owner).bytes(),
        )
        .expect("canonical continuation");
        assert!(token.len() <= MAXIMUM_QUERY_CONTINUATION_BYTES);
        let mut resumed = request.clone();
        resumed.continuation = Some(token.clone());
        resumed.limits.items = 99;
        resumed.limits.output_bytes = 8192;
        assert_eq!(
            bind_continuation(&resumed, binding, selector)
                .expect("limit-independent binding")
                .expect("resume key"),
            EncodedOwnerKey::new(owner).bytes()
        );

        let mut changed = resumed.clone();
        changed.selection = QuerySelection::Owners {
            kind: Some(OwnerKind::Module),
        };
        assert_eq!(
            bind_continuation(
                &changed,
                binding,
                query_digest(&changed.selection).expect("changed selector"),
            )
            .expect_err("changed selector")
            .code,
            "query_continuation_mismatch"
        );
        assert_eq!(
            bind_continuation(
                &resumed,
                QueryBinding {
                    revision: RevisionId::from_digest([9; 32]),
                    ..binding
                },
                selector,
            )
            .expect_err("stale revision")
            .code,
            "query_continuation_stale"
        );
        let mut mutated = token.clone().into_bytes();
        let index = mutated.len() - 2;
        mutated[index] = if mutated[index] == b'A' { b'B' } else { b'A' };
        let mutated = String::from_utf8(mutated).expect("ASCII continuation");
        assert!(matches!(
            decode_continuation(&mutated)
                .expect_err("mutated continuation")
                .code
                .as_str(),
            "query_continuation_integrity" | "query_continuation_malformed"
        ));
        assert_eq!(
            decode_continuation("cont_predecessor")
                .expect_err("predecessor token")
                .code,
            "predecessor_contract"
        );

        assert_eq!(
            decode_continuation(&format!("{token}="))
                .expect_err("padded continuation")
                .code,
            "query_continuation_noncanonical"
        );
        assert_eq!(
            decode_continuation(&format!(
                "{QUERY_CONTINUATION_PREFIX}{}",
                "A".repeat(MAXIMUM_QUERY_CONTINUATION_BYTES)
            ))
            .expect_err("oversized continuation")
            .code,
            "query_continuation_oversized"
        );
        let raw = raw_continuation(&token);
        let mut query_three = raw[..raw.len() - CONTINUATION_CHECKSUM_BYTES].to_vec();
        query_three[..8].copy_from_slice(b"LKJQCT03");
        assert_eq!(
            decode_continuation(&sealed_continuation(query_three))
                .expect_err("query-3 continuation")
                .code,
            "query_continuation_contract"
        );
        let truncated = format!(
            "{QUERY_CONTINUATION_PREFIX}{}",
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&raw[..10])
        );
        assert_eq!(
            decode_continuation(&truncated)
                .expect_err("truncated continuation")
                .code,
            "query_continuation_malformed"
        );

        let mutate_and_seal = |offset: usize, replacement: &[u8]| {
            let mut candidate = raw[..raw.len() - CONTINUATION_CHECKSUM_BYTES].to_vec();
            candidate[offset..offset + replacement.len()].copy_from_slice(replacement);
            sealed_continuation(candidate)
        };
        assert_eq!(
            decode_continuation(&mutate_and_seal(18, &2_u16.to_be_bytes()))
                .expect_err("foreign continuation payload version")
                .code,
            "query_continuation_contract"
        );
        assert_eq!(
            decode_continuation(&mutate_and_seal(20, &2_u16.to_be_bytes()))
                .expect_err("foreign query version")
                .code,
            "query_continuation_contract"
        );
        assert_eq!(
            decode_continuation(&mutate_and_seal(22, &[0; 16]))
                .expect_err("reserved repository identity")
                .code,
            "query_continuation_reserved_identity"
        );
        assert_eq!(
            decode_continuation(&mutate_and_seal(86, &[u8::MAX]))
                .expect_err("foreign query operation")
                .code,
            "query_continuation_operation"
        );
        assert_eq!(
            decode_continuation(&mutate_and_seal(121, &[0]))
                .and_then(|decoded| {
                    validate_resume_key(&request.selection, binding.package, &decoded.resume_key)
                })
                .expect_err("invalid logical resume key")
                .code,
            "query_continuation_resume_key"
        );

        let mut trailing = raw[..raw.len() - CONTINUATION_CHECKSUM_BYTES].to_vec();
        let payload_length = u64::from_le_bytes(
            trailing[10..18]
                .try_into()
                .expect("continuation payload length"),
        ) + 1;
        trailing[10..18].copy_from_slice(&payload_length.to_le_bytes());
        trailing.push(0);
        assert_eq!(
            decode_continuation(&sealed_continuation(trailing))
                .expect_err("continuation trailing payload")
                .code,
            "query_continuation_trailing"
        );
        assert_ne!(
            domain_digest(QUERY_SELECTOR_DIGEST_DOMAIN, b"same"),
            domain_digest(QUERY_CONTINUATION_INTEGRITY_DOMAIN, b"same")
        );
    }

    #[test]
    fn normalized_owner_find_and_relation_pages_match_full_oracles_and_write_nothing() {
        let (_temporary, repository, snapshot) = fixture();
        let before = file_inventory(repository.root());
        let view = repository.view_current().expect("query view");

        let mut owner_request = NormalizedQueryRequest {
            selection: QuerySelection::Owners { kind: None },
            limits: QueryPageLimits {
                items: 3,
                output_bytes: DEFAULT_QUERY_OUTPUT_BYTES,
            },
            continuation: None,
        };
        let mut observed_owners = Vec::new();
        let mut page = 0_u64;
        loop {
            owner_request.limits.items = if page.is_multiple_of(2) { 3 } else { 5 };
            let bytes = execute_normalized_query(&repository, &view, &owner_request)
                .expect("owner query page");
            let page_records = records(&bytes);
            let rendered = page_records
                .iter()
                .find(|record| record.operation == "work")
                .and_then(|record| field(record, "rendered-output-bytes"))
                .expect("rendered byte work")
                .parse::<usize>()
                .expect("rendered byte count");
            assert_eq!(rendered, bytes.len());
            observed_owners.extend(
                page_records
                    .iter()
                    .filter(|record| record.operation == "owner")
                    .map(|record| {
                        field(record, "id")
                            .expect("owner id")
                            .parse::<OwnerKey>()
                            .expect("typed owner")
                    }),
            );
            owner_request.continuation = page_records
                .iter()
                .find(|record| record.operation == "continuation")
                .and_then(|record| field(record, "token"))
                .map(str::to_owned);
            page += 1;
            if owner_request.continuation.is_none() {
                break;
            }
            assert!(page < 100, "owner pagination must terminate");
        }
        let mut expected_owners = snapshot.owners.keys().copied().collect::<Vec<_>>();
        expected_owners.sort_by_key(|owner| EncodedOwnerKey::new(*owner).bytes());
        assert_eq!(observed_owners, expected_owners);

        let first_module = named_owner(&snapshot, "first");
        let find_module = NormalizedQueryRequest {
            selection: QuerySelection::Find {
                class: NamespaceClass::Module,
                name: Name::new("first").expect("module name"),
                parent: None,
            },
            limits: QueryPageLimits {
                items: 1,
                output_bytes: DEFAULT_QUERY_OUTPUT_BYTES,
            },
            continuation: None,
        };
        let found = records(
            &execute_normalized_query(&repository, &view, &find_module).expect("exact module find"),
        );
        assert_eq!(
            found
                .iter()
                .find(|record| record.operation == "owner")
                .and_then(|record| field(record, "id")),
            Some(first_module.to_string().as_str())
        );

        let payload = named_owner(&snapshot, "Payload");
        let find_payload = NormalizedQueryRequest {
            selection: QuerySelection::Find {
                class: NamespaceClass::Declaration,
                name: Name::new("Payload").expect("payload name"),
                parent: Some(first_module),
            },
            ..find_module.clone()
        };
        let found = records(
            &execute_normalized_query(&repository, &view, &find_payload)
                .expect("nested declaration find"),
        );
        assert_eq!(
            found
                .iter()
                .find(|record| record.operation == "owner")
                .and_then(|record| field(record, "id")),
            Some(payload.to_string().as_str())
        );
        let missing_payload = NormalizedQueryRequest {
            selection: QuerySelection::Find {
                class: NamespaceClass::Declaration,
                name: Name::new("Missing").expect("missing lookup name"),
                parent: Some(first_module),
            },
            ..find_module.clone()
        };
        let missing = records(
            &execute_normalized_query(&repository, &view, &missing_payload)
                .expect("exact namespace no-match"),
        );
        assert_eq!(
            missing
                .iter()
                .find(|record| record.operation == "summary")
                .and_then(|record| field(record, "match")),
            Some("false")
        );
        assert_eq!(
            exact_namespace_projection(
                payload,
                &snapshot.owners[&payload],
                NamespaceClass::Declaration,
                &Name::new("ForeignName").expect("foreign namespace name"),
                Some(first_module),
            )
            .expect_err("namespace witness/canonical disagreement")
            .code,
            "query_namespace_owner_disagreement"
        );

        let caller = named_owner(&snapshot, "caller");
        let exact_caller = RelationEndpoint::Owner(ExactOwnerKey {
            package: snapshot.root.package_id,
            owner: caller,
        });
        let mut expected_relations = extract_relations(
            snapshot.root.package_id,
            &snapshot.owners,
            &snapshot.types,
            &snapshot.dependencies,
        )
        .expect("full relation oracle")
        .into_iter()
        .filter(|edge| edge.source == exact_caller)
        .collect::<Vec<_>>();
        expected_relations.sort_by_key(|edge| forward_relation_key(*edge));
        let mut relation_request = NormalizedQueryRequest {
            selection: QuerySelection::Relations {
                endpoint: QueryEndpointSelector::Owner(caller),
                direction: QueryDirection::Outgoing,
                kind: None,
            },
            limits: QueryPageLimits {
                items: 2,
                output_bytes: DEFAULT_QUERY_OUTPUT_BYTES,
            },
            continuation: None,
        };
        let mut observed_relations = Vec::new();
        loop {
            let page_records = records(
                &execute_normalized_query(&repository, &view, &relation_request)
                    .expect("relation page"),
            );
            observed_relations.extend(
                page_records
                    .iter()
                    .filter(|record| record.operation == "relation")
                    .map(relation_from_record),
            );
            relation_request.continuation = page_records
                .iter()
                .find(|record| record.operation == "continuation")
                .and_then(|record| field(record, "token"))
                .map(str::to_owned);
            if relation_request.continuation.is_none() {
                break;
            }
            relation_request.limits.items = 3;
        }
        assert_eq!(observed_relations, expected_relations);

        let callee = named_owner(&snapshot, "callee");
        let exact_callee = RelationEndpoint::Owner(ExactOwnerKey {
            package: snapshot.root.package_id,
            owner: callee,
        });
        let full_relations = extract_relations(
            snapshot.root.package_id,
            &snapshot.owners,
            &snapshot.types,
            &snapshot.dependencies,
        )
        .expect("full relation oracle for incoming and package queries");
        let mut expected_incoming = full_relations
            .iter()
            .copied()
            .filter(|edge| edge.target == exact_callee && edge.kind == RelationKind::FunctionCall)
            .collect::<Vec<_>>();
        expected_incoming.sort_by_key(|edge| reverse_relation_key(*edge));
        let incoming = records(
            &execute_normalized_query(
                &repository,
                &view,
                &NormalizedQueryRequest {
                    selection: QuerySelection::Relations {
                        endpoint: QueryEndpointSelector::Owner(callee),
                        direction: QueryDirection::Incoming,
                        kind: Some(RelationKind::FunctionCall),
                    },
                    limits: QueryPageLimits {
                        items: 100,
                        output_bytes: DEFAULT_QUERY_OUTPUT_BYTES,
                    },
                    continuation: None,
                },
            )
            .expect("filtered incoming relation query"),
        )
        .iter()
        .filter(|record| record.operation == "relation")
        .map(relation_from_record)
        .collect::<Vec<_>>();
        assert_eq!(incoming, expected_incoming);

        for direction in QueryDirection::ALL {
            let package_endpoint = RelationEndpoint::Package(snapshot.root.package_id);
            let mut expected = full_relations
                .iter()
                .copied()
                .filter(|edge| match direction {
                    QueryDirection::Incoming => edge.target == package_endpoint,
                    QueryDirection::Outgoing => edge.source == package_endpoint,
                })
                .collect::<Vec<_>>();
            match direction {
                QueryDirection::Incoming => {
                    expected.sort_by_key(|edge| reverse_relation_key(*edge));
                }
                QueryDirection::Outgoing => {
                    expected.sort_by_key(|edge| forward_relation_key(*edge));
                }
            }
            let observed = records(
                &execute_normalized_query(
                    &repository,
                    &view,
                    &NormalizedQueryRequest {
                        selection: QuerySelection::Relations {
                            endpoint: QueryEndpointSelector::Package,
                            direction,
                            kind: None,
                        },
                        limits: QueryPageLimits {
                            items: 100,
                            output_bytes: DEFAULT_QUERY_OUTPUT_BYTES,
                        },
                        continuation: None,
                    },
                )
                .expect("current package relation query"),
            )
            .iter()
            .filter(|record| record.operation == "relation")
            .map(relation_from_record)
            .collect::<Vec<_>>();
            assert_eq!(observed, expected, "package direction {direction:?}");
        }

        let absent_owner = OwnerKey::Module(ModuleId::migrate(b"normalized-query-absent", 1));
        assert!(!snapshot.owners.contains_key(&absent_owner));
        assert_eq!(
            execute_normalized_query(
                &repository,
                &view,
                &NormalizedQueryRequest {
                    selection: QuerySelection::Relations {
                        endpoint: QueryEndpointSelector::Owner(absent_owner),
                        direction: QueryDirection::Outgoing,
                        kind: None,
                    },
                    limits: QueryPageLimits {
                        items: 1,
                        output_bytes: DEFAULT_QUERY_OUTPUT_BYTES,
                    },
                    continuation: None,
                },
            )
            .expect_err("nonexistent relation owner")
            .code,
            "query_owner_not_found"
        );
        assert_eq!(file_inventory(repository.root()), before);
        assert_eq!(
            GraphRepository::open(repository.root())
                .expect("repository after query")
                .view_current()
                .expect("view after query")
                .revision(),
            view.revision()
        );
    }

    #[test]
    fn bounded_context_pages_match_complete_oracle_and_contain_failures_without_writes() {
        let (_temporary, repository, snapshot) = fixture();
        let view = repository.view_current().expect("context query view");
        let before = file_inventory(repository.root());
        let root = named_owner(&snapshot, "caller");

        for (direction, depth) in [
            (ContextDirection::Incoming, 1),
            (ContextDirection::Outgoing, 2),
            (ContextDirection::Both, 3),
        ] {
            let expected = full_context_oracle(&snapshot, root, direction, depth);
            let (observed, pages) =
                collect_context_pages(&repository, &view, root, direction, depth);
            assert_eq!(observed, expected, "{direction:?} depth {depth}");
            assert!(pages.len() > 1, "continuation must be exercised");
            assert_eq!(
                observed
                    .relations
                    .iter()
                    .copied()
                    .collect::<BTreeSet<_>>()
                    .len(),
                observed.relations.len(),
                "duplicate witness observations must emit one relation"
            );
            assert!(observed.owners.windows(2).all(|pair| {
                (pair[0].1, EncodedOwnerKey::new(pair[0].0).bytes())
                    < (pair[1].1, EncodedOwnerKey::new(pair[1].0).bytes())
            }));
            assert!(
                observed
                    .relations
                    .windows(2)
                    .all(|pair| { forward_relation_key(pair[0]) < forward_relation_key(pair[1]) })
            );
        }

        let selection = QuerySelection::Context {
            root,
            direction: ContextDirection::Both,
            depth: 3,
        };
        let selector = query_digest(&selection).expect("context selector");
        let binding = QueryBinding {
            repository: view.current().head.repository_id,
            package: view.package(),
            revision: view.revision(),
        };
        let first_request = NormalizedQueryRequest {
            selection: selection.clone(),
            limits: QueryPageLimits {
                items: 1,
                output_bytes: MINIMUM_QUERY_OUTPUT_BYTES,
            },
            continuation: None,
        };
        let first = records(
            &execute_normalized_query(&repository, &view, &first_request)
                .expect("first context page"),
        );
        let token = field(
            first
                .iter()
                .find(|record| record.operation == "continuation")
                .expect("context continuation"),
            "token",
        )
        .expect("context continuation token")
        .to_owned();
        for changed in [
            QuerySelection::Context {
                root,
                direction: ContextDirection::Incoming,
                depth: 3,
            },
            QuerySelection::Context {
                root,
                direction: ContextDirection::Both,
                depth: 2,
            },
            QuerySelection::Context {
                root: named_owner(&snapshot, "callee"),
                direction: ContextDirection::Both,
                depth: 3,
            },
        ] {
            let error = execute_normalized_query(
                &repository,
                &view,
                &NormalizedQueryRequest {
                    selection: changed,
                    limits: first_request.limits,
                    continuation: Some(token.clone()),
                },
            )
            .expect_err("selector-mismatched context continuation");
            assert_eq!(error.code, "query_continuation_mismatch");
        }

        let oracle = full_context_oracle(&snapshot, root, ContextDirection::Both, 3);
        let terminal_key = oracle
            .relations
            .last()
            .copied()
            .map(context_relation_resume_key)
            .unwrap_or_else(|| {
                let (owner, depth) = oracle.owners.last().expect("terminal context owner");
                context_owner_resume_key(*depth, *owner)
            });
        let terminal_token =
            encode_continuation(binding, QueryOperation::Context, selector, &terminal_key)
                .expect("syntactic terminal context token");
        let terminal_error = execute_normalized_query(
            &repository,
            &view,
            &NormalizedQueryRequest {
                selection: selection.clone(),
                limits: first_request.limits,
                continuation: Some(terminal_token),
            },
        )
        .expect_err("logically terminal context token");
        assert_eq!(terminal_error.code, "query_continuation_resume_key");

        let absent = OwnerKey::Module(ModuleId::migrate(b"absent-context-root", 1));
        let missing = execute_normalized_query(
            &repository,
            &view,
            &NormalizedQueryRequest {
                selection: QuerySelection::Context {
                    root: absent,
                    direction: ContextDirection::Outgoing,
                    depth: 1,
                },
                limits: first_request.limits,
                continuation: None,
            },
        )
        .expect_err("missing context root");
        assert_eq!(missing.class, DiagnosticClass::Semantic);
        assert_eq!(missing.code, "query_owner_not_found");

        let control = ExecutionControl::uncancelled();
        let mut checks = 0_u64;
        let mut cancellation = || {
            checks += 1;
            if checks == 8 {
                control.cancel();
            }
            check_query_control(&control)
        };
        let cancelled = execute_normalized_query_with_cancellation(
            &repository,
            &view,
            &NormalizedQueryRequest {
                selection,
                limits: QueryPageLimits {
                    items: 7,
                    output_bytes: 4_096,
                },
                continuation: None,
            },
            &mut cancellation,
        )
        .expect_err("injected context cancellation");
        assert_eq!(cancelled.class, DiagnosticClass::Cancelled);
        assert_eq!(cancelled.code, "query_cancelled");

        for (maximum, code, dimension) in [
            (
                MAXIMUM_CONTEXT_OWNERS,
                "query_context_owner_limit",
                "unique local owners",
            ),
            (
                MAXIMUM_CONTEXT_RELATIONS,
                "query_context_relation_limit",
                "unique selected relations",
            ),
        ] {
            assert!(
                ensure_context_capacity(
                    usize::try_from(maximum - 1).expect("exact context capacity"),
                    maximum,
                    code,
                    dimension,
                )
                .is_ok()
            );
            assert_eq!(
                ensure_context_capacity(
                    usize::try_from(maximum).expect("one-over context capacity"),
                    maximum,
                    code,
                    dimension,
                )
                .expect_err("one-over context capacity")
                .code,
                code
            );
        }
        assert_eq!(
            checked_context_count(
                MAXIMUM_CONTEXT_RELATION_WITNESSES - 1,
                MAXIMUM_CONTEXT_RELATION_WITNESSES,
                "query_context_witness_limit",
                "relation witnesses",
            )
            .expect("exact witness fit"),
            MAXIMUM_CONTEXT_RELATION_WITNESSES
        );
        assert_eq!(
            checked_context_count(
                MAXIMUM_CONTEXT_RELATION_WITNESSES,
                MAXIMUM_CONTEXT_RELATION_WITNESSES,
                "query_context_witness_limit",
                "relation witnesses",
            )
            .expect_err("one-over witness admission")
            .code,
            "query_context_witness_limit"
        );
        assert_eq!(file_inventory(repository.root()), before);
    }

    #[test]
    fn context_keeps_an_isolated_root_and_observes_cycles_diamonds_and_duplicate_edges_once() {
        let mut isolated = crate::platform::kernel::tests::witness_snapshot();
        let root = named_owner(&isolated, "first");
        let root_record = isolated
            .owners
            .get(&root)
            .expect("isolated root record")
            .clone();
        isolated.owners = BTreeMap::from([(root, root_record)]);
        isolated.types.clear();
        isolated.dependencies.clear();
        isolated.dependency_interfaces.clear();
        isolated.dependency_types.clear();
        isolated.blobs.clear();
        isolated.retirements.clear();
        let owner_root = isolated.root.owners;
        isolated.root.owners = super::super::persistent_map::MapRoot::from_parts(
            owner_root.page(),
            1,
            owner_root.content(),
        );
        let temporary = tempfile::tempdir().expect("isolated context parent");
        let created = GraphRepository::create(&temporary.path().join("project"), &isolated, None)
            .expect("isolated context repository");
        let view = created
            .repository
            .view_current()
            .expect("isolated context view");
        let before = file_inventory(created.repository.root());
        let (observed, pages) = collect_context_pages(
            &created.repository,
            &view,
            root,
            ContextDirection::Both,
            MAXIMUM_CONTEXT_DEPTH,
        );
        assert_eq!(observed.owners, vec![(root, 0)]);
        assert!(observed.relations.is_empty());
        assert_eq!(observed.expanded_owners, 1);
        assert_eq!(observed.relation_witnesses, 0);
        assert_eq!(pages.len(), 1);
        assert_eq!(file_inventory(created.repository.root()), before);

        let (_fixture, repository, snapshot) = fixture();
        let root = named_owner(&snapshot, "caller");
        let expected = full_context_oracle(&snapshot, root, ContextDirection::Both, 4);
        let (observed, _pages) = collect_context_pages(
            &repository,
            &repository.view_current().expect("adversarial context view"),
            root,
            ContextDirection::Both,
            4,
        );
        assert_eq!(observed, expected);
        assert!(
            observed.relation_witnesses > observed.relations.len() as u64,
            "both-direction traversal must deduplicate edges observed from both endpoints"
        );
        let distances = observed.owners.iter().copied().collect::<BTreeMap<_, _>>();
        let package = snapshot.root.package_id;
        let has_cycle = observed.relations.iter().any(|edge| {
            matches!(
                (edge.source, edge.target),
                (RelationEndpoint::Owner(source), RelationEndpoint::Owner(target))
                    if source.package == package
                        && target.package == package
                        && distances.contains_key(&source.owner)
                        && distances.contains_key(&target.owner)
            )
        });
        assert!(
            has_cycle,
            "both-direction local edges form revisited cycles"
        );
        let has_equal_path_diamond = observed.owners.iter().any(|(candidate, depth)| {
            if *depth < 2 {
                return false;
            }
            let predecessor_depth = depth - 1;
            observed
                .relations
                .iter()
                .filter_map(|edge| match (edge.source, edge.target) {
                    (RelationEndpoint::Owner(source), RelationEndpoint::Owner(target))
                        if source.package == package
                            && target.package == package
                            && source.owner == *candidate =>
                    {
                        Some(target.owner)
                    }
                    (RelationEndpoint::Owner(source), RelationEndpoint::Owner(target))
                        if source.package == package
                            && target.package == package
                            && target.owner == *candidate =>
                    {
                        Some(source.owner)
                    }
                    _ => None,
                })
                .filter(|predecessor| distances.get(predecessor) == Some(&predecessor_depth))
                .collect::<BTreeSet<_>>()
                .len()
                > 1
        });
        assert!(
            has_equal_path_diamond,
            "fixture must retain an equal-length diamond"
        );

        let self_owner = ExactOwnerKey {
            package,
            owner: root,
        };
        let self_edge = RelationEdge {
            source: RelationEndpoint::Owner(self_owner),
            kind: RelationKind::FunctionCall,
            target: RelationEndpoint::Owner(self_owner),
        };
        let mut selected = BTreeMap::new();
        for _ in 0..2 {
            let key = forward_relation_key(self_edge);
            if !selected.contains_key(&key) {
                ensure_context_capacity(
                    selected.len(),
                    MAXIMUM_CONTEXT_RELATIONS,
                    "query_context_relation_limit",
                    "unique selected relations",
                )
                .expect("self-edge capacity");
            }
            selected.insert(key, self_edge);
        }
        assert_eq!(selected.into_values().collect::<Vec<_>>(), vec![self_edge]);
    }

    #[test]
    fn normalized_query_reports_selected_canonical_object_corruption_without_oracle_rebuild() {
        let (_temporary, repository, snapshot) = fixture();
        let view = repository.view_current().expect("corruption query view");
        let callee = named_owner(&snapshot, "callee");
        let module = match &snapshot.owners[&callee] {
            OwnerRecord::Declaration(declaration) => OwnerKey::Module(declaration.module),
            _ => panic!("callee must be a declaration"),
        };
        let digest = encode_owner(&snapshot.owners[&callee])
            .expect("callee owner encoding")
            .0;
        let key = ObjectKey::from_digest(ObjectDomain::Owner, digest.bytes());
        corrupt_repository_object(&repository, key);
        let after_corruption = file_inventory(repository.root());

        let context_error = execute_normalized_query(
            &repository,
            &view,
            &NormalizedQueryRequest {
                selection: QuerySelection::Context {
                    root: callee,
                    direction: ContextDirection::Both,
                    depth: 1,
                },
                limits: QueryPageLimits {
                    items: 1,
                    output_bytes: DEFAULT_QUERY_OUTPUT_BYTES,
                },
                continuation: None,
            },
        )
        .expect_err("context must reject its corrupt canonical root owner");
        assert_eq!(context_error.class, DiagnosticClass::Corrupt);
        assert_eq!(file_inventory(repository.root()), after_corruption);

        let error = execute_normalized_query(
            &repository,
            &view,
            &NormalizedQueryRequest {
                selection: QuerySelection::Find {
                    class: NamespaceClass::Declaration,
                    name: Name::new("callee").expect("callee query name"),
                    parent: Some(module),
                },
                limits: QueryPageLimits {
                    items: 1,
                    output_bytes: DEFAULT_QUERY_OUTPUT_BYTES,
                },
                continuation: None,
            },
        )
        .expect_err("selected corrupt canonical owner must reject");
        assert_eq!(error.class, DiagnosticClass::Corrupt);
        assert_eq!(file_inventory(repository.root()), after_corruption);
    }

    #[test]
    fn context_rejects_corrupt_owner_and_relation_map_pages_without_writes() {
        for relation_map in [false, true] {
            let (_temporary, repository, snapshot) = fixture();
            let view = repository
                .view_current()
                .expect("context map corruption view");
            let root = named_owner(&snapshot, "caller");
            let page = if relation_map {
                view.current().witness.roots.forward_relations.page()
            } else {
                view.current().semantic_root.owners.page()
            };
            corrupt_repository_object(
                &repository,
                ObjectKey::from_digest(ObjectDomain::MapPage, page.bytes()),
            );
            let after_corruption = file_inventory(repository.root());
            let error = execute_normalized_query(
                &repository,
                &view,
                &NormalizedQueryRequest {
                    selection: QuerySelection::Context {
                        root,
                        direction: ContextDirection::Outgoing,
                        depth: 1,
                    },
                    limits: QueryPageLimits {
                        items: 1,
                        output_bytes: DEFAULT_QUERY_OUTPUT_BYTES,
                    },
                    continuation: None,
                },
            )
            .expect_err("context must reject a corrupt selected map page");
            assert_eq!(error.class, DiagnosticClass::Corrupt);
            assert_eq!(file_inventory(repository.root()), after_corruption);
        }
    }

    #[test]
    fn context_rejects_missing_local_endpoints_and_revision_binding_without_writes() {
        let (_temporary, repository, snapshot) = fixture();
        let view = repository.view_current().expect("context binding view");
        let before = file_inventory(repository.root());
        let binding = QueryBinding {
            repository: view.current().head.repository_id,
            package: view.package(),
            revision: view.revision(),
        };
        let absent = OwnerKey::Module(ModuleId::migrate(b"missing-context-neighbor", 1));
        assert!(!snapshot.owners.contains_key(&absent));
        let mut work = RepositoryReadWork::default();
        let missing = read_context_owner(
            &view,
            absent,
            binding,
            context_query_admission(),
            &mut work,
            false,
        )
        .expect_err("missing selected local endpoint must reject");
        assert_eq!(missing.class, DiagnosticClass::Corrupt);
        assert_eq!(missing.code, "query_context_owner_missing");

        let root = named_owner(&snapshot, "caller");
        let mut work = RepositoryReadWork::default();
        let mismatched = read_context_owner(
            &view,
            root,
            QueryBinding {
                revision: RevisionId::from_digest([0x5a; 32]),
                ..binding
            },
            context_query_admission(),
            &mut work,
            true,
        )
        .expect_err("mismatched pinned revision must reject");
        assert_eq!(mismatched.class, DiagnosticClass::Corrupt);
        assert_eq!(mismatched.code, "query_revision_binding");
        assert_eq!(file_inventory(repository.root()), before);
    }

    #[test]
    fn normalized_query_admits_map_store_and_decode_dimensions_independently() {
        let (_temporary, repository, snapshot) = fixture();
        let view = repository.view_current().expect("admission query view");
        let baseline = query_admission(4, 4).expect("query admission baseline");
        for (name, admission, expected) in [
            (
                "map-pages",
                RepositoryQueryAdmission {
                    map_pages: 0,
                    ..baseline
                },
                "persistent_map_admission_pages_read",
            ),
            (
                "map-bytes",
                RepositoryQueryAdmission {
                    map_bytes: 0,
                    ..baseline
                },
                "persistent_map_admission_bytes_read",
            ),
            (
                "map-entries",
                RepositoryQueryAdmission {
                    map_entries: 0,
                    ..baseline
                },
                "persistent_map_admission_entries_visited",
            ),
            (
                "catalog-lookups",
                RepositoryQueryAdmission {
                    catalog_lookups: 0,
                    ..baseline
                },
                "object_read_catalog_lookups_exhausted",
            ),
            (
                "store-objects",
                RepositoryQueryAdmission {
                    store_objects: 0,
                    ..baseline
                },
                "object_read_objects_exhausted",
            ),
            (
                "store-bytes",
                RepositoryQueryAdmission {
                    store_bytes: 0,
                    ..baseline
                },
                "object_read_bytes_exhausted",
            ),
            (
                "canonical-records",
                RepositoryQueryAdmission {
                    canonical_records: 0,
                    ..baseline
                },
                "query_admission_canonical_records",
            ),
        ] {
            let error = view
                .visit_query_owners(None, None, 4, 4, admission, |_owner, _record| {
                    Ok(MapRangeControl::Continue)
                })
                .expect_err(name);
            assert_eq!(error.class, DiagnosticClass::Resource, "{name}");
            assert_eq!(error.code, expected, "{name}");
        }

        let caller = named_owner(&snapshot, "caller");
        let error = view
            .visit_query_relations(
                RepositoryRelationQueryRange {
                    endpoint: RelationEndpoint::Owner(ExactOwnerKey {
                        package: snapshot.root.package_id,
                        owner: caller,
                    }),
                    kind: None,
                    incoming: false,
                    exclusive_lower_bound: None,
                    maximum_scan: 4,
                    maximum_items: 4,
                },
                RepositoryQueryAdmission {
                    witness_records: 0,
                    ..baseline
                },
                |_edge| Ok(MapRangeControl::Continue),
            )
            .expect_err("witness record admission");
        assert_eq!(error.class, DiagnosticClass::Resource);
        assert_eq!(error.code, "query_admission_witness_records");
    }

    #[test]
    fn context_exact_owner_maximum_fits_and_one_owner_over_fails_atomically() {
        for (neighbors, expected_error) in [
            (MAXIMUM_CONTEXT_OWNERS as usize - 1, None),
            (
                MAXIMUM_CONTEXT_OWNERS as usize,
                Some("query_context_owner_limit"),
            ),
        ] {
            let temporary = tempfile::tempdir().expect("owner-limit context parent");
            let destination = temporary.path().join("project");
            let (snapshot, root) = owner_limit_context_snapshot(neighbors);
            let created = GraphRepository::create(&destination, &snapshot, None)
                .expect("owner-limit context repository");
            let view = created
                .repository
                .view_current()
                .expect("owner-limit context view");
            let before = file_inventory(created.repository.root());
            let request = NormalizedQueryRequest {
                selection: QuerySelection::Context {
                    root,
                    direction: ContextDirection::Incoming,
                    depth: 1,
                },
                limits: QueryPageLimits {
                    items: 1,
                    output_bytes: 4_096,
                },
                continuation: None,
            };
            match expected_error {
                None => {
                    let bytes = execute_normalized_query(&created.repository, &view, &request)
                        .expect("exact owner-limit context traversal");
                    let output = records(&bytes);
                    let summary = output
                        .iter()
                        .find(|record| record.operation == "summary")
                        .expect("owner-limit context summary");
                    assert_eq!(
                        field(summary, "total-owners"),
                        Some(MAXIMUM_CONTEXT_OWNERS.to_string().as_str())
                    );
                    assert_eq!(
                        field(summary, "total-relations"),
                        Some((MAXIMUM_CONTEXT_OWNERS - 1).to_string().as_str())
                    );
                }
                Some(code) => {
                    let error = execute_normalized_query(&created.repository, &view, &request)
                        .expect_err("one-over context owner maximum");
                    assert_eq!(error.class, DiagnosticClass::Resource);
                    assert_eq!(error.code, code);
                }
            }
            assert_eq!(file_inventory(created.repository.root()), before);
        }
    }

    #[test]
    fn context_exact_logical_maxima_fit_and_one_relation_over_fails_atomically() {
        for (named_relations, expected_error) in [
            (12_289_usize, None),
            (12_290_usize, Some("query_context_relation_limit")),
        ] {
            let temporary = tempfile::tempdir().expect("maximum context parent");
            let destination = temporary.path().join("project");
            let (snapshot, root) = maximum_context_snapshot(named_relations);
            let created = GraphRepository::create(&destination, &snapshot, None)
                .expect("maximum context repository");
            let view = created
                .repository
                .view_current()
                .expect("maximum context view");
            let before = file_inventory(created.repository.root());
            let request = NormalizedQueryRequest {
                selection: QuerySelection::Context {
                    root,
                    direction: ContextDirection::Both,
                    depth: 3,
                },
                limits: QueryPageLimits {
                    items: 1,
                    output_bytes: 4_096,
                },
                continuation: None,
            };
            let started = Instant::now();
            match expected_error {
                None => {
                    let bytes = execute_normalized_query(&created.repository, &view, &request)
                        .expect("exact maximum context traversal");
                    let output = records(&bytes);
                    let summary = output
                        .iter()
                        .find(|record| record.operation == "summary")
                        .expect("maximum context summary");
                    let work = output
                        .iter()
                        .find(|record| record.operation == "work")
                        .expect("maximum context work");
                    assert_eq!(
                        field(summary, "total-owners"),
                        Some(MAXIMUM_CONTEXT_OWNERS.to_string().as_str())
                    );
                    assert_eq!(
                        field(summary, "total-relations"),
                        Some(MAXIMUM_CONTEXT_RELATIONS.to_string().as_str())
                    );
                    assert_eq!(
                        field(summary, "expanded-owners"),
                        Some(MAXIMUM_CONTEXT_OWNERS.to_string().as_str())
                    );
                    assert_eq!(
                        field(work, "relation-witnesses-visited"),
                        Some(MAXIMUM_CONTEXT_RELATION_WITNESSES.to_string().as_str())
                    );
                    assert_eq!(
                        field(work, "canonical-records-decoded"),
                        Some(MAXIMUM_CONTEXT_OWNERS.to_string().as_str())
                    );
                    assert_eq!(
                        field(work, "witness-records-decoded"),
                        Some(MAXIMUM_CONTEXT_RELATION_WITNESSES.to_string().as_str())
                    );
                }
                Some(code) => {
                    let error = execute_normalized_query(&created.repository, &view, &request)
                        .expect_err("one-over maximum context traversal");
                    assert_eq!(error.class, DiagnosticClass::Resource);
                    assert_eq!(error.code, code);
                }
            }
            assert_eq!(file_inventory(created.repository.root()), before);
            println!(
                "context-maximum named-relations={named_relations} expected-error={} wall-micros={} peak-rss-kib={}",
                expected_error.unwrap_or("none"),
                started.elapsed().as_micros(),
                process_peak_rss_kib()
                    .map_or_else(|| "unavailable".to_owned(), |value| value.to_string()),
            );
        }
    }

    #[test]
    fn context_physical_admissions_fit_exact_observed_work_and_reject_one_over() {
        let (_temporary, repository, snapshot) = fixture();
        let view = repository.view_current().expect("context admission view");
        let root = named_owner(&snapshot, "caller");
        let selection = QuerySelection::Context {
            root,
            direction: ContextDirection::Both,
            depth: 3,
        };
        let request = NormalizedQueryRequest {
            selection: selection.clone(),
            limits: QueryPageLimits {
                items: 7,
                output_bytes: 4_096,
            },
            continuation: None,
        };
        let binding = QueryBinding {
            repository: view.current().head.repository_id,
            package: view.package(),
            revision: view.revision(),
        };
        let capabilities = capabilities_snapshot().expect("context admission capabilities");
        let render_context = QueryRenderContext {
            repository_root: repository.root(),
            project_name: view.current().semantic_root.package_name.as_str(),
            repository: binding.repository,
            package: binding.package,
            revision: binding.revision,
            capabilities_digest: &capabilities.digest,
        };
        let selector = query_digest(&selection).expect("context admission selector");
        let mut uncancelled = || Ok(());
        let baseline = execute_context(
            &view,
            context_query_admission(),
            &request,
            &render_context,
            binding,
            selector,
            None,
            root,
            ContextDirection::Both,
            3,
            &mut uncancelled,
        )
        .expect("context admission baseline");
        let exact = RepositoryQueryAdmission {
            map_pages: baseline.work.map.pages_read,
            map_bytes: baseline.work.map.bytes_read,
            map_entries: baseline.work.map.entries_visited,
            catalog_lookups: baseline.work.store.catalog_lookups,
            store_objects: baseline.work.store.objects_read,
            store_bytes: baseline.work.store.bytes_read,
            canonical_records: baseline.work.canonical_records_decoded,
            witness_records: baseline.work.witness_records_decoded,
        };
        assert!(
            [
                exact.map_pages,
                exact.map_bytes,
                exact.map_entries,
                exact.catalog_lookups,
                exact.store_objects,
                exact.store_bytes,
                exact.canonical_records,
                exact.witness_records,
            ]
            .into_iter()
            .all(|value| value > 0)
        );
        let mut uncancelled = || Ok(());
        let exact_fit = execute_context(
            &view,
            exact,
            &request,
            &render_context,
            binding,
            selector,
            None,
            root,
            ContextDirection::Both,
            3,
            &mut uncancelled,
        )
        .expect("exact physical context admission");
        assert_eq!(exact_fit.items, baseline.items);
        assert_eq!(exact_fit.context_summary, baseline.context_summary);
        assert_eq!(exact_fit.context_work, baseline.context_work);

        let one_less = |value: u64| value.checked_sub(1).expect("positive context work");
        for (name, admission, expected_code) in [
            (
                "map-pages",
                RepositoryQueryAdmission {
                    map_pages: one_less(exact.map_pages),
                    ..exact
                },
                "persistent_map_admission_pages_read",
            ),
            (
                "map-bytes",
                RepositoryQueryAdmission {
                    map_bytes: one_less(exact.map_bytes),
                    ..exact
                },
                "persistent_map_admission_bytes_read",
            ),
            (
                "map-entries",
                RepositoryQueryAdmission {
                    map_entries: one_less(exact.map_entries),
                    ..exact
                },
                "persistent_map_admission_entries_visited",
            ),
            (
                "catalog-lookups",
                RepositoryQueryAdmission {
                    catalog_lookups: one_less(exact.catalog_lookups),
                    ..exact
                },
                "object_read_catalog_lookups_exhausted",
            ),
            (
                "store-objects",
                RepositoryQueryAdmission {
                    store_objects: one_less(exact.store_objects),
                    ..exact
                },
                "object_read_objects_exhausted",
            ),
            (
                "store-bytes",
                RepositoryQueryAdmission {
                    store_bytes: one_less(exact.store_bytes),
                    ..exact
                },
                "object_read_bytes_exhausted",
            ),
            (
                "canonical-records",
                RepositoryQueryAdmission {
                    canonical_records: one_less(exact.canonical_records),
                    ..exact
                },
                "query_admission_canonical_records",
            ),
            (
                "witness-records",
                RepositoryQueryAdmission {
                    witness_records: one_less(exact.witness_records),
                    ..exact
                },
                "query_admission_witness_records",
            ),
        ] {
            let mut uncancelled = || Ok(());
            let error = execute_context(
                &view,
                admission,
                &request,
                &render_context,
                binding,
                selector,
                None,
                root,
                ContextDirection::Both,
                3,
                &mut uncancelled,
            )
            .expect_err(name);
            assert_eq!(error.class, DiagnosticClass::Resource, "{name}");
            assert_eq!(error.code, expected_code, "{name}");
        }
        let maximum = context_query_admission();
        assert_eq!(maximum.canonical_records, MAXIMUM_CONTEXT_OWNERS);
        assert_eq!(maximum.witness_records, MAXIMUM_CONTEXT_RELATION_WITNESSES);
        assert_eq!(
            maximum.map_pages,
            MAXIMUM_CONTEXT_OWNERS * (MAXIMUM_OWNER_MAP_KEY_BYTES + 1)
                + (MAXIMUM_CONTEXT_RELATION_RANGES + MAXIMUM_CONTEXT_RELATION_WITNESSES)
                    * (MAXIMUM_RELATION_MAP_KEY_BYTES + 1)
        );
        assert_eq!(
            maximum.store_bytes,
            maximum.map_bytes + MAXIMUM_CONTEXT_OWNERS * 4 * 1_048_576
        );
    }

    #[test]
    fn context_logical_work_is_local_beside_one_hundred_and_ten_thousand_unrelated_owners() {
        let mut baseline = None;
        let mut baseline_work = None;
        for unrelated in [100_u64, 10_000_u64] {
            let temporary = tempfile::tempdir().expect("context locality parent");
            let destination = temporary.path().join("project");
            let mut snapshot = crate::platform::kernel::tests::witness_snapshot();
            let root = named_owner(&snapshot, "caller");
            for ordinal in 0..unrelated {
                let module_id = ModuleId::migrate(b"context-locality-unrelated", ordinal);
                let owner = OwnerKey::Module(module_id);
                assert!(
                    snapshot
                        .owners
                        .insert(
                            owner,
                            OwnerRecord::Module(ModuleRecord {
                                header: OwnerHeader::new(owner, OwnerKind::Module),
                                name: Name::new(format!("unrelated_{ordinal:05}"))
                                    .expect("unrelated module name"),
                            }),
                        )
                        .is_none()
                );
            }
            let owner_root = snapshot.root.owners;
            snapshot.root.owners = super::super::persistent_map::MapRoot::from_parts(
                owner_root.page(),
                u64::try_from(snapshot.owners.len()).expect("locality owner count"),
                owner_root.content(),
            );
            let created = GraphRepository::create(&destination, &snapshot, None)
                .expect("context locality repository");
            let view = created
                .repository
                .view_current()
                .expect("context locality view");
            let before = file_inventory(created.repository.root());
            let started = Instant::now();
            let (observed, pages) =
                collect_context_pages(&created.repository, &view, root, ContextDirection::Both, 3);
            let elapsed = started.elapsed();
            let work_record = pages[0]
                .iter()
                .find(|record| record.operation == "work")
                .expect("context locality work");
            let number = |name: &str| {
                field(work_record, name)
                    .expect("context locality work field")
                    .parse::<u64>()
                    .expect("context locality work number")
            };
            let logical_work = (
                observed.expanded_owners,
                observed.owners.len(),
                observed.relations.len(),
                observed.relation_witnesses,
            );
            if let Some(expected) = &baseline {
                assert_eq!(&observed, expected);
                assert_eq!(Some(logical_work), baseline_work);
            } else {
                baseline = Some(observed.clone());
                baseline_work = Some(logical_work);
            }
            assert_eq!(file_inventory(created.repository.root()), before);
            println!(
                "context-locality unrelated-owners={unrelated} selected-owners={} selected-relations={} expanded-owners={} relation-witnesses={} map-pages-read={} map-bytes-read={} map-entries-visited={} store-objects-read={} store-bytes-read={} wall-micros={} peak-rss-kib={}",
                observed.owners.len(),
                observed.relations.len(),
                observed.expanded_owners,
                observed.relation_witnesses,
                number("map-pages-read"),
                number("map-bytes-read"),
                number("map-entries-visited"),
                number("store-objects-read"),
                number("store-bytes-read"),
                elapsed.as_micros(),
                process_peak_rss_kib()
                    .map_or_else(|| "unavailable".to_owned(), |value| value.to_string()),
            );
        }
    }

    #[test]
    fn ten_thousand_owner_and_high_fanout_relation_pages_remain_logically_local() {
        let temporary = tempfile::tempdir().expect("scale query parent");
        let destination = temporary.path().join("project");
        let mut snapshot = crate::platform::kernel::tests::witness_snapshot();
        let module_id = ModuleId::migrate(b"normalized-query-scale", 0);
        let module = OwnerKey::Module(module_id);
        snapshot.owners.clear();
        let unit_type = snapshot
            .types
            .iter()
            .find_map(|(digest, object)| matches!(object.form, TypeForm::Unit).then_some(*digest))
            .expect("scale unit type");
        snapshot
            .types
            .retain(|digest, _object| *digest == unit_type);
        snapshot.owners.insert(
            module,
            OwnerRecord::Module(ModuleRecord {
                header: OwnerHeader::new(module, OwnerKind::Module),
                name: Name::new("scale").expect("scale module name"),
            }),
        );
        for ordinal in 0..9_999_u64 {
            let declaration_id = DeclarationId::migrate(b"normalized-query-scale", ordinal);
            let owner = OwnerKey::Declaration(declaration_id);
            snapshot.owners.insert(
                owner,
                OwnerRecord::Declaration(DeclarationRecord {
                    header: OwnerHeader::new(owner, OwnerKind::External),
                    module: module_id,
                    name: Name::new(format!("record_{ordinal:05}"))
                        .expect("scale declaration name"),
                    visibility: DeclarationVisibility::Public,
                    payload: DeclarationPayload::External(ExternalDeclaration {
                        type_parameters: Vec::new(),
                        parameters: Vec::new(),
                        result: unit_type,
                        implementation: ImplementationName::new("scale_host")
                            .expect("scale external implementation"),
                    }),
                }),
            );
        }
        let owner_root = snapshot.root.owners;
        snapshot.root.owners = super::super::persistent_map::MapRoot::from_parts(
            owner_root.page(),
            10_000,
            owner_root.content(),
        );
        let created = GraphRepository::create(&destination, &snapshot, None)
            .expect("10,000-owner normalized repository");
        let view = created.repository.view_current().expect("scale query view");
        let before_queries = file_inventory(created.repository.root());
        let binding = QueryBinding {
            repository: view.current().head.repository_id,
            package: view.package(),
            revision: view.revision(),
        };

        let mut owner_keys = snapshot
            .owners
            .keys()
            .map(|owner| EncodedOwnerKey::new(*owner).bytes().to_vec())
            .collect::<Vec<_>>();
        owner_keys.sort();
        let owner_selection = QuerySelection::Owners { kind: None };
        let owner_selector = query_digest(&owner_selection).expect("owner scale selector");
        let first_owner_request = NormalizedQueryRequest {
            selection: owner_selection.clone(),
            limits: QueryPageLimits {
                items: 5,
                output_bytes: DEFAULT_QUERY_OUTPUT_BYTES,
            },
            continuation: None,
        };
        let (first_owner_records, _) = measured_scale_query(
            "owner-first",
            &created.repository,
            &view,
            &first_owner_request,
        );
        assert_eq!(
            first_owner_records
                .iter()
                .filter(|record| record.operation == "owner")
                .count(),
            5
        );
        let owner_lower = owner_keys[9_000].clone();
        let mut observed_owners = Vec::new();
        let owner_page = view
            .visit_query_owners(
                Some(&owner_lower),
                None,
                6,
                5,
                query_admission(6, 5).expect("owner scale admission"),
                |owner, _record| {
                    observed_owners.push(EncodedOwnerKey::new(owner).bytes().to_vec());
                    Ok(MapRangeControl::Continue)
                },
            )
            .expect("middle owner page");
        assert_eq!(observed_owners, owner_keys[9_001..9_006]);
        assert!(owner_page.value.has_more);
        assert!(owner_page.work.map.entries_visited <= 4_102);
        assert!(owner_page.work.map.entries_skipped > 8_000);

        let middle_owner_request = NormalizedQueryRequest {
            selection: owner_selection.clone(),
            limits: first_owner_request.limits,
            continuation: Some(
                encode_continuation(
                    binding,
                    QueryOperation::Owners,
                    owner_selector,
                    &owner_lower,
                )
                .expect("middle owner continuation"),
            ),
        };
        let (middle_owner_records, _) = measured_scale_query(
            "owner-middle",
            &created.repository,
            &view,
            &middle_owner_request,
        );
        let middle_owner_work = middle_owner_records
            .iter()
            .find(|record| record.operation == "work")
            .expect("middle owner work");
        assert!(
            field(middle_owner_work, "map-entries-visited")
                .expect("middle owner map entries")
                .parse::<u64>()
                .expect("middle owner map entries number")
                <= 4_101
        );

        let terminal_owner_request = NormalizedQueryRequest {
            selection: owner_selection,
            limits: first_owner_request.limits,
            continuation: Some(
                encode_continuation(
                    binding,
                    QueryOperation::Owners,
                    owner_selector,
                    &owner_keys[owner_keys.len() - 6],
                )
                .expect("terminal owner continuation"),
            ),
        };
        let (terminal_owner_records, _) = measured_scale_query(
            "owner-terminal",
            &created.repository,
            &view,
            &terminal_owner_request,
        );
        assert!(
            terminal_owner_records
                .iter()
                .all(|record| record.operation != "continuation")
        );

        let mut filtered_request = NormalizedQueryRequest {
            selection: QuerySelection::Owners {
                kind: Some(OwnerKind::Target),
            },
            limits: QueryPageLimits {
                items: 1,
                output_bytes: DEFAULT_QUERY_OUTPUT_BYTES,
            },
            continuation: None,
        };
        let (filtered_first, _) = measured_scale_query(
            "owner-filter-empty-progress",
            &created.repository,
            &view,
            &filtered_request,
        );
        assert!(
            filtered_first
                .iter()
                .all(|record| record.operation != "owner")
        );
        let first_filter_token = filtered_first
            .iter()
            .find(|record| record.operation == "continuation")
            .and_then(|record| field(record, "token"))
            .expect("progressing empty filtered continuation")
            .to_owned();
        filtered_request.continuation = Some(first_filter_token.clone());
        let (filtered_second, _) = measured_scale_query(
            "owner-filter-empty-resume",
            &created.repository,
            &view,
            &filtered_request,
        );
        let second_filter_token = filtered_second
            .iter()
            .find(|record| record.operation == "continuation")
            .and_then(|record| field(record, "token"))
            .expect("second progressing empty filtered continuation");
        assert_ne!(second_filter_token, first_filter_token);

        let exact_name = Name::new("record_09000").expect("scale exact name");
        let exact_owner = snapshot
            .owners
            .iter()
            .find_map(|(owner, record)| {
                record
                    .name()
                    .is_some_and(|name| name == &exact_name)
                    .then_some(*owner)
            })
            .expect("scale exact owner");
        let namespace_read = view
            .query_namespace(
                &NamespaceKey {
                    parent: Some(module),
                    class: NamespaceClass::Declaration,
                    name: exact_name,
                },
                find_admission(),
            )
            .expect("bounded exact namespace read");
        assert_eq!(namespace_read.value, Some(exact_owner));
        assert!(namespace_read.work.map.entries_visited < 4_096);
        let (exact_find_records, _) = measured_scale_query(
            "find-exact",
            &created.repository,
            &view,
            &NormalizedQueryRequest {
                selection: QuerySelection::Find {
                    class: NamespaceClass::Declaration,
                    name: Name::new("record_09000").expect("scale exact query name"),
                    parent: Some(module),
                },
                limits: QueryPageLimits {
                    items: 1,
                    output_bytes: DEFAULT_QUERY_OUTPUT_BYTES,
                },
                continuation: None,
            },
        );
        assert_eq!(
            exact_find_records
                .iter()
                .find(|record| record.operation == "summary")
                .and_then(|record| field(record, "match")),
            Some("true")
        );

        let endpoint = RelationEndpoint::Owner(ExactOwnerKey {
            package: snapshot.root.package_id,
            owner: module,
        });
        let mut relation_keys = extract_relations(
            snapshot.root.package_id,
            &snapshot.owners,
            &snapshot.types,
            &snapshot.dependencies,
        )
        .expect("scale relation oracle")
        .into_iter()
        .filter(|edge| edge.target == endpoint)
        .map(reverse_relation_key)
        .collect::<Vec<_>>();
        relation_keys.sort();
        assert_eq!(relation_keys.len(), 9_999);
        let relation_selection = QuerySelection::Relations {
            endpoint: QueryEndpointSelector::Owner(module),
            direction: QueryDirection::Incoming,
            kind: Some(RelationKind::DeclarationModule),
        };
        let relation_selector = query_digest(&relation_selection).expect("relation scale selector");
        let relation_first_request = NormalizedQueryRequest {
            selection: relation_selection.clone(),
            limits: QueryPageLimits {
                items: 5,
                output_bytes: DEFAULT_QUERY_OUTPUT_BYTES,
            },
            continuation: None,
        };
        let (first_relation_records, _) = measured_scale_query(
            "relation-first-kind-prefix",
            &created.repository,
            &view,
            &relation_first_request,
        );
        assert_eq!(
            first_relation_records
                .iter()
                .filter(|record| record.operation == "relation")
                .count(),
            5
        );
        let relation_lower = relation_keys[9_000].clone();
        let mut observed_relations = Vec::new();
        let relation_page = view
            .visit_query_relations(
                RepositoryRelationQueryRange {
                    endpoint,
                    kind: None,
                    incoming: true,
                    exclusive_lower_bound: Some(&relation_lower),
                    maximum_scan: 6,
                    maximum_items: 5,
                },
                query_admission(6, 5).expect("relation scale admission"),
                |edge| {
                    observed_relations.push(reverse_relation_key(edge));
                    Ok(MapRangeControl::Continue)
                },
            )
            .expect("middle relation page");
        assert_eq!(observed_relations, relation_keys[9_001..9_006]);
        assert!(relation_page.value.has_more);
        assert!(relation_page.work.map.entries_visited <= 4_102);
        assert!(relation_page.work.map.entries_skipped > 8_000);

        let (middle_relation_records, _) = measured_scale_query(
            "relation-middle-kind-prefix",
            &created.repository,
            &view,
            &NormalizedQueryRequest {
                selection: relation_selection,
                limits: relation_first_request.limits,
                continuation: Some(
                    encode_continuation(
                        binding,
                        QueryOperation::Relations,
                        relation_selector,
                        &relation_lower,
                    )
                    .expect("middle relation continuation"),
                ),
            },
        );
        let middle_relation_work = middle_relation_records
            .iter()
            .find(|record| record.operation == "work")
            .expect("middle relation work");
        assert!(
            field(middle_relation_work, "map-entries-visited")
                .expect("middle relation map entries")
                .parse::<u64>()
                .expect("middle relation map entries number")
                <= 4_101
        );
        assert_eq!(file_inventory(created.repository.root()), before_queries);
        println!(
            "query-scale fixture-owners=10000 fixture-relations=9999 repository-bytes-written=0 peak-rss-kib={}",
            process_peak_rss_kib()
                .map_or_else(|| "unavailable".to_owned(), |value| value.to_string())
        );
    }
}
