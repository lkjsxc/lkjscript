//! Explicit deterministic encoding for normalized authored intent.
//!
//! Record and list order is part of authored intent and allocation traversal, including for
//! collections that lower into keyed graph relations. This codec does not claim to canonicalize
//! distinct authored requests that happen to produce behaviorally equal graph content.

use super::*;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    AnnotationClass, DeclarationVisibility, DocumentationClass, ExternalVisibility, Idempotency,
    ParameterUse, ResourceUnit,
};
use crate::platform::package::RunnerKind;

const INTENT_MAGIC: [u8; 8] = *b"LKJACR06";
const BUDGET_MAGIC: [u8; 8] = *b"LKJABG01";
const MAXIMUM_BUDGET_BYTES: usize = 1_024;

pub(super) fn encode_authored_intent(
    request: &AuthoredChangeSet,
    definitions: &BTreeMap<String, SymbolDefinition>,
) -> Result<Vec<u8>, Diagnostic> {
    let mut writer = Writer::new(MAXIMUM_AUTHORED_CHANGE_BYTES);
    writer.raw(&INTENT_MAGIC)?;
    writer.raw(&request.base.bytes())?;
    writer.list(&request.preconditions, |writer, precondition| {
        writer.precondition(precondition)
    })?;
    writer.list(&request.changes, |writer, change| {
        writer.change(change, definitions)
    })?;
    Ok(writer.finish())
}

pub(super) fn encode_budget(budget: ChangeBudget) -> Result<Vec<u8>, Diagnostic> {
    let mut writer = Writer::new(MAXIMUM_BUDGET_BYTES);
    writer.raw(&BUDGET_MAGIC)?;
    for maximum in [
        budget.authored.maximum_operations,
        budget.authored.maximum_preconditions,
        budget.authored.maximum_allocated_identities,
        budget.authored.maximum_type_nodes,
        budget.canonical_edits.maximum_owner_edits,
        budget.canonical_edits.maximum_type_edits,
        budget.canonical_edits.maximum_dependency_edits,
        budget.canonical_edits.maximum_retirement_edits,
        budget.canonical_reads.maximum_point_reads,
        budget.canonical_reads.maximum_map_pages,
        budget.canonical_reads.maximum_map_entries,
        budget.canonical_reads.maximum_catalog_lookups,
        budget.canonical_reads.maximum_objects,
        budget.canonical_reads.maximum_bytes,
        budget.canonical_reads.maximum_decoded_records,
        budget.canonical_map_update.maximum_pages_encoded,
        budget.canonical_map_update.maximum_bytes_encoded,
        budget.witness_reads.maximum_point_reads,
        budget.witness_reads.maximum_map_pages,
        budget.witness_reads.maximum_map_entries,
        budget.witness_reads.maximum_catalog_lookups,
        budget.witness_reads.maximum_objects,
        budget.witness_reads.maximum_bytes,
        budget.witness_reads.maximum_decoded_records,
        budget.impact.maximum_affected_owners,
        budget.impact.maximum_summary_owners,
        budget.impact.maximum_summary_edits,
        budget.impact.maximum_ownership_steps,
        budget.impact.maximum_behavior_owners,
        budget.impact.maximum_relation_edges,
        budget.impact.maximum_relation_fanout,
        budget.validation.maximum_owner_records,
        budget.validation.maximum_ownership_entries,
        budget.validation.maximum_type_objects,
        budget.validation.maximum_expression_steps,
        budget.validation.maximum_diagnostics,
        budget.tests.maximum_selected,
        budget.tests.maximum_dependencies_per_test,
        budget.tests.maximum_ownership_steps,
        budget.tests.maximum_owners_visited,
        budget.witness_update.maximum_edits,
        budget.witness_update.maximum_pages_encoded,
        budget.witness_update.maximum_bytes_encoded,
        budget.staging.maximum_objects,
        budget.staging.maximum_bytes,
        budget.staging.maximum_pages,
    ] {
        writer.u64(maximum)?;
    }
    Ok(writer.finish())
}

struct Writer {
    bytes: Vec<u8>,
    maximum: usize,
}

impl Writer {
    fn new(maximum: usize) -> Self {
        Self {
            bytes: Vec::new(),
            maximum,
        }
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }

    fn raw(&mut self, bytes: &[u8]) -> Result<(), Diagnostic> {
        let required = self.bytes.len().checked_add(bytes.len()).ok_or_else(|| {
            codec_error(
                "change_authored_bytes",
                "authored request byte length overflowed",
            )
        })?;
        if required > self.maximum {
            return Err(codec_error(
                "change_authored_bytes",
                format!(
                    "authored request requires more than the {}-byte encoding budget",
                    self.maximum
                ),
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }

    fn tag(&mut self, value: u8) -> Result<(), Diagnostic> {
        self.raw(&[value])
    }

    fn boolean(&mut self, value: bool) -> Result<(), Diagnostic> {
        self.tag(u8::from(value))
    }

    fn u64(&mut self, value: u64) -> Result<(), Diagnostic> {
        self.raw(&value.to_be_bytes())
    }

    fn i64(&mut self, value: i64) -> Result<(), Diagnostic> {
        self.raw(&value.to_be_bytes())
    }

    fn length(&mut self, value: usize) -> Result<(), Diagnostic> {
        self.u64(u64::try_from(value).map_err(|_| {
            codec_error(
                "change_authored_length",
                "authored collection length exceeds its encoding domain",
            )
        })?)
    }

    fn string(&mut self, value: &str) -> Result<(), Diagnostic> {
        self.length(value.len())?;
        self.raw(value.as_bytes())
    }

    fn name(&mut self, value: &Name) -> Result<(), Diagnostic> {
        self.string(value.as_str())
    }

    fn list<T>(
        &mut self,
        values: &[T],
        mut encode: impl FnMut(&mut Self, &T) -> Result<(), Diagnostic>,
    ) -> Result<(), Diagnostic> {
        self.length(values.len())?;
        for value in values {
            encode(self, value)?;
        }
        Ok(())
    }

    fn optional<T: ?Sized>(
        &mut self,
        value: Option<&T>,
        encode: impl FnOnce(&mut Self, &T) -> Result<(), Diagnostic>,
    ) -> Result<(), Diagnostic> {
        match value {
            Some(value) => {
                self.tag(1)?;
                encode(self, value)
            }
            None => self.tag(0),
        }
    }

    fn symbol(
        &mut self,
        value: &str,
        definitions: &BTreeMap<String, SymbolDefinition>,
    ) -> Result<(), Diagnostic> {
        validate_symbol(value)?;
        let definition = definitions.get(value).ok_or_else(|| {
            request_error(
                DiagnosticClass::Source,
                "change_authored_symbol_missing",
                format!("request-local symbol {value} has no unique definition"),
            )
        })?;
        self.tag(definition.kind.canonical_tag())?;
        self.u64(definition.ordinal)
    }

    fn precondition(&mut self, value: &AuthoredPrecondition) -> Result<(), Diagnostic> {
        match value {
            AuthoredPrecondition::OwnerExists { owner } => {
                self.tag(1)?;
                self.owner(*owner)
            }
            AuthoredPrecondition::OwnerAbsent { owner } => {
                self.tag(2)?;
                self.owner(*owner)
            }
            AuthoredPrecondition::OwnerName { owner, equals } => {
                self.tag(3)?;
                self.owner(*owner)?;
                self.name(equals)
            }
            AuthoredPrecondition::OwnerParent { owner, equals } => {
                self.tag(4)?;
                self.owner(*owner)?;
                match equals {
                    AuthoredOwnerParent::Package => self.tag(1),
                    AuthoredOwnerParent::Owner(parent) => {
                        self.tag(2)?;
                        self.owner(*parent)
                    }
                }
            }
            AuthoredPrecondition::NamespaceAbsent {
                parent,
                class,
                name,
            } => {
                self.tag(5)?;
                self.optional(parent.as_ref(), |writer, parent| writer.owner(*parent))?;
                self.tag(class.tag())?;
                self.name(name)
            }
            AuthoredPrecondition::NamespacePointsTo {
                parent,
                class,
                name,
                owner,
            } => {
                self.tag(6)?;
                self.optional(parent.as_ref(), |writer, parent| writer.owner(*parent))?;
                self.tag(class.tag())?;
                self.name(name)?;
                self.owner(*owner)
            }
            AuthoredPrecondition::DependencyBinding {
                package,
                semantic_revision,
                package_revision,
            } => {
                self.tag(7)?;
                self.raw(&package.bytes())?;
                self.raw(&semantic_revision.bytes())?;
                self.raw(&package_revision.bytes())
            }
        }
    }

    fn owner(&mut self, owner: OwnerKey) -> Result<(), Diagnostic> {
        self.tag(owner.identity_kind().tag())?;
        self.raw(&owner.bytes())
    }

    fn change(
        &mut self,
        value: &AuthoredChange,
        definitions: &BTreeMap<String, SymbolDefinition>,
    ) -> Result<(), Diagnostic> {
        match value {
            AuthoredChange::CreateModule { symbol, name } => {
                self.tag(1)?;
                self.symbol(symbol, definitions)?;
                self.name(name)
            }
            AuthoredChange::CreateFunction {
                symbol,
                module,
                name,
                visibility,
                type_parameters,
                parameters,
                result,
                effect,
                body,
            } => {
                self.tag(2)?;
                self.symbol(symbol, definitions)?;
                self.module_selector(module, definitions)?;
                self.name(name)?;
                self.visibility(*visibility)?;
                self.list(type_parameters, |writer, value| {
                    writer.type_parameter(value, definitions)
                })?;
                self.list(parameters, |writer, value| {
                    writer.parameter(value, definitions)
                })?;
                self.authored_type(result, definitions, 1)?;
                self.function_effect(effect, definitions)?;
                self.expression(body, definitions, 1)
            }
            AuthoredChange::CreateRecord {
                symbol,
                module,
                name,
                visibility,
                fields,
            } => {
                self.tag(3)?;
                self.symbol(symbol, definitions)?;
                self.module_selector(module, definitions)?;
                self.name(name)?;
                self.visibility(*visibility)?;
                self.list(fields, |writer, value| writer.field(value, definitions))
            }
            AuthoredChange::CreateVariant {
                symbol,
                module,
                name,
                visibility,
                cases,
            } => {
                self.tag(4)?;
                self.symbol(symbol, definitions)?;
                self.module_selector(module, definitions)?;
                self.name(name)?;
                self.visibility(*visibility)?;
                self.list(cases, |writer, value| writer.case(value, definitions))
            }
            AuthoredChange::CreateInterface {
                symbol,
                module,
                name,
                visibility,
                operations,
            } => {
                self.tag(5)?;
                self.symbol(symbol, definitions)?;
                self.module_selector(module, definitions)?;
                self.name(name)?;
                self.visibility(*visibility)?;
                self.list(operations, |writer, value| {
                    writer.operation(value, definitions)
                })
            }
            AuthoredChange::CreateExternal {
                symbol,
                module,
                name,
                visibility,
                type_parameters,
                parameters,
                result,
                implementation,
            } => {
                self.tag(6)?;
                self.symbol(symbol, definitions)?;
                self.module_selector(module, definitions)?;
                self.name(name)?;
                self.visibility(*visibility)?;
                self.list(type_parameters, |writer, value| {
                    writer.type_parameter(value, definitions)
                })?;
                self.list(parameters, |writer, value| {
                    writer.parameter(value, definitions)
                })?;
                self.authored_type(result, definitions, 1)?;
                self.string(implementation.as_str())
            }
            AuthoredChange::CreateConstant {
                symbol,
                module,
                name,
                visibility,
                ty,
                value,
            } => {
                self.tag(7)?;
                self.symbol(symbol, definitions)?;
                self.module_selector(module, definitions)?;
                self.name(name)?;
                self.visibility(*visibility)?;
                self.authored_type(ty, definitions, 1)?;
                self.expression(value, definitions, 1)
            }
            AuthoredChange::CreateComponent {
                symbol,
                module,
                name,
                visibility,
                requirements,
                ports,
            } => {
                self.tag(8)?;
                self.symbol(symbol, definitions)?;
                self.module_selector(module, definitions)?;
                self.name(name)?;
                self.visibility(*visibility)?;
                self.list(requirements, |writer, value| {
                    writer.requirement(value, definitions)
                })?;
                self.list(ports, |writer, value| writer.port(value, definitions))
            }
            AuthoredChange::CreateTest {
                symbol,
                module,
                name,
                visibility,
                actual,
                expected,
            } => {
                self.tag(9)?;
                self.symbol(symbol, definitions)?;
                self.module_selector(module, definitions)?;
                self.name(name)?;
                self.visibility(*visibility)?;
                self.expression(actual, definitions, 1)?;
                self.expression(expected, definitions, 1)
            }
            AuthoredChange::CreateTarget {
                symbol,
                name,
                component,
                port,
                runner,
            } => {
                self.tag(10)?;
                self.symbol(symbol, definitions)?;
                self.name(name)?;
                self.declaration_reference(component, definitions)?;
                self.port_reference(port, definitions)?;
                self.runner(*runner)
            }
            AuthoredChange::CreateDocumentation {
                symbol,
                owner,
                class,
                text,
            } => {
                self.tag(11)?;
                self.symbol(symbol, definitions)?;
                self.owner_selector(owner, definitions)?;
                self.documentation_class(*class)?;
                self.string(text)
            }
            AuthoredChange::CreateAnnotation {
                symbol,
                owner,
                class,
                key,
                value,
            } => {
                self.tag(12)?;
                self.symbol(symbol, definitions)?;
                self.owner_selector(owner, definitions)?;
                self.annotation_class(*class)?;
                self.name(key)?;
                self.annotation_value(value)
            }
            AuthoredChange::AddField { record, field } => {
                self.tag(13)?;
                self.declaration_selector(record, definitions)?;
                self.field(field, definitions)
            }
            AuthoredChange::AddCase { variant, case } => {
                self.tag(14)?;
                self.declaration_selector(variant, definitions)?;
                self.case(case, definitions)
            }
            AuthoredChange::AddOperation {
                interface,
                operation,
            } => {
                self.tag(15)?;
                self.declaration_selector(interface, definitions)?;
                self.operation(operation, definitions)
            }
            AuthoredChange::AddTypeParameter {
                declaration,
                parameter,
            } => {
                self.tag(16)?;
                self.declaration_selector(declaration, definitions)?;
                self.type_parameter(parameter, definitions)
            }
            AuthoredChange::AddParameter { parent, parameter } => {
                self.tag(17)?;
                self.parameter_parent_selector(parent, definitions)?;
                self.parameter(parameter, definitions)
            }
            AuthoredChange::AddRequirement {
                component,
                requirement,
            } => {
                self.tag(18)?;
                self.declaration_selector(component, definitions)?;
                self.requirement(requirement, definitions)
            }
            AuthoredChange::AddPort { component, port } => {
                self.tag(19)?;
                self.declaration_selector(component, definitions)?;
                self.port(port, definitions)
            }
            AuthoredChange::SetDeclarationVisibility {
                declaration,
                visibility,
            } => {
                self.tag(20)?;
                self.declaration_selector(declaration, definitions)?;
                self.visibility(*visibility)
            }
            AuthoredChange::SetFunctionContract {
                function,
                result,
                effect,
            } => {
                self.tag(21)?;
                self.declaration_selector(function, definitions)?;
                self.authored_type(result, definitions, 1)?;
                self.function_effect(effect, definitions)
            }
            AuthoredChange::SetExternalContract {
                external,
                result,
                implementation,
            } => {
                self.tag(22)?;
                self.declaration_selector(external, definitions)?;
                self.authored_type(result, definitions, 1)?;
                self.string(implementation.as_str())
            }
            AuthoredChange::SetFieldType { field, ty } => {
                self.tag(23)?;
                self.owner_selector(field, definitions)?;
                self.authored_type(ty, definitions, 1)
            }
            AuthoredChange::SetCasePayload { case, payload } => {
                self.tag(24)?;
                self.owner_selector(case, definitions)?;
                self.optional(payload.as_ref(), |writer, value| {
                    writer.authored_type(value, definitions, 1)
                })
            }
            AuthoredChange::SetParameterType { parameter, ty } => {
                self.tag(25)?;
                self.owner_selector(parameter, definitions)?;
                self.authored_type(ty, definitions, 1)
            }
            AuthoredChange::SetOperationContract {
                operation,
                result,
                idempotency,
                external_visibility,
            } => {
                self.tag(26)?;
                self.owner_selector(operation, definitions)?;
                self.authored_type(result, definitions, 1)?;
                self.idempotency(*idempotency)?;
                self.external_visibility(*external_visibility)
            }
            AuthoredChange::SetRequirementContract {
                requirement,
                interface,
                operations,
                limits,
            } => {
                self.tag(27)?;
                self.owner_selector(requirement, definitions)?;
                self.declaration_reference(interface, definitions)?;
                self.list(operations, |writer, value| {
                    writer.operation_reference(value, definitions)
                })?;
                self.list(limits, |writer, value| writer.resource_limit(value))
            }
            AuthoredChange::SetTarget {
                target,
                component,
                port,
                runner,
            } => {
                self.tag(28)?;
                self.owner_selector(target, definitions)?;
                self.declaration_reference(component, definitions)?;
                self.port_reference(port, definitions)?;
                self.runner(*runner)
            }
            AuthoredChange::AddDependency {
                package,
                semantic_revision,
                package_revision,
            } => {
                self.tag(29)?;
                self.raw(&package.bytes())?;
                self.raw(&semantic_revision.bytes())?;
                self.raw(&package_revision.bytes())
            }
            AuthoredChange::ReplaceDependency {
                package,
                semantic_revision,
                package_revision,
            } => {
                self.tag(30)?;
                self.raw(&package.bytes())?;
                self.raw(&semantic_revision.bytes())?;
                self.raw(&package_revision.bytes())
            }
            AuthoredChange::DeleteDependency { package } => {
                self.tag(31)?;
                self.raw(&package.bytes())
            }
            AuthoredChange::DeleteOwner { owner, policy } => {
                self.tag(32)?;
                self.owner_selector(owner, definitions)?;
                self.delete_policy(policy)
            }
            AuthoredChange::RenameOwner { owner, name } => {
                self.tag(33)?;
                self.owner_selector(owner, definitions)?;
                self.name(name)
            }
            AuthoredChange::MoveDeclaration {
                declaration,
                module,
            } => {
                self.tag(34)?;
                self.declaration_selector(declaration, definitions)?;
                self.module_selector(module, definitions)
            }
            AuthoredChange::ReplaceFunctionBody { function, body } => {
                self.tag(35)?;
                self.declaration_selector(function, definitions)?;
                self.expression(body, definitions, 1)
            }
        }
    }

    fn owner_selector(
        &mut self,
        value: &OwnerSelector,
        definitions: &BTreeMap<String, SymbolDefinition>,
    ) -> Result<(), Diagnostic> {
        match value {
            OwnerSelector::Exact { owner } => {
                self.tag(1)?;
                self.owner(*owner)
            }
            OwnerSelector::ModuleName { name } => {
                self.tag(2)?;
                self.name(name)
            }
            OwnerSelector::DeclarationName { module, name } => {
                self.tag(3)?;
                self.module_selector(module, definitions)?;
                self.name(name)
            }
            OwnerSelector::Symbol { symbol } => {
                self.tag(4)?;
                self.symbol(symbol, definitions)
            }
        }
    }

    fn delete_policy(&mut self, value: &AuthoredDeletePolicy) -> Result<(), Diagnostic> {
        match value {
            AuthoredDeletePolicy::Reject => self.tag(1),
            AuthoredDeletePolicy::OwnedClosure => self.tag(2),
        }
    }

    fn module_selector(
        &mut self,
        value: &ModuleSelector,
        definitions: &BTreeMap<String, SymbolDefinition>,
    ) -> Result<(), Diagnostic> {
        match value {
            ModuleSelector::Id { module } => {
                self.tag(1)?;
                self.raw(&module.bytes())
            }
            ModuleSelector::Name { name } => {
                self.tag(2)?;
                self.name(name)
            }
            ModuleSelector::Symbol { symbol } => {
                self.tag(3)?;
                self.symbol(symbol, definitions)
            }
        }
    }

    fn declaration_selector(
        &mut self,
        value: &DeclarationSelector,
        definitions: &BTreeMap<String, SymbolDefinition>,
    ) -> Result<(), Diagnostic> {
        match value {
            DeclarationSelector::Id { declaration } => {
                self.tag(1)?;
                self.raw(&declaration.bytes())
            }
            DeclarationSelector::Qualified { module, name } => {
                self.tag(2)?;
                self.module_selector(module, definitions)?;
                self.name(name)
            }
            DeclarationSelector::Symbol { symbol } => {
                self.tag(3)?;
                self.symbol(symbol, definitions)
            }
        }
    }

    fn parameter_parent_selector(
        &mut self,
        value: &ParameterParentSelector,
        definitions: &BTreeMap<String, SymbolDefinition>,
    ) -> Result<(), Diagnostic> {
        match value {
            ParameterParentSelector::Declaration { declaration } => {
                self.tag(1)?;
                self.declaration_selector(declaration, definitions)
            }
            ParameterParentSelector::Operation { operation } => {
                self.tag(2)?;
                self.owner_selector(operation, definitions)
            }
        }
    }

    fn type_parameter(
        &mut self,
        value: &AuthoredTypeParameter,
        definitions: &BTreeMap<String, SymbolDefinition>,
    ) -> Result<(), Diagnostic> {
        self.symbol(&value.symbol, definitions)?;
        self.name(&value.name)
    }

    fn parameter(
        &mut self,
        value: &AuthoredParameter,
        definitions: &BTreeMap<String, SymbolDefinition>,
    ) -> Result<(), Diagnostic> {
        self.symbol(&value.symbol, definitions)?;
        self.name(&value.name)?;
        self.authored_type(&value.ty, definitions, 1)?;
        self.parameter_use(value.use_mode)
    }

    fn function_effect(
        &mut self,
        value: &AuthoredFunctionEffect,
        definitions: &BTreeMap<String, SymbolDefinition>,
    ) -> Result<(), Diagnostic> {
        match value {
            AuthoredFunctionEffect::Pure {} => self.tag(1),
            AuthoredFunctionEffect::Task { requirements } => {
                self.tag(2)?;
                self.list(requirements, |writer, value| {
                    writer.requirement_reference(value, definitions)
                })
            }
        }
    }

    fn authored_type(
        &mut self,
        value: &AuthoredType,
        definitions: &BTreeMap<String, SymbolDefinition>,
        depth: usize,
    ) -> Result<(), Diagnostic> {
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return Err(codec_error(
                "change_authored_type_depth",
                "authored type exceeds the maximum structural depth",
            ));
        }
        let next = depth.saturating_add(1);
        match value {
            AuthoredType::Unit {} => self.tag(1),
            AuthoredType::Bool {} => self.tag(2),
            AuthoredType::I64 {} => self.tag(3),
            AuthoredType::Bytes {} => self.tag(4),
            AuthoredType::Text {} => self.tag(5),
            AuthoredType::StaticText {} => self.tag(6),
            AuthoredType::Secret {} => self.tag(7),
            AuthoredType::TypeParameter { parameter } => {
                self.tag(8)?;
                self.type_parameter_reference(parameter, definitions)
            }
            AuthoredType::Named { declaration } => {
                self.tag(9)?;
                self.declaration_reference(declaration, definitions)
            }
            AuthoredType::CapabilityResource { interface } => {
                self.tag(17)?;
                self.declaration_reference(interface, definitions)
            }
            AuthoredType::StructuralRecord { fields } => {
                self.tag(10)?;
                self.list(fields, |writer, value| {
                    writer.name(&value.name)?;
                    writer.authored_type(&value.ty, definitions, next)
                })
            }
            AuthoredType::List { item } => {
                self.tag(11)?;
                self.authored_type(item, definitions, next)
            }
            AuthoredType::Map { key, value } => {
                self.tag(12)?;
                self.authored_type(key, definitions, next)?;
                self.authored_type(value, definitions, next)
            }
            AuthoredType::Option { item } => {
                self.tag(13)?;
                self.authored_type(item, definitions, next)
            }
            AuthoredType::Result { ok, error } => {
                self.tag(14)?;
                self.authored_type(ok, definitions, next)?;
                self.authored_type(error, definitions, next)
            }
            AuthoredType::Stream { item } => {
                self.tag(15)?;
                self.authored_type(item, definitions, next)
            }
            AuthoredType::Function { parameters, result } => {
                self.tag(16)?;
                self.list(parameters, |writer, value| {
                    writer.authored_type(value, definitions, next)
                })?;
                self.authored_type(result, definitions, next)
            }
        }
    }

    fn parameter_use(&mut self, value: ParameterUse) -> Result<(), Diagnostic> {
        self.tag(match value {
            ParameterUse::Unrestricted => 1,
            ParameterUse::Borrow => 2,
            ParameterUse::Consume => 3,
        })
    }

    fn type_parameter_reference(
        &mut self,
        value: &AuthoredTypeParameterReference,
        definitions: &BTreeMap<String, SymbolDefinition>,
    ) -> Result<(), Diagnostic> {
        match value {
            AuthoredTypeParameterReference::Id { parameter } => {
                self.tag(1)?;
                self.raw(&parameter.bytes())
            }
            AuthoredTypeParameterReference::Symbol { symbol } => {
                self.tag(2)?;
                self.symbol(symbol, definitions)
            }
        }
    }

    fn declaration_reference(
        &mut self,
        value: &AuthoredDeclarationReference,
        definitions: &BTreeMap<String, SymbolDefinition>,
    ) -> Result<(), Diagnostic> {
        match value {
            AuthoredDeclarationReference::Local { declaration } => {
                self.tag(1)?;
                self.declaration_selector(declaration, definitions)
            }
            AuthoredDeclarationReference::Exact {
                package,
                declaration,
            } => {
                self.tag(2)?;
                self.raw(&package.bytes())?;
                self.raw(&declaration.bytes())
            }
        }
    }

    fn field_reference(
        &mut self,
        value: &AuthoredFieldReference,
        definitions: &BTreeMap<String, SymbolDefinition>,
    ) -> Result<(), Diagnostic> {
        match value {
            AuthoredFieldReference::Exact { package, field } => {
                self.tag(1)?;
                self.raw(&package.bytes())?;
                self.raw(&field.bytes())
            }
            AuthoredFieldReference::Symbol { symbol } => {
                self.tag(2)?;
                self.symbol(symbol, definitions)
            }
        }
    }

    fn case_reference(
        &mut self,
        value: &AuthoredCaseReference,
        definitions: &BTreeMap<String, SymbolDefinition>,
    ) -> Result<(), Diagnostic> {
        match value {
            AuthoredCaseReference::Exact { package, case } => {
                self.tag(1)?;
                self.raw(&package.bytes())?;
                self.raw(&case.bytes())
            }
            AuthoredCaseReference::Symbol { symbol } => {
                self.tag(2)?;
                self.symbol(symbol, definitions)
            }
        }
    }

    fn operation_reference(
        &mut self,
        value: &AuthoredOperationReference,
        definitions: &BTreeMap<String, SymbolDefinition>,
    ) -> Result<(), Diagnostic> {
        match value {
            AuthoredOperationReference::Exact { package, operation } => {
                self.tag(1)?;
                self.raw(&package.bytes())?;
                self.raw(&operation.bytes())
            }
            AuthoredOperationReference::Symbol { symbol } => {
                self.tag(2)?;
                self.symbol(symbol, definitions)
            }
        }
    }

    fn requirement_reference(
        &mut self,
        value: &AuthoredRequirementReference,
        definitions: &BTreeMap<String, SymbolDefinition>,
    ) -> Result<(), Diagnostic> {
        match value {
            AuthoredRequirementReference::Exact {
                package,
                requirement,
            } => {
                self.tag(1)?;
                self.raw(&package.bytes())?;
                self.raw(&requirement.bytes())
            }
            AuthoredRequirementReference::Symbol { symbol } => {
                self.tag(2)?;
                self.symbol(symbol, definitions)
            }
        }
    }

    fn port_reference(
        &mut self,
        value: &AuthoredPortReference,
        definitions: &BTreeMap<String, SymbolDefinition>,
    ) -> Result<(), Diagnostic> {
        match value {
            AuthoredPortReference::Exact { package, port } => {
                self.tag(1)?;
                self.raw(&package.bytes())?;
                self.raw(&port.bytes())
            }
            AuthoredPortReference::Symbol { symbol } => {
                self.tag(2)?;
                self.symbol(symbol, definitions)
            }
        }
    }

    fn local_reference(
        &mut self,
        value: &AuthoredLocalReference,
        definitions: &BTreeMap<String, SymbolDefinition>,
    ) -> Result<(), Diagnostic> {
        match value {
            AuthoredLocalReference::FunctionParameter { parameter } => {
                self.tag(1)?;
                self.raw(&parameter.bytes())
            }
            AuthoredLocalReference::OperationParameter { parameter } => {
                self.tag(2)?;
                self.raw(&parameter.bytes())
            }
            AuthoredLocalReference::LexicalBinding { binding } => {
                self.tag(3)?;
                self.raw(&binding.bytes())
            }
            AuthoredLocalReference::MatchPayload { binding } => {
                self.tag(4)?;
                self.raw(&binding.bytes())
            }
            AuthoredLocalReference::TransactionBinding { binding } => {
                self.tag(5)?;
                self.raw(&binding.bytes())
            }
            AuthoredLocalReference::Symbol { symbol } => {
                self.tag(6)?;
                self.symbol(symbol, definitions)
            }
        }
    }

    fn expression(
        &mut self,
        value: &AuthoredExpression,
        definitions: &BTreeMap<String, SymbolDefinition>,
        depth: usize,
    ) -> Result<(), Diagnostic> {
        if depth > crate::platform::kernel::contract::MAXIMUM_EXPRESSION_DEPTH {
            return Err(codec_error(
                "change_authored_expression_depth",
                "authored expression exceeds the maximum structural depth",
            ));
        }
        self.optional(value.symbol.as_deref(), |writer, symbol| {
            writer.symbol(symbol, definitions)
        })?;
        self.expression_operation(&value.operation, definitions, depth)
    }

    fn expression_operation(
        &mut self,
        value: &AuthoredExpressionOperation,
        definitions: &BTreeMap<String, SymbolDefinition>,
        depth: usize,
    ) -> Result<(), Diagnostic> {
        let next = depth.saturating_add(1);
        match value {
            AuthoredExpressionOperation::Unit {} => self.tag(1),
            AuthoredExpressionOperation::Bool { value } => {
                self.tag(2)?;
                self.boolean(*value)
            }
            AuthoredExpressionOperation::I64 { value } => {
                self.tag(3)?;
                self.i64(*value)
            }
            AuthoredExpressionOperation::Text { value } => {
                self.tag(4)?;
                self.string(value)
            }
            AuthoredExpressionOperation::StaticText { value } => {
                self.tag(5)?;
                self.string(value)
            }
            AuthoredExpressionOperation::Local { value } => {
                self.tag(6)?;
                self.local_reference(value, definitions)
            }
            AuthoredExpressionOperation::Constant { declaration } => {
                self.tag(7)?;
                self.declaration_reference(declaration, definitions)
            }
            AuthoredExpressionOperation::If {
                condition,
                when_true,
                when_false,
            } => {
                self.tag(8)?;
                self.expression(condition, definitions, next)?;
                self.expression(when_true, definitions, next)?;
                self.expression(when_false, definitions, next)
            }
            AuthoredExpressionOperation::Let { bindings, body } => {
                self.tag(9)?;
                self.list(bindings, |writer, binding| {
                    writer.symbol(&binding.symbol, definitions)?;
                    writer.name(&binding.name)?;
                    writer.expression(&binding.value, definitions, next)?;
                    writer.optional(binding.declared_type.as_ref(), |writer, value| {
                        writer.authored_type(value, definitions, 1)
                    })
                })?;
                self.expression(body, definitions, next)
            }
            AuthoredExpressionOperation::Sequence { items } => {
                self.tag(10)?;
                self.list(items, |writer, value| {
                    writer.expression(value, definitions, next)
                })
            }
            AuthoredExpressionOperation::Call {
                function,
                type_arguments,
                arguments,
            } => {
                self.tag(11)?;
                self.declaration_reference(function, definitions)?;
                self.list(type_arguments, |writer, value| {
                    writer.authored_type(value, definitions, 1)
                })?;
                self.list(arguments, |writer, value| {
                    writer.expression(value, definitions, next)
                })
            }
            AuthoredExpressionOperation::FunctionValue {
                function,
                type_arguments,
            } => {
                self.tag(12)?;
                self.declaration_reference(function, definitions)?;
                self.list(type_arguments, |writer, value| {
                    writer.authored_type(value, definitions, 1)
                })
            }
            AuthoredExpressionOperation::Invoke { callee, arguments } => {
                self.tag(13)?;
                self.expression(callee, definitions, next)?;
                self.list(arguments, |writer, value| {
                    writer.expression(value, definitions, next)
                })
            }
            AuthoredExpressionOperation::Record {
                nominal_type,
                fields,
            } => {
                self.tag(14)?;
                self.optional(nominal_type.as_ref(), |writer, value| {
                    writer.declaration_reference(value, definitions)
                })?;
                self.list(fields, |writer, field| {
                    writer.field_selector(&field.selector, definitions)?;
                    writer.expression(&field.value, definitions, next)
                })
            }
            AuthoredExpressionOperation::Variant { case, payload } => {
                self.tag(15)?;
                self.case_reference(case, definitions)?;
                self.optional(payload.as_deref(), |writer, value| {
                    writer.expression(value, definitions, next)
                })
            }
            AuthoredExpressionOperation::Field { value, selector } => {
                self.tag(16)?;
                self.expression(value, definitions, next)?;
                self.field_selector(selector, definitions)
            }
            AuthoredExpressionOperation::List { item_type, items } => {
                self.tag(17)?;
                self.authored_type(item_type, definitions, 1)?;
                self.list(items, |writer, value| {
                    writer.expression(value, definitions, next)
                })
            }
            AuthoredExpressionOperation::Map {
                key_type,
                value_type,
                entries,
            } => {
                self.tag(18)?;
                self.authored_type(key_type, definitions, 1)?;
                self.authored_type(value_type, definitions, 1)?;
                self.list(entries, |writer, entry| {
                    writer.expression(&entry.key, definitions, next)?;
                    writer.expression(&entry.value, definitions, next)
                })
            }
            AuthoredExpressionOperation::Match { value, arms } => {
                self.tag(19)?;
                self.expression(value, definitions, next)?;
                self.list(arms, |writer, arm| {
                    writer.case_reference(&arm.case, definitions)?;
                    writer.optional(arm.payload_binding.as_ref(), |writer, binding| {
                        writer.symbol(&binding.symbol, definitions)?;
                        writer.name(&binding.name)?;
                        writer.authored_type(
                            binding.declared_type.as_ref().ok_or_else(|| {
                                codec_error(
                                    "change_codec_match_binding_type",
                                    "match payload binding omitted its exact declared type",
                                )
                            })?,
                            definitions,
                            1,
                        )
                    })?;
                    writer.expression(&arm.body, definitions, next)
                })
            }
            AuthoredExpressionOperation::CapabilityCall {
                requirement,
                operation,
                arguments,
            } => {
                self.tag(20)?;
                self.requirement_reference(requirement, definitions)?;
                self.operation_reference(operation, definitions)?;
                self.list(arguments, |writer, value| {
                    writer.expression(value, definitions, next)
                })
            }
            AuthoredExpressionOperation::Transaction {
                requirement,
                binding,
                body,
            } => {
                self.tag(21)?;
                self.requirement_reference(requirement, definitions)?;
                self.symbol(&binding.symbol, definitions)?;
                self.name(&binding.name)?;
                self.expression(body, definitions, next)
            }
        }
    }

    fn field_selector(
        &mut self,
        value: &AuthoredFieldSelector,
        definitions: &BTreeMap<String, SymbolDefinition>,
    ) -> Result<(), Diagnostic> {
        match value {
            AuthoredFieldSelector::Nominal { field } => {
                self.tag(1)?;
                self.field_reference(field, definitions)
            }
            AuthoredFieldSelector::Structural { name } => {
                self.tag(2)?;
                self.name(name)
            }
        }
    }

    fn field(
        &mut self,
        value: &AuthoredField,
        definitions: &BTreeMap<String, SymbolDefinition>,
    ) -> Result<(), Diagnostic> {
        self.symbol(&value.symbol, definitions)?;
        self.name(&value.name)?;
        self.authored_type(&value.ty, definitions, 1)
    }

    fn case(
        &mut self,
        value: &AuthoredCase,
        definitions: &BTreeMap<String, SymbolDefinition>,
    ) -> Result<(), Diagnostic> {
        self.symbol(&value.symbol, definitions)?;
        self.name(&value.name)?;
        self.optional(value.payload.as_ref(), |writer, value| {
            writer.authored_type(value, definitions, 1)
        })
    }

    fn operation(
        &mut self,
        value: &AuthoredOperation,
        definitions: &BTreeMap<String, SymbolDefinition>,
    ) -> Result<(), Diagnostic> {
        self.symbol(&value.symbol, definitions)?;
        self.name(&value.name)?;
        self.list(&value.parameters, |writer, value| {
            writer.parameter(value, definitions)
        })?;
        self.authored_type(&value.result, definitions, 1)?;
        self.idempotency(value.idempotency)?;
        self.external_visibility(value.external_visibility)
    }

    fn resource_limit(&mut self, value: &AuthoredResourceLimit) -> Result<(), Diagnostic> {
        self.name(&value.name)?;
        self.u64(value.maximum)?;
        self.resource_unit(value.unit)
    }

    fn requirement(
        &mut self,
        value: &AuthoredRequirement,
        definitions: &BTreeMap<String, SymbolDefinition>,
    ) -> Result<(), Diagnostic> {
        self.symbol(&value.symbol, definitions)?;
        self.name(&value.name)?;
        self.declaration_reference(&value.interface, definitions)?;
        self.list(&value.operations, |writer, value| {
            writer.operation_reference(value, definitions)
        })?;
        self.list(&value.limits, |writer, value| writer.resource_limit(value))
    }

    fn port(
        &mut self,
        value: &AuthoredPort,
        definitions: &BTreeMap<String, SymbolDefinition>,
    ) -> Result<(), Diagnostic> {
        self.symbol(&value.symbol, definitions)?;
        self.name(&value.name)?;
        self.authored_type(&value.function_type, definitions, 1)?;
        match &value.implementation {
            AuthoredPortImplementation::Expression { expression } => {
                self.tag(1)?;
                self.expression(expression, definitions, 1)
            }
            AuthoredPortImplementation::Function { function } => {
                self.tag(2)?;
                self.declaration_reference(function, definitions)
            }
        }
    }

    fn annotation_value(&mut self, value: &AuthoredAnnotationValue) -> Result<(), Diagnostic> {
        match value {
            AuthoredAnnotationValue::Bool(value) => {
                self.tag(1)?;
                self.boolean(*value)
            }
            AuthoredAnnotationValue::I64(value) => {
                self.tag(2)?;
                self.i64(*value)
            }
            AuthoredAnnotationValue::Text(value) => {
                self.tag(3)?;
                self.string(value)
            }
            AuthoredAnnotationValue::Name(value) => {
                self.tag(4)?;
                self.name(value)
            }
        }
    }

    fn visibility(&mut self, value: DeclarationVisibility) -> Result<(), Diagnostic> {
        self.tag(match value {
            DeclarationVisibility::Private => 1,
            DeclarationVisibility::Package => 2,
            DeclarationVisibility::Public => 3,
        })
    }

    fn idempotency(&mut self, value: Idempotency) -> Result<(), Diagnostic> {
        self.tag(match value {
            Idempotency::Idempotent => 1,
            Idempotency::IdempotentWithKey => 2,
            Idempotency::NonIdempotent => 3,
        })
    }

    fn external_visibility(&mut self, value: ExternalVisibility) -> Result<(), Diagnostic> {
        self.tag(match value {
            ExternalVisibility::None => 1,
            ExternalVisibility::Possible => 2,
        })
    }

    fn resource_unit(&mut self, value: ResourceUnit) -> Result<(), Diagnostic> {
        self.tag(match value {
            ResourceUnit::Bytes => 1,
            ResourceUnit::Items => 2,
            ResourceUnit::Calls => 3,
            ResourceUnit::Tasks => 4,
            ResourceUnit::Milliseconds => 5,
        })
    }

    fn documentation_class(&mut self, value: DocumentationClass) -> Result<(), Diagnostic> {
        self.tag(match value {
            DocumentationClass::Semantic => 1,
            DocumentationClass::Nonsemantic => 2,
        })
    }

    fn annotation_class(&mut self, value: AnnotationClass) -> Result<(), Diagnostic> {
        self.tag(match value {
            AnnotationClass::Semantic => 1,
            AnnotationClass::Nonsemantic => 2,
        })
    }

    fn runner(&mut self, value: RunnerKind) -> Result<(), Diagnostic> {
        self.tag(match value {
            RunnerKind::Command => 1,
            RunnerKind::Http => 2,
            RunnerKind::Interactive => 3,
            RunnerKind::Batch => 4,
            RunnerKind::Worker => 5,
            RunnerKind::Test => 6,
        })
    }
}

fn codec_error(code: &'static str, message: impl Into<String>) -> Diagnostic {
    request_error(DiagnosticClass::Resource, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn revision() -> RevisionId {
        RevisionId::from_digest([9; 32])
    }

    fn connected_request(
        module_symbol: &str,
        function_symbol: &str,
        parameter_symbol: &str,
        body_symbol: &str,
    ) -> AuthoredChangeSet {
        AuthoredChangeSet {
            base: revision(),
            preconditions: Vec::new(),
            changes: vec![
                AuthoredChange::CreateModule {
                    symbol: module_symbol.to_owned(),
                    name: Name::new("application").unwrap(),
                },
                AuthoredChange::CreateFunction {
                    symbol: function_symbol.to_owned(),
                    module: ModuleSelector::Symbol {
                        symbol: module_symbol.to_owned(),
                    },
                    name: Name::new("identity").unwrap(),
                    visibility: DeclarationVisibility::Public,
                    type_parameters: Vec::new(),
                    parameters: vec![AuthoredParameter {
                        symbol: parameter_symbol.to_owned(),
                        name: Name::new("value").unwrap(),
                        ty: AuthoredType::Text {},
                        use_mode: ParameterUse::Unrestricted,
                    }],
                    result: AuthoredType::Text {},
                    effect: AuthoredFunctionEffect::Pure {},
                    body: AuthoredExpression {
                        symbol: Some(body_symbol.to_owned()),
                        operation: AuthoredExpressionOperation::Local {
                            value: AuthoredLocalReference::Symbol {
                                symbol: parameter_symbol.to_owned(),
                            },
                        },
                    },
                },
            ],
            budget: ChangeBudget::default(),
        }
    }

    #[test]
    fn authored_intent_encoding_is_explicit_golden_and_label_independent() {
        let first = connected_request("$module", "$function", "$parameter", "$body");
        let renamed = connected_request("$m", "$f", "$p", "$expression");
        let first = canonical_authored_intent_bytes(&first).unwrap();
        let renamed = canonical_authored_intent_bytes(&renamed).unwrap();
        assert_eq!(first, renamed);
        assert_eq!(&first[..8], &INTENT_MAGIC);
        assert_eq!(
            crate::platform::semantic_id::encode_hex(blake3::hash(&first).as_bytes()),
            "8a09b2b164bafecbf4a1c4f572e1c57cf0741271dacc50ddb8d51aaed9ff9995"
        );
    }

    #[test]
    fn deletion_policy_encoding_preserves_reject_tag_and_separates_owned_closure() {
        let owner = OwnerKey::Module(ModuleId::migrate(b"authored-delete-policy", 1));
        let request = |policy| AuthoredChangeSet {
            base: revision(),
            preconditions: Vec::new(),
            changes: vec![AuthoredChange::DeleteOwner {
                owner: OwnerSelector::Exact { owner },
                policy,
            }],
            budget: ChangeBudget::default(),
        };
        let reject = canonical_authored_intent_bytes(&request(AuthoredDeletePolicy::Reject))
            .expect("encode reject deletion");
        let closure = canonical_authored_intent_bytes(&request(AuthoredDeletePolicy::OwnedClosure))
            .expect("encode closure deletion");
        assert_ne!(reject, closure);
        assert_eq!(reject.last(), Some(&1));
        assert_eq!(closure.last(), Some(&2));
    }

    #[test]
    fn operational_budget_is_disjoint_from_semantic_request_bytes() {
        let first = connected_request("$module", "$function", "$parameter", "$body");
        let mut changed = first.clone();
        changed.budget.canonical_reads.maximum_bytes -= 1;
        assert_eq!(
            canonical_authored_intent_bytes(&first).unwrap(),
            canonical_authored_intent_bytes(&changed).unwrap()
        );
        assert_ne!(
            encode_budget(first.budget).unwrap(),
            encode_budget(changed.budget).unwrap()
        );
    }

    #[test]
    fn same_domain_allocation_order_does_not_follow_label_sort_order() {
        let request = |first: &str, second: &str| AuthoredChangeSet {
            base: revision(),
            preconditions: Vec::new(),
            changes: vec![
                AuthoredChange::CreateModule {
                    symbol: first.to_owned(),
                    name: Name::new("first").unwrap(),
                },
                AuthoredChange::CreateModule {
                    symbol: second.to_owned(),
                    name: Name::new("second").unwrap(),
                },
            ],
            budget: ChangeBudget::default(),
        };
        assert_eq!(
            canonical_authored_intent_bytes(&request("$z_first", "$a_second")).unwrap(),
            canonical_authored_intent_bytes(&request("$a_first", "$z_second")).unwrap()
        );
    }

    #[test]
    fn exact_owner_encoding_separates_identity_domains() {
        let identity = [7_u8; 16];
        let request = |owner| AuthoredChangeSet {
            base: revision(),
            preconditions: vec![AuthoredPrecondition::OwnerExists { owner }],
            changes: Vec::new(),
            budget: ChangeBudget::default(),
        };
        let module = OwnerKey::Module(ModuleId::from_bytes(identity).unwrap());
        let declaration = OwnerKey::Declaration(DeclarationId::from_bytes(identity).unwrap());
        assert_ne!(
            canonical_authored_intent_bytes(&request(module)).unwrap(),
            canonical_authored_intent_bytes(&request(declaration)).unwrap()
        );
    }

    #[test]
    fn semantic_fields_and_ordered_lists_change_the_encoding() {
        let first = connected_request("$module", "$function", "$parameter", "$body");
        let mut renamed_owner = first.clone();
        let AuthoredChange::CreateFunction { name, .. } = &mut renamed_owner.changes[1] else {
            panic!("fixture function")
        };
        *name = Name::new("different").unwrap();
        assert_ne!(
            canonical_authored_intent_bytes(&first).unwrap(),
            canonical_authored_intent_bytes(&renamed_owner).unwrap()
        );

        let mut reordered = first.clone();
        reordered.changes.swap(0, 1);
        assert_ne!(
            canonical_authored_intent_bytes(&first).unwrap(),
            canonical_authored_intent_bytes(&reordered).unwrap()
        );
    }

    #[test]
    fn undefined_symbols_and_excessive_type_depth_fail_before_hashing() {
        let mut missing = connected_request("$module", "$function", "$parameter", "$body");
        let AuthoredChange::CreateFunction { module, .. } = &mut missing.changes[1] else {
            panic!("fixture function")
        };
        *module = ModuleSelector::Symbol {
            symbol: "$missing".to_owned(),
        };
        assert_eq!(
            canonical_authored_intent_bytes(&missing).unwrap_err().code,
            "change_authored_symbol_missing"
        );

        let mut ty = AuthoredType::Unit {};
        for _ in 0..crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            ty = AuthoredType::List { item: Box::new(ty) };
        }
        let mut deep = connected_request("$module", "$function", "$parameter", "$body");
        let AuthoredChange::CreateFunction { result, .. } = &mut deep.changes[1] else {
            panic!("fixture function")
        };
        *result = ty;
        assert_eq!(
            canonical_authored_intent_bytes(&deep).unwrap_err().code,
            "change_authored_type_depth"
        );
    }
}
