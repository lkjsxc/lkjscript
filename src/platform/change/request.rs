//! High-level Graph 5 authored intent lowered to exact primitive owner edits.

use super::{
    CanonicalBaseRead, CanonicalReadWork, PrimitiveEdit, WitnessBaseRead, WitnessReadWork,
};
use crate::platform::contract::registry::CHANGE_ALLOCATION_SEED_DOMAIN;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    ExpressionOperation, ExpressionRecord, ModuleRecord, Name, NamespaceClass, OwnerHeader,
    OwnerKey, OwnerKind, OwnerRecord, encode_owner,
};
use crate::platform::semantic_id::{ExpressionId, ModuleId, RevisionId};
use crate::platform::witness::NamespaceKey;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const MAXIMUM_AUTHORED_CHANGES: usize = 10_000;
pub const MAXIMUM_AUTHORED_CHANGE_BYTES: usize = 4 * 1_048_576;
const MAXIMUM_REQUEST_SYMBOL_BYTES: usize = 128;

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredChangeSet {
    pub base: RevisionId,
    pub changes: Vec<AuthoredChange>,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum AuthoredChange {
    CreateModule {
        #[serde(rename = "as")]
        symbol: String,
        name: Name,
    },
    RenameOwner {
        owner: OwnerSelector,
        name: Name,
    },
    MoveDeclaration {
        declaration: DeclarationSelector,
        module: ModuleSelector,
    },
    ReplaceExpression {
        expression: ExpressionId,
        operation: ExpressionOperation,
    },
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(tag = "by", rename_all = "snake_case", deny_unknown_fields)]
pub enum OwnerSelector {
    Exact { owner: OwnerKey },
    ModuleName { name: Name },
    DeclarationName { module: ModuleSelector, name: Name },
    Symbol { symbol: String },
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(tag = "by", rename_all = "snake_case", deny_unknown_fields)]
pub enum ModuleSelector {
    Id { module: ModuleId },
    Name { name: Name },
    Symbol { symbol: String },
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(tag = "by", rename_all = "snake_case", deny_unknown_fields)]
pub enum DeclarationSelector {
    Id {
        declaration: crate::platform::semantic_id::DeclarationId,
    },
    Qualified {
        module: ModuleSelector,
        name: Name,
    },
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AuthoredLoweringWork {
    pub canonical: CanonicalReadWork,
    pub witness: WitnessReadWork,
}

#[derive(Clone, Debug)]
pub struct AuthoredLowering {
    pub edits: Vec<PrimitiveEdit>,
    pub allocated: BTreeMap<String, OwnerKey>,
    pub work: AuthoredLoweringWork,
}

pub fn lower_authored_changes<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    base: &B,
    witness: &W,
    request: &AuthoredChangeSet,
    idempotency_key: Option<&str>,
) -> Result<AuthoredLowering, Diagnostic> {
    if base.exact_revision() != Some(request.base) {
        return Err(request_error(
            DiagnosticClass::Semantic,
            "change_authored_stale_base",
            "authored change base is not the exact pinned repository revision",
        ));
    }
    if !witness.witness_contract_is_current()
        || witness.witness_repository_id() != base.repository_id()
        || witness.witness_package_id() != base.package_id()
        || witness.witness_manifest().semantic_root
            != crate::platform::kernel::encode_root(base.semantic_root())?.0
    {
        return Err(request_error(
            DiagnosticClass::Corrupt,
            "change_authored_witness_base",
            "authored change inputs do not share one exact canonical and witness base",
        ));
    }
    if request.changes.is_empty() || request.changes.len() > MAXIMUM_AUTHORED_CHANGES {
        return Err(request_error(
            DiagnosticClass::Resource,
            "change_authored_count",
            format!("authored change requires 1 through {MAXIMUM_AUTHORED_CHANGES} operations"),
        ));
    }

    let seed = allocation_seed(base, request, idempotency_key)?;
    let creations = collect_module_creations(request)?;
    let mut allocated = BTreeMap::new();
    for (index, symbol) in creations.keys().enumerate() {
        let ordinal = u64::try_from(index)
            .ok()
            .and_then(|value| value.checked_add(1))
            .ok_or_else(|| {
                request_error(
                    DiagnosticClass::Resource,
                    "change_authored_allocation_ordinal",
                    "request-local allocation ordinal was exhausted",
                )
            })?;
        allocated.insert(
            symbol.clone(),
            OwnerKey::Module(ModuleId::allocate(&seed, ordinal)),
        );
    }

    let mut lowerer = AuthoredLowerer::new(base, witness, allocated);
    for (symbol, name) in creations {
        let OwnerKey::Module(module) = lowerer.allocated[&symbol] else {
            unreachable!("module creation allocation has a fixed typed domain")
        };
        lowerer.insert_created(OwnerRecord::Module(ModuleRecord {
            header: OwnerHeader::new(OwnerKey::Module(module), OwnerKind::Module),
            name,
        }))?;
    }
    for change in &request.changes {
        match change {
            AuthoredChange::CreateModule { .. } => {}
            AuthoredChange::RenameOwner { owner, name } => {
                let owner = lowerer.resolve_owner(owner)?;
                rename_owner(lowerer.candidate_mut(owner)?, name.clone())?;
            }
            AuthoredChange::MoveDeclaration {
                declaration,
                module,
            } => {
                let declaration = lowerer.resolve_declaration(declaration)?;
                let module = lowerer.resolve_module(module)?;
                let OwnerRecord::Declaration(record) =
                    lowerer.candidate_mut(OwnerKey::Declaration(declaration))?
                else {
                    return Err(request_error(
                        DiagnosticClass::Corrupt,
                        "change_authored_declaration_record",
                        "resolved declaration identity is bound to a foreign owner record",
                    ));
                };
                record.module = module;
            }
            AuthoredChange::ReplaceExpression {
                expression,
                operation,
            } => {
                let owner = OwnerKey::Expression(*expression);
                let candidate = lowerer.candidate_mut(owner)?;
                let OwnerRecord::Expression(_) = candidate else {
                    return Err(request_error(
                        DiagnosticClass::Semantic,
                        "change_authored_expression_kind",
                        "expression selector does not name an expression owner",
                    ));
                };
                *candidate =
                    OwnerRecord::Expression(ExpressionRecord::new(*expression, operation.clone())?);
            }
        }
    }
    lowerer.finish()
}

fn collect_module_creations(
    request: &AuthoredChangeSet,
) -> Result<BTreeMap<String, Name>, Diagnostic> {
    let mut creations = BTreeMap::new();
    for change in &request.changes {
        if let AuthoredChange::CreateModule { symbol, name } = change {
            validate_symbol(symbol)?;
            if creations.insert(symbol.clone(), name.clone()).is_some() {
                return Err(request_error(
                    DiagnosticClass::Source,
                    "change_authored_symbol_duplicate",
                    format!("request-local symbol {symbol} is defined more than once"),
                ));
            }
        }
    }
    Ok(creations)
}

fn allocation_seed<B: CanonicalBaseRead + ?Sized>(
    base: &B,
    request: &AuthoredChangeSet,
    idempotency_key: Option<&str>,
) -> Result<[u8; 32], Diagnostic> {
    let configuration = bincode::config::standard()
        .with_little_endian()
        .with_variable_int_encoding();
    let mut counter =
        bincode::enc::EncoderImpl::new(bincode::enc::write::SizeWriter::default(), configuration);
    Encode::encode(request, &mut counter).map_err(|error| {
        request_error(
            DiagnosticClass::Resource,
            "change_authored_encode",
            format!("authored change cannot be canonically sized: {error}"),
        )
    })?;
    let encoded_bytes = counter.into_writer().bytes_written;
    if encoded_bytes > MAXIMUM_AUTHORED_CHANGE_BYTES {
        return Err(request_error(
            DiagnosticClass::Resource,
            "change_authored_bytes",
            format!(
                "authored change requires {encoded_bytes} canonical bytes, exceeding the {MAXIMUM_AUTHORED_CHANGE_BYTES}-byte budget"
            ),
        ));
    }
    let mut bytes = vec![0_u8; encoded_bytes];
    let written =
        bincode::encode_into_slice(request, &mut bytes, configuration).map_err(|error| {
            request_error(
                DiagnosticClass::Resource,
                "change_authored_encode",
                format!("authored change cannot be canonically encoded: {error}"),
            )
        })?;
    debug_assert_eq!(written, encoded_bytes);
    let idempotency = idempotency_key.unwrap_or_default().as_bytes();
    let mut hasher = blake3::Hasher::new_derive_key(CHANGE_ALLOCATION_SEED_DOMAIN);
    hasher.update(&base.repository_id().bytes());
    hasher.update(&request.base.bytes());
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(&bytes);
    hasher.update(&(idempotency.len() as u64).to_be_bytes());
    hasher.update(idempotency);
    Ok(*hasher.finalize().as_bytes())
}

struct WorkingOwner {
    before: Option<crate::platform::kernel::OwnerObjectDigest>,
    record: OwnerRecord,
}

struct AuthoredLowerer<'a, B: ?Sized, W: ?Sized> {
    base: &'a B,
    witness: &'a W,
    allocated: BTreeMap<String, OwnerKey>,
    owners: BTreeMap<OwnerKey, WorkingOwner>,
    namespace: BTreeMap<NamespaceKey, Option<OwnerKey>>,
    work: AuthoredLoweringWork,
}

impl<'a, B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized> AuthoredLowerer<'a, B, W> {
    fn new(base: &'a B, witness: &'a W, allocated: BTreeMap<String, OwnerKey>) -> Self {
        Self {
            base,
            witness,
            allocated,
            owners: BTreeMap::new(),
            namespace: BTreeMap::new(),
            work: AuthoredLoweringWork::default(),
        }
    }

    fn insert_created(&mut self, record: OwnerRecord) -> Result<(), Diagnostic> {
        let owner = record.owner();
        if self.owners.contains_key(&owner) {
            return Err(request_error(
                DiagnosticClass::Corrupt,
                "change_authored_allocation_collision",
                "one request-local identity was allocated more than once",
            ));
        }
        self.owners.insert(
            owner,
            WorkingOwner {
                before: None,
                record,
            },
        );
        Ok(())
    }

    fn resolve_owner(&mut self, selector: &OwnerSelector) -> Result<OwnerKey, Diagnostic> {
        let owner = match selector {
            OwnerSelector::Exact { owner } => *owner,
            OwnerSelector::ModuleName { name } => {
                OwnerKey::Module(self.resolve_module(&ModuleSelector::Name { name: name.clone() })?)
            }
            OwnerSelector::DeclarationName { module, name } => OwnerKey::Declaration(
                self.resolve_declaration(&DeclarationSelector::Qualified {
                    module: module.clone(),
                    name: name.clone(),
                })?,
            ),
            OwnerSelector::Symbol { symbol } => self.resolve_symbol(symbol)?,
        };
        self.require_owner(owner)?;
        Ok(owner)
    }

    fn resolve_module(&mut self, selector: &ModuleSelector) -> Result<ModuleId, Diagnostic> {
        let owner = match selector {
            ModuleSelector::Id { module } => OwnerKey::Module(*module),
            ModuleSelector::Name { name } => self.namespace_owner(NamespaceKey {
                parent: None,
                class: NamespaceClass::Module,
                name: name.clone(),
            })?,
            ModuleSelector::Symbol { symbol } => self.resolve_symbol(symbol)?,
        };
        self.require_owner(owner)?;
        match owner {
            OwnerKey::Module(module) => Ok(module),
            _ => Err(request_error(
                DiagnosticClass::Semantic,
                "change_authored_module_kind",
                "module selector resolved to a foreign owner domain",
            )),
        }
    }

    fn resolve_declaration(
        &mut self,
        selector: &DeclarationSelector,
    ) -> Result<crate::platform::semantic_id::DeclarationId, Diagnostic> {
        let owner = match selector {
            DeclarationSelector::Id { declaration } => OwnerKey::Declaration(*declaration),
            DeclarationSelector::Qualified { module, name } => {
                let module = self.resolve_module(module)?;
                self.namespace_owner(NamespaceKey {
                    parent: Some(OwnerKey::Module(module)),
                    class: NamespaceClass::Declaration,
                    name: name.clone(),
                })?
            }
        };
        self.require_owner(owner)?;
        match owner {
            OwnerKey::Declaration(declaration) => Ok(declaration),
            _ => Err(request_error(
                DiagnosticClass::Semantic,
                "change_authored_declaration_kind",
                "declaration selector resolved to a foreign owner domain",
            )),
        }
    }

    fn resolve_symbol(&self, symbol: &str) -> Result<OwnerKey, Diagnostic> {
        validate_symbol(symbol)?;
        self.allocated.get(symbol).copied().ok_or_else(|| {
            request_error(
                DiagnosticClass::Source,
                "change_authored_symbol_missing",
                format!("request-local symbol {symbol} has no unique definition"),
            )
        })
    }

    fn namespace_owner(&mut self, key: NamespaceKey) -> Result<OwnerKey, Diagnostic> {
        if !self.namespace.contains_key(&key) {
            let read = self.witness.read_namespace(&key)?;
            self.work.witness.add(read.work);
            self.namespace.insert(key.clone(), read.value);
        }
        self.namespace.get(&key).copied().flatten().ok_or_else(|| {
            request_error(
                DiagnosticClass::Semantic,
                "change_authored_selector_missing",
                format!("qualified selector has no owner at namespace key {key:?}"),
            )
        })
    }

    fn require_owner(&mut self, owner: OwnerKey) -> Result<(), Diagnostic> {
        if self.owners.contains_key(&owner) {
            return Ok(());
        }
        let read = self.base.read_owner(owner)?;
        self.work.canonical.add(read.work);
        let record = read.value.ok_or_else(|| {
            request_error(
                DiagnosticClass::Semantic,
                "change_authored_owner_missing",
                format!("selector names missing owner {owner:?}"),
            )
        })?;
        let (before, _) = encode_owner(&record)?;
        self.owners.insert(
            owner,
            WorkingOwner {
                before: Some(before),
                record,
            },
        );
        Ok(())
    }

    fn candidate_mut(&mut self, owner: OwnerKey) -> Result<&mut OwnerRecord, Diagnostic> {
        self.require_owner(owner)?;
        self.owners
            .get_mut(&owner)
            .map(|working| &mut working.record)
            .ok_or_else(|| {
                request_error(
                    DiagnosticClass::Corrupt,
                    "change_authored_owner_cache",
                    "resolved owner was not retained in the authored candidate overlay",
                )
            })
    }

    fn finish(self) -> Result<AuthoredLowering, Diagnostic> {
        let mut edits = Vec::new();
        for (_, working) in self.owners {
            let (after, _) = encode_owner(&working.record)?;
            match working.before {
                None => edits.push(PrimitiveEdit::InsertOwner {
                    record: working.record,
                }),
                Some(before) if before != after => edits.push(PrimitiveEdit::ReplaceOwner {
                    expected: before,
                    record: working.record,
                }),
                Some(_) => {}
            }
        }
        Ok(AuthoredLowering {
            edits,
            allocated: self.allocated,
            work: self.work,
        })
    }
}

fn rename_owner(record: &mut OwnerRecord, name: Name) -> Result<(), Diagnostic> {
    match record {
        OwnerRecord::Module(value) => value.name = name,
        OwnerRecord::Declaration(value) => value.name = name,
        OwnerRecord::TypeParameter(value) => value.name = name,
        OwnerRecord::Field(value) => value.name = name,
        OwnerRecord::Case(value) => value.name = name,
        OwnerRecord::Operation(value) => value.name = name,
        OwnerRecord::Parameter(value) => value.name = name,
        OwnerRecord::Binding(value) => value.name = name,
        OwnerRecord::Requirement(value) => value.name = name,
        OwnerRecord::Port(value) => value.name = name,
        OwnerRecord::Target(value) => value.name = name,
        OwnerRecord::Expression(_) | OwnerRecord::Documentation(_) | OwnerRecord::Annotation(_) => {
            return Err(request_error(
                DiagnosticClass::Semantic,
                "change_authored_rename_kind",
                "selected owner kind has no renameable semantic name",
            ));
        }
    }
    Ok(())
}

fn validate_symbol(symbol: &str) -> Result<(), Diagnostic> {
    if !symbol.starts_with('$')
        || symbol.len() < 2
        || symbol.len() > MAXIMUM_REQUEST_SYMBOL_BYTES
        || !symbol.as_bytes()[1..]
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(*byte, b'_' | b'-'))
    {
        return Err(request_error(
            DiagnosticClass::Source,
            "change_authored_symbol",
            format!(
                "request-local symbol must start with '$' and contain 1 through {} ASCII name bytes",
                MAXIMUM_REQUEST_SYMBOL_BYTES - 1
            ),
        ));
    }
    Ok(())
}

fn request_error(
    class: DiagnosticClass,
    code: &'static str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}
