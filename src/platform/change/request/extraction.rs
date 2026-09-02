//! Identity-preserving lowering for the bounded `extract.function` authored operation.

use super::{AuthoredLowerer, DeclarationSelector, request_error};
use crate::platform::change::{
    CanonicalBaseRead, FunctionExtractionCapture, FunctionExtractionEvidence, WitnessBaseRead,
};
use crate::platform::contract::{
    MAXIMUM_FUNCTION_DEFINITION_BODY_RECORDS, MAXIMUM_FUNCTION_EXTRACTION_CAPTURE_USES,
    MAXIMUM_FUNCTION_EXTRACTION_CAPTURES, MAXIMUM_FUNCTION_EXTRACTION_REQUIREMENTS,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::contract::MAXIMUM_NAME_BYTES;
use crate::platform::kernel::{
    BindingKind, DeclarationPayload, DeclarationRecord, DeclarationReference, DependencyRecord,
    EncodedOwnerKey, ExpressionChildRole, ExpressionOperation, ExpressionRead, ExpressionRecord,
    ExpressionValidationLimits, FunctionDeclaration, FunctionEffect, LocalValueReference, Name,
    OwnerHeader, OwnerKey, OwnerKind, OwnerRecord, PackageId, PackageInterfaceDeclarationPayload,
    PackageInterfaceRecord, ParameterParent, ParameterRecord, ParameterUse, RequirementReference,
    TypeForm, TypeObject, TypeObjectDigest, encode_owner, infer_function_expression_type,
    validate_affine_roots_with_limits,
};
use crate::platform::semantic_id::{
    BindingId, DeclarationId, ExpressionId, ParameterId, encode_hex,
};
use crate::platform::witness::{
    BindingContainerRole, ExpressionRootRole, NamespaceKey, OwnershipEntry, OwnershipParent,
    OwnershipRole,
};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

const MOVED_DIGEST_DOMAIN: &str = "lkjscript.function-extraction.moved-owners.v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StructuralParent {
    Function(DeclarationId),
    Binding(BindingId),
    Expression(ExpressionId),
}

#[derive(Clone, Debug)]
struct LocalUse {
    expression: ExpressionId,
    value: LocalValueReference,
    ordinal: u64,
    selected: bool,
}

#[derive(Default)]
struct BodyInventory {
    owners: Vec<OwnerKey>,
    selected_owners: Vec<OwnerKey>,
    selected_owner_set: BTreeSet<OwnerKey>,
    expressions: BTreeMap<ExpressionId, ExpressionOperation>,
    expression_ordinals: BTreeMap<ExpressionId, u64>,
    bindings: BTreeMap<BindingId, crate::platform::kernel::BindingRecord>,
    parents: BTreeMap<ExpressionId, StructuralParent>,
    local_uses: Vec<LocalUse>,
    seen: BTreeSet<OwnerKey>,
    expression_count: u64,
}

#[derive(Clone)]
struct CaptureAnalysis {
    source: LocalValueReference,
    owner: OwnerKey,
    name: Name,
    ty: TypeObjectDigest,
    first_use: u64,
    uses: Vec<ExpressionId>,
    use_mode: ParameterUse,
    resource_requirement: Option<RequirementReference>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ResourceClass {
    None,
    Direct(DeclarationReference),
    Contained,
}

pub(super) fn lower<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    symbol: &str,
    selector: &DeclarationSelector,
    selected: ExpressionId,
    helper_name: &Name,
) -> Result<(), Diagnostic> {
    if lowerer.extraction.is_some() {
        return Err(extract_error(
            "change_extract_multiple",
            "one authored request may contain at most one function extraction",
        ));
    }
    let function_id = lowerer.resolve_declaration(selector)?;
    let function_owner = OwnerKey::Declaration(function_id);
    let function_record = accepted_existing_owner(lowerer, function_owner)?;
    let OwnerRecord::Declaration(declaration) = function_record else {
        return Err(extract_corrupt(
            "change_extract_function_record",
            "resolved function identity is bound to a foreign owner record",
        ));
    };
    let DeclarationPayload::Function(function) = declaration.payload.clone() else {
        return Err(extract_error(
            "change_extract_function_kind",
            "extract.function selector does not name a local function declaration",
        ));
    };
    if !function.type_parameters.is_empty() {
        return Err(extract_error(
            "change_extract_generic_target",
            "extract.function does not admit generic target functions",
        ));
    }
    if selected == function.body {
        return Err(extract_error(
            "change_extract_whole_body",
            "extract.function requires a proper expression subtree, not the complete function body",
        ));
    }
    require_helper_namespace_absent(lowerer, declaration.module, helper_name)?;
    let helper = lowerer.declaration_symbol(symbol)?;

    let mut inventory = BodyInventory::default();
    walk_expression(
        lowerer,
        function.body,
        StructuralParent::Function(function_id),
        OwnershipEntry::new(
            OwnershipParent::Owner(function_owner),
            OwnershipRole::ExpressionRoot(ExpressionRootRole::FunctionBody),
        ),
        selected,
        false,
        0,
        &mut inventory,
    )?;
    if inventory.selected_owners.is_empty() || !inventory.parents.contains_key(&selected) {
        return Err(extract_error(
            "change_extract_expression_foreign",
            format!(
                "expression {selected} is not a live proper structural descendant of function {function_id}"
            ),
        ));
    }
    let parent = inventory.parents.get(&selected).copied().ok_or_else(|| {
        extract_corrupt(
            "change_extract_parent_missing",
            "selected expression has no exact structural parent",
        )
    })?;

    reject_recursive_target(lowerer, function_id, &inventory)?;
    let selected_root_ordinal = inventory
        .expression_ordinals
        .get(&selected)
        .copied()
        .ok_or_else(|| {
            extract_corrupt(
                "change_extract_expression_inventory",
                "selected expression is absent from its canonical expression inventory",
            )
        })?;
    reject_escaping_bindings(&inventory)?;

    let (mut captures, result, mut requirements, analysis_read_work) = {
        let reader = CandidateRead::new(lowerer);
        let mut inference_work = 0_usize;
        let result = infer_function_expression_type(
            &reader,
            function_id,
            selected,
            &function.effect,
            &mut inference_work,
            usize::try_from(lowerer.budget.validation.maximum_expression_steps)
                .unwrap_or(usize::MAX),
        )
        .map_err(|diagnostic| {
            extract_error(
                "change_extract_result_type",
                format!(
                    "selected expression result type is not inferable: {}",
                    diagnostic.message
                ),
            )
        })?;
        if type_contains_parameter(&reader, result, &mut BTreeSet::new())? {
            return Err(extract_error(
                "change_extract_free_type",
                "selected expression result contains an unsupported free type parameter",
            ));
        }
        if resource_class(&reader, result, &mut BTreeSet::new(), &mut BTreeSet::new())?
            != ResourceClass::None
        {
            return Err(extract_error(
                "change_extract_resource_result",
                "selected expression result may not contain a capability resource",
            ));
        }
        let captures = analyze_captures(
            &reader,
            function_id,
            &function,
            selected_root_ordinal,
            &inventory,
            &mut inference_work,
            usize::try_from(lowerer.budget.validation.maximum_expression_steps)
                .unwrap_or(usize::MAX),
        )?;
        let requirements = infer_requirements(&reader, &function, &inventory, &captures)?;
        if u64::try_from(captures.len()).unwrap_or(u64::MAX) > MAXIMUM_FUNCTION_EXTRACTION_CAPTURES
        {
            return Err(extract_resource(
                "change_extract_capture_limit",
                format!(
                    "extraction capture count exceeds the {MAXIMUM_FUNCTION_EXTRACTION_CAPTURES}-capture boundary"
                ),
            ));
        }
        let capture_uses = captures.iter().try_fold(0_u64, |total, capture| {
            total.checked_add(u64::try_from(capture.uses.len()).unwrap_or(u64::MAX))
        });
        if capture_uses.is_none_or(|uses| uses > MAXIMUM_FUNCTION_EXTRACTION_CAPTURE_USES) {
            return Err(extract_resource(
                "change_extract_capture_use_limit",
                format!(
                    "extraction capture-use count exceeds the {MAXIMUM_FUNCTION_EXTRACTION_CAPTURE_USES}-use boundary"
                ),
            ));
        }
        if u64::try_from(requirements.len()).unwrap_or(u64::MAX)
            > MAXIMUM_FUNCTION_EXTRACTION_REQUIREMENTS
        {
            return Err(extract_resource(
                "change_extract_requirement_limit",
                format!(
                    "extraction requirement count exceeds the {MAXIMUM_FUNCTION_EXTRACTION_REQUIREMENTS}-requirement boundary"
                ),
            ));
        }
        (captures, result, requirements, reader.work())
    };
    lowerer.work.canonical.add(analysis_read_work);

    let resource_capture = captures
        .iter()
        .position(|capture| capture.resource_requirement.is_some());
    if let Some(index) = resource_capture {
        let capture = captures.remove(index);
        requirements.insert(capture.resource_requirement.ok_or_else(|| {
            extract_corrupt(
                "change_extract_affine_requirement",
                "resource capture lost its inferred requirement",
            )
        })?);
        captures.push(capture);
    }
    let effect = inferred_effect(&function.effect, &requirements)?;
    assign_capture_names(&mut captures)?;

    let moved_owners = canonical_owners(inventory.selected_owners.iter().copied());
    let moved_digest = moved_digest(lowerer, &moved_owners)?;
    let moved_count = u64::try_from(moved_owners.len()).map_err(|_| {
        extract_resource(
            "change_extract_moved_owners",
            "moved-owner count exceeds the platform counter domain",
        )
    })?;
    // Only the direct call and its local-read arguments belong to the caller body. The helper
    // parameters are declaration children and therefore are not function-definition body records.
    let generated_caller_body_records = u64::try_from(captures.len())
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let body_count = u64::try_from(inventory.owners.len()).unwrap_or(u64::MAX);
    let caller_body_records = body_count
        .checked_sub(moved_count)
        .and_then(|count| count.checked_add(generated_caller_body_records))
        .ok_or_else(|| {
            extract_resource(
                "change_extract_body_count",
                "post-extraction caller body count overflowed",
            )
        })?;
    if caller_body_records > MAXIMUM_FUNCTION_DEFINITION_BODY_RECORDS {
        return Err(extract_resource(
            "change_extract_caller_body_limit",
            format!(
                "post-extraction caller contains {caller_body_records} body records, exceeding the {MAXIMUM_FUNCTION_DEFINITION_BODY_RECORDS}-record inspection boundary"
            ),
        ));
    }

    lowerer.admitting_extraction = true;
    let mut capture_evidence = Vec::new();
    let mut generated_owners = Vec::new();
    let mut changed_owners = BTreeSet::new();
    let mut call_arguments = Vec::new();
    for capture in &captures {
        let parameter = lowerer.generated_parameter_identity()?;
        let argument = lowerer.expression_identity(None)?;
        lowerer.insert_created(OwnerRecord::Parameter(ParameterRecord {
            header: OwnerHeader::new(OwnerKey::Parameter(parameter), OwnerKind::Parameter),
            parent: ParameterParent::Function(helper),
            name: capture.name.clone(),
            ty: capture.ty,
            use_mode: capture.use_mode,
            resource_requirement: capture.resource_requirement,
        }))?;
        lowerer.insert_created(OwnerRecord::Expression(ExpressionRecord::new(
            argument,
            ExpressionOperation::Local {
                value: capture.source,
            },
        )?))?;
        for use_expression in &capture.uses {
            let OwnerRecord::Expression(record) =
                lowerer.candidate_mut(OwnerKey::Expression(*use_expression))?
            else {
                return Err(extract_corrupt(
                    "change_extract_capture_record",
                    "free-local use is bound to a foreign owner record",
                ));
            };
            if record.operation
                != (ExpressionOperation::Local {
                    value: capture.source,
                })
            {
                return Err(extract_corrupt(
                    "change_extract_capture_changed",
                    "free-local use changed after extraction analysis",
                ));
            }
            record.operation = ExpressionOperation::Local {
                value: LocalValueReference::FunctionParameter(parameter),
            };
            changed_owners.insert(OwnerKey::Expression(*use_expression));
        }
        call_arguments.push(argument);
        generated_owners.push(OwnerKey::Parameter(parameter));
        generated_owners.push(OwnerKey::Expression(argument));
        capture_evidence.push(FunctionExtractionCapture {
            source: capture.source,
            parameter,
            name: capture.name.clone(),
            ty: capture.ty,
            use_mode: capture.use_mode,
            resource_requirement: capture.resource_requirement,
            rewritten_uses: capture.uses.clone(),
        });
    }
    let call = lowerer.expression_identity(None)?;
    lowerer.insert_created(OwnerRecord::Expression(ExpressionRecord::new(
        call,
        ExpressionOperation::Call {
            function: DeclarationReference {
                package: lowerer.base.package_id(),
                declaration: helper,
            },
            type_arguments: Vec::new(),
            arguments: call_arguments,
        },
    )?))?;
    replace_parent_edge(lowerer, parent, selected, call)?;
    changed_owners.insert(parent_owner(parent));

    let helper_kind = match effect {
        FunctionEffect::Pure => OwnerKind::PureFunction,
        FunctionEffect::Task { .. } => OwnerKind::TaskFunction,
    };
    lowerer.insert_created(OwnerRecord::Declaration(DeclarationRecord {
        header: OwnerHeader::new(OwnerKey::Declaration(helper), helper_kind),
        module: declaration.module,
        name: helper_name.clone(),
        visibility: crate::platform::kernel::DeclarationVisibility::Private,
        payload: DeclarationPayload::Function(FunctionDeclaration {
            type_parameters: Vec::new(),
            parameters: capture_evidence
                .iter()
                .map(|capture| capture.parameter)
                .collect(),
            result,
            effect: effect.clone(),
            body: selected,
        }),
    }))?;
    generated_owners.push(OwnerKey::Expression(call));
    generated_owners.push(OwnerKey::Declaration(helper));

    if resource_capture.is_some() {
        validate_affine_candidate(lowerer, function_id, helper)?;
    }

    let changed_owners = canonical_owners(changed_owners);
    let changed_set = changed_owners.iter().copied().collect::<BTreeSet<_>>();
    let preserved_owners = canonical_owners(
        inventory
            .selected_owners
            .iter()
            .copied()
            .filter(|owner| !changed_set.contains(owner)),
    );
    generated_owners = canonical_owners(generated_owners);
    let mut protected = BTreeSet::from([function_owner]);
    protected.extend(inventory.selected_owner_set.iter().copied());
    protected.insert(parent_owner(parent));
    protected.extend(captures.iter().map(|capture| capture.owner));
    protected.extend(generated_owners.iter().copied());

    lowerer.extraction = Some(FunctionExtractionEvidence {
        base_definition: None,
        function: function_id,
        selected_root: selected,
        moved_digest,
        moved_owners,
        captures: capture_evidence,
        helper,
        helper_name: helper_name.clone(),
        result,
        effect,
        preserved_owners,
        changed_owners,
        generated_owners,
        caller_body_records,
        helper_body_records: moved_count,
    });
    lowerer.extraction_protected = protected;
    lowerer.admitting_extraction = false;
    lowerer.check_budget("function extraction rewrite")
}

fn accepted_existing_owner<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    owner: OwnerKey,
) -> Result<OwnerRecord, Diagnostic> {
    lowerer.require_owner(owner)?;
    let working = lowerer.owners.get(&owner).ok_or_else(|| {
        extract_corrupt(
            "change_extract_owner_cache",
            "extraction owner disappeared from the candidate cache",
        )
    })?;
    working.original.clone().ok_or_else(|| {
        extract_error(
            "change_extract_existing_function",
            "extract.function requires an existing accepted function",
        )
    })
}

fn require_helper_namespace_absent<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    module: crate::platform::semantic_id::ModuleId,
    name: &Name,
) -> Result<(), Diagnostic> {
    let key = NamespaceKey {
        parent: Some(OwnerKey::Module(module)),
        class: crate::platform::kernel::NamespaceClass::Declaration,
        name: name.clone(),
    };
    if !lowerer.namespace.contains_key(&key) {
        let read = lowerer.witness.read_namespace(&key)?;
        lowerer.work.witness.add(read.work);
        lowerer.namespace.insert(key.clone(), read.value);
    }
    if let Some(owner) = lowerer.namespace.get(&key).copied().flatten() {
        return Err(extract_error(
            "change_extract_helper_collision",
            format!("helper name {name} already selects owner {owner:?} in the target module"),
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn walk_expression<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    expression: ExpressionId,
    parent: StructuralParent,
    expected: OwnershipEntry,
    selected: ExpressionId,
    selected_ancestor: bool,
    depth: usize,
    inventory: &mut BodyInventory,
) -> Result<(), Diagnostic> {
    if depth > crate::platform::kernel::contract::MAXIMUM_EXPRESSION_DEPTH {
        return Err(extract_resource(
            "change_extract_depth",
            "function body exceeds the extraction structural-depth bound",
        ));
    }
    let owner = OwnerKey::Expression(expression);
    require_structural_owner(lowerer, owner, expected, inventory)?;
    let record = match lowerer
        .owners
        .get(&owner)
        .map(|working| working.record.clone())
    {
        Some(OwnerRecord::Expression(record)) if record.id == expression => record,
        Some(_) => {
            return Err(extract_corrupt(
                "change_extract_expression_record",
                "structural expression identity is bound to a foreign owner record",
            ));
        }
        None => {
            return Err(extract_corrupt(
                "change_extract_expression_cache",
                "structural expression disappeared from the candidate cache",
            ));
        }
    };
    inventory.parents.insert(expression, parent);
    inventory.expression_count = inventory.expression_count.checked_add(1).ok_or_else(|| {
        extract_resource(
            "change_extract_expression_count",
            "function expression count overflowed",
        )
    })?;
    inventory
        .expression_ordinals
        .insert(expression, inventory.expression_count);
    let inside = selected_ancestor || expression == selected;
    if inside {
        inventory.selected_owners.push(owner);
        inventory.selected_owner_set.insert(owner);
    }
    inventory.owners.push(owner);
    inventory
        .expressions
        .insert(expression, record.operation.clone());
    if let ExpressionOperation::Local { value } = record.operation {
        inventory.local_uses.push(LocalUse {
            expression,
            value,
            ordinal: inventory.expression_count,
            selected: inside,
        });
        return Ok(());
    }

    let next = depth.saturating_add(1);
    match record.operation {
        ExpressionOperation::Let { bindings, body } => {
            for (ordinal, binding) in bindings.into_iter().enumerate() {
                walk_binding(
                    lowerer,
                    binding,
                    expression,
                    BindingContainerRole::Let,
                    ordinal,
                    selected,
                    inside,
                    next,
                    inventory,
                )?;
            }
            walk_expression_child(
                lowerer,
                body,
                expression,
                ExpressionChildRole::LetBody,
                0,
                selected,
                inside,
                next,
                inventory,
            )?;
        }
        ExpressionOperation::Match { value, arms } => {
            walk_expression_child(
                lowerer,
                value,
                expression,
                ExpressionChildRole::MatchValue,
                0,
                selected,
                inside,
                next,
                inventory,
            )?;
            for (ordinal, arm) in arms.into_iter().enumerate() {
                if let Some(binding) = arm.payload_binding {
                    walk_binding(
                        lowerer,
                        binding,
                        expression,
                        BindingContainerRole::MatchPayload,
                        ordinal,
                        selected,
                        inside,
                        next,
                        inventory,
                    )?;
                }
                walk_expression_child(
                    lowerer,
                    arm.body,
                    expression,
                    ExpressionChildRole::MatchArmBody,
                    ordinal,
                    selected,
                    inside,
                    next,
                    inventory,
                )?;
            }
        }
        ExpressionOperation::Transaction { binding, body, .. } => {
            walk_binding(
                lowerer,
                binding,
                expression,
                BindingContainerRole::Transaction,
                0,
                selected,
                inside,
                next,
                inventory,
            )?;
            walk_expression_child(
                lowerer,
                body,
                expression,
                ExpressionChildRole::TransactionBody,
                0,
                selected,
                inside,
                next,
                inventory,
            )?;
        }
        operation => {
            for child in
                crate::platform::kernel::ExpressionRecord::new(expression, operation)?.children()
            {
                walk_expression_child(
                    lowerer,
                    child.expression,
                    expression,
                    child.role,
                    usize::try_from(child.ordinal).unwrap_or(usize::MAX),
                    selected,
                    inside,
                    next,
                    inventory,
                )?;
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn walk_expression_child<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    child: ExpressionId,
    parent: ExpressionId,
    role: ExpressionChildRole,
    ordinal: usize,
    selected: ExpressionId,
    inside: bool,
    depth: usize,
    inventory: &mut BodyInventory,
) -> Result<(), Diagnostic> {
    let ordinal = u32::try_from(ordinal).map_err(|_| {
        extract_resource(
            "change_extract_ordinal",
            "structural expression ordinal cannot be represented",
        )
    })?;
    walk_expression(
        lowerer,
        child,
        StructuralParent::Expression(parent),
        OwnershipEntry::new(
            OwnershipParent::Owner(OwnerKey::Expression(parent)),
            OwnershipRole::ExpressionChild { role, ordinal },
        ),
        selected,
        inside,
        depth,
        inventory,
    )
}

#[allow(clippy::too_many_arguments)]
fn walk_binding<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    binding: BindingId,
    parent: ExpressionId,
    role: BindingContainerRole,
    ordinal: usize,
    selected: ExpressionId,
    inside: bool,
    depth: usize,
    inventory: &mut BodyInventory,
) -> Result<(), Diagnostic> {
    let ordinal = u32::try_from(ordinal).map_err(|_| {
        extract_resource(
            "change_extract_ordinal",
            "structural binding ordinal cannot be represented",
        )
    })?;
    let owner = OwnerKey::Binding(binding);
    require_structural_owner(
        lowerer,
        owner,
        OwnershipEntry::new(
            OwnershipParent::Owner(OwnerKey::Expression(parent)),
            OwnershipRole::ExpressionBinding { role, ordinal },
        ),
        inventory,
    )?;
    let record = match lowerer
        .owners
        .get(&owner)
        .map(|working| working.record.clone())
    {
        Some(OwnerRecord::Binding(record)) => record,
        _ => {
            return Err(extract_corrupt(
                "change_extract_binding_record",
                "structural binding identity is bound to a foreign owner record",
            ));
        }
    };
    let expected_kind = match role {
        BindingContainerRole::Let => BindingKind::Let,
        BindingContainerRole::MatchPayload => BindingKind::MatchPayload,
        BindingContainerRole::Transaction => BindingKind::Transaction,
    };
    if record.kind != expected_kind {
        return Err(extract_corrupt(
            "change_extract_binding_kind",
            "structural binding role disagrees with its canonical binding kind",
        ));
    }
    inventory.owners.push(owner);
    if inside {
        inventory.selected_owners.push(owner);
        inventory.selected_owner_set.insert(owner);
    }
    inventory.bindings.insert(binding, record.clone());
    if let Some(value) = record.value {
        walk_expression(
            lowerer,
            value,
            StructuralParent::Binding(binding),
            OwnershipEntry::new(
                OwnershipParent::Owner(owner),
                OwnershipRole::ExpressionRoot(ExpressionRootRole::BindingValue),
            ),
            selected,
            inside,
            depth,
            inventory,
        )?;
    }
    Ok(())
}

fn require_structural_owner<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    owner: OwnerKey,
    expected: OwnershipEntry,
    inventory: &mut BodyInventory,
) -> Result<(), Diagnostic> {
    if !inventory.seen.insert(owner) {
        return Err(extract_error(
            "change_extract_structural_alias",
            format!("owner {owner:?} is structurally shared or cyclic"),
        ));
    }
    if u64::try_from(inventory.seen.len()).unwrap_or(u64::MAX)
        > MAXIMUM_FUNCTION_DEFINITION_BODY_RECORDS
    {
        return Err(extract_resource(
            "change_extract_moved_owners",
            format!(
                "function body exceeds the {MAXIMUM_FUNCTION_DEFINITION_BODY_RECORDS}-record extraction boundary"
            ),
        ));
    }
    lowerer.require_owner(owner)?;
    if !lowerer.ownership.contains_key(&owner) {
        let read = lowerer.witness.read_ownership(owner)?;
        lowerer.work.witness.add(read.work);
        lowerer.ownership.insert(owner, read.value);
    }
    if lowerer.ownership.get(&owner).copied().flatten() != Some(expected) {
        return Err(extract_corrupt(
            "change_extract_ownership",
            format!("owner {owner:?} does not have its exact expected structural parent"),
        ));
    }
    lowerer.work.ownership_steps = lowerer.work.ownership_steps.saturating_add(1);
    lowerer.check_budget("function extraction ownership traversal")
}

fn reject_escaping_bindings(inventory: &BodyInventory) -> Result<(), Diagnostic> {
    for local_use in &inventory.local_uses {
        if local_use.selected {
            continue;
        }
        let owner = local_owner(local_use.value);
        if matches!(owner, OwnerKey::Binding(_)) && inventory.selected_owner_set.contains(&owner) {
            return Err(extract_error(
                "change_extract_binding_escape",
                format!("binding {owner:?} defined inside the selected subtree escapes it"),
            ));
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn analyze_captures<B: CanonicalBaseRead + ?Sized>(
    reader: &CandidateRead<'_, B>,
    function_id: DeclarationId,
    function: &FunctionDeclaration,
    selected_root_ordinal: u64,
    inventory: &BodyInventory,
    inference_work: &mut usize,
    maximum_steps: usize,
) -> Result<Vec<CaptureAnalysis>, Diagnostic> {
    let function_parameters = function.parameters.iter().copied().collect::<BTreeSet<_>>();
    let mut uses = BTreeMap::<LocalValueReference, Vec<&LocalUse>>::new();
    for local_use in &inventory.local_uses {
        if local_use.selected {
            uses.entry(local_use.value).or_default().push(local_use);
        }
    }
    let mut captures = Vec::new();
    for (source, selected_uses) in uses {
        let owner = local_owner(source);
        if inventory.selected_owner_set.contains(&owner) {
            continue;
        }
        let (name, ty, parameter_record) = match source {
            LocalValueReference::FunctionParameter(parameter)
                if function_parameters.contains(&parameter) =>
            {
                let record = read_parameter(reader, parameter)?;
                if record.parent != ParameterParent::Function(function_id) {
                    return Err(extract_corrupt(
                        "change_extract_capture_scope",
                        "captured function parameter belongs to another declaration",
                    ));
                }
                (record.name.clone(), record.ty, Some(record))
            }
            LocalValueReference::LexicalBinding(binding)
            | LocalValueReference::MatchPayload(binding) => {
                let record = inventory.bindings.get(&binding).ok_or_else(|| {
                    extract_error(
                        "change_extract_capture_foreign",
                        format!("free local binding {binding} is not owned by the target function"),
                    )
                })?;
                let expected = match source {
                    LocalValueReference::LexicalBinding(_) => BindingKind::Let,
                    LocalValueReference::MatchPayload(_) => BindingKind::MatchPayload,
                    _ => BindingKind::Transaction,
                };
                if record.kind != expected {
                    return Err(extract_corrupt(
                        "change_extract_capture_kind",
                        "free-local reference kind disagrees with its binding record",
                    ));
                }
                let ty = match (record.declared_type, record.value) {
                    (Some(ty), _) => ty,
                    (None, Some(value)) => infer_function_expression_type(
                        reader,
                        function_id,
                        value,
                        &function.effect,
                        inference_work,
                        maximum_steps,
                    )?,
                    (None, None) => {
                        return Err(extract_error(
                            "change_extract_capture_type",
                            "free local binding has no exact declared or inferable type",
                        ));
                    }
                };
                (record.name.clone(), ty, None)
            }
            LocalValueReference::TransactionBinding(_) => {
                return Err(extract_error(
                    "change_extract_transaction_capture",
                    "a live transaction binding cannot cross an extracted function boundary",
                ));
            }
            LocalValueReference::OperationParameter(_) => {
                return Err(extract_error(
                    "change_extract_capture_foreign",
                    "an operation parameter is not a local function capture",
                ));
            }
            LocalValueReference::FunctionParameter(_) => {
                return Err(extract_error(
                    "change_extract_capture_foreign",
                    "free function parameter is not owned by the target function",
                ));
            }
        };
        if type_contains_parameter(reader, ty, &mut BTreeSet::new())? {
            return Err(extract_error(
                "change_extract_free_type",
                format!("capture {owner:?} contains an unsupported free type parameter"),
            ));
        }
        let class = resource_class(reader, ty, &mut BTreeSet::new(), &mut BTreeSet::new())?;
        let (use_mode, resource_requirement) = match class {
            ResourceClass::None => (ParameterUse::Unrestricted, None),
            ResourceClass::Contained => {
                return Err(extract_error(
                    "change_extract_resource_container",
                    format!("capture {owner:?} contains a resource in an unsupported shape"),
                ));
            }
            ResourceClass::Direct(interface) => {
                let requirement = capture_resource_requirement(
                    reader,
                    source,
                    parameter_record.as_ref(),
                    function,
                    inventory,
                    interface,
                )?;
                require_resource_capture_shape(
                    reader,
                    source,
                    requirement,
                    interface,
                    selected_root_ordinal,
                    &selected_uses,
                    inventory,
                )?;
                (ParameterUse::Consume, Some(requirement))
            }
        };
        captures.push(CaptureAnalysis {
            source,
            owner,
            name,
            ty,
            first_use: selected_uses[0].ordinal,
            uses: selected_uses
                .iter()
                .map(|local_use| local_use.expression)
                .collect(),
            use_mode,
            resource_requirement,
        });
    }
    if captures
        .iter()
        .filter(|capture| capture.resource_requirement.is_some())
        .count()
        > 1
    {
        return Err(extract_error(
            "change_extract_multiple_resources",
            "selected subtree has more than one free capability resource",
        ));
    }
    captures.sort_by(|left, right| {
        left.first_use
            .cmp(&right.first_use)
            .then_with(|| EncodedOwnerKey::new(left.owner).cmp(&EncodedOwnerKey::new(right.owner)))
    });
    Ok(captures)
}

fn read_parameter<B: CanonicalBaseRead + ?Sized>(
    reader: &CandidateRead<'_, B>,
    parameter: ParameterId,
) -> Result<ParameterRecord, Diagnostic> {
    match reader.owner(OwnerKey::Parameter(parameter))? {
        Some(OwnerRecord::Parameter(record)) => Ok(record),
        _ => Err(extract_corrupt(
            "change_extract_capture_parameter",
            "captured parameter identity is missing or bound to another owner kind",
        )),
    }
}

fn capture_resource_requirement<B: CanonicalBaseRead + ?Sized>(
    reader: &CandidateRead<'_, B>,
    source: LocalValueReference,
    parameter: Option<&ParameterRecord>,
    function: &FunctionDeclaration,
    inventory: &BodyInventory,
    interface: DeclarationReference,
) -> Result<RequirementReference, Diagnostic> {
    if let Some(parameter) = parameter {
        if parameter.use_mode != ParameterUse::Consume {
            return Err(extract_error(
                "change_extract_resource_source",
                "captured capability parameter is not the current consume-only affine shape",
            ));
        }
        return parameter.resource_requirement.ok_or_else(|| {
            extract_error(
                "change_extract_resource_provenance",
                "captured capability parameter has no exact acquiring requirement",
            )
        });
    }
    let binding = match source {
        LocalValueReference::LexicalBinding(binding)
        | LocalValueReference::MatchPayload(binding) => binding,
        _ => {
            return Err(extract_error(
                "change_extract_resource_provenance",
                "captured capability binding has unsupported affine provenance",
            ));
        }
    };
    let record = inventory.bindings.get(&binding).ok_or_else(|| {
        extract_corrupt(
            "change_extract_resource_binding",
            "captured resource binding is absent from the target body",
        )
    })?;
    if matches!(source, LocalValueReference::MatchPayload(_)) {
        let FunctionEffect::Task { requirements } = &function.effect else {
            return Err(extract_error(
                "change_extract_resource_provenance",
                "captured match payload has no task requirement provenance",
            ));
        };
        let mut candidates = Vec::new();
        for requirement in requirements {
            if read_requirement_interface(reader, *requirement)? == interface {
                candidates.push(*requirement);
            }
        }
        return match candidates.as_slice() {
            [requirement] => Ok(*requirement),
            [] => Err(extract_error(
                "change_extract_resource_provenance",
                "captured match payload has no exact caller requirement for its resource interface",
            )),
            _ => Err(extract_error(
                "change_extract_resource_ambiguity",
                "captured match payload has more than one caller requirement for its resource interface",
            )),
        };
    }
    let Some(value) = record.value else {
        return Err(extract_error(
            "change_extract_resource_provenance",
            "captured capability binding has no exact acquiring expression",
        ));
    };
    match reader.owner(OwnerKey::Expression(value))? {
        Some(OwnerRecord::Expression(ExpressionRecord {
            operation: ExpressionOperation::CapabilityCall { requirement, .. },
            ..
        })) => Ok(requirement),
        _ => Err(extract_error(
            "change_extract_resource_provenance",
            "captured capability binding is not acquired by one exact capability call",
        )),
    }
}

fn read_requirement_interface<B: CanonicalBaseRead + ?Sized>(
    reader: &CandidateRead<'_, B>,
    requirement: RequirementReference,
) -> Result<DeclarationReference, Diagnostic> {
    if requirement.package == reader.package_id() {
        return match reader.owner(OwnerKey::Requirement(requirement.requirement))? {
            Some(OwnerRecord::Requirement(record)) => Ok(record.interface),
            _ => Err(extract_error(
                "change_extract_resource_requirement",
                "caller effect names no exact local resource requirement",
            )),
        };
    }
    match reader.package_interface_owner(
        requirement.package,
        OwnerKey::Requirement(requirement.requirement),
    )? {
        Some(PackageInterfaceRecord::Requirement(record)) => Ok(record.interface),
        _ => Err(extract_error(
            "change_extract_resource_requirement",
            "caller effect names no exact dependency resource requirement",
        )),
    }
}

fn require_resource_capture_shape<B: CanonicalBaseRead + ?Sized>(
    reader: &CandidateRead<'_, B>,
    source: LocalValueReference,
    requirement: RequirementReference,
    interface: DeclarationReference,
    selected_root_ordinal: u64,
    selected_uses: &[&LocalUse],
    inventory: &BodyInventory,
) -> Result<(), Diagnostic> {
    if selected_uses.len() != 1 {
        return Err(extract_error(
            "change_extract_resource_use",
            "free capability resource must have exactly one use inside the selected subtree",
        ));
    }
    if inventory.local_uses.iter().any(|local_use| {
        !local_use.selected
            && local_use.value == source
            && local_use.ordinal > selected_root_ordinal
    }) {
        return Err(extract_error(
            "change_extract_resource_post_use",
            "free capability resource has a use after the extracted call position",
        ));
    }
    let requirement_record = if requirement.package == reader.package_id() {
        match reader.owner(OwnerKey::Requirement(requirement.requirement))? {
            Some(OwnerRecord::Requirement(record)) => record,
            _ => {
                return Err(extract_error(
                    "change_extract_resource_requirement",
                    "captured capability names no exact local acquiring requirement",
                ));
            }
        }
    } else {
        match reader.package_interface_owner(
            requirement.package,
            OwnerKey::Requirement(requirement.requirement),
        )? {
            Some(PackageInterfaceRecord::Requirement(record)) => record,
            _ => {
                return Err(extract_error(
                    "change_extract_resource_requirement",
                    "captured capability names no exact dependency acquiring requirement",
                ));
            }
        }
    };
    if requirement_record.interface != interface {
        return Err(extract_error(
            "change_extract_resource_requirement",
            "capability resource interface disagrees with its exact acquiring requirement",
        ));
    }
    Ok(())
}

fn infer_requirements<B: CanonicalBaseRead + ?Sized>(
    reader: &CandidateRead<'_, B>,
    function: &FunctionDeclaration,
    inventory: &BodyInventory,
    captures: &[CaptureAnalysis],
) -> Result<BTreeSet<RequirementReference>, Diagnostic> {
    let mut requirements = BTreeSet::new();
    for owner in &inventory.selected_owners {
        let OwnerKey::Expression(expression) = owner else {
            continue;
        };
        let operation = inventory.expressions.get(expression).ok_or_else(|| {
            extract_corrupt(
                "change_extract_effect_inventory",
                "selected expression disappeared from the effect inventory",
            )
        })?;
        match operation {
            ExpressionOperation::CapabilityCall { requirement, .. }
            | ExpressionOperation::Transaction { requirement, .. } => {
                requirements.insert(*requirement);
            }
            ExpressionOperation::Call { function, .. } => {
                match referenced_function_effect(reader, *function)? {
                    FunctionEffect::Pure => {}
                    FunctionEffect::Task {
                        requirements: called,
                    } => requirements.extend(called),
                }
            }
            ExpressionOperation::FunctionValue { .. } | ExpressionOperation::Invoke { .. } => {
                return Err(extract_error(
                    "change_extract_closure",
                    "function values and indirect invocation are outside the extraction boundary",
                ));
            }
            _ => {}
        }
    }
    requirements.extend(
        captures
            .iter()
            .filter_map(|capture| capture.resource_requirement),
    );
    let available = match &function.effect {
        FunctionEffect::Pure => BTreeSet::new(),
        FunctionEffect::Task { requirements } => requirements.iter().copied().collect(),
    };
    if let Some(missing) = requirements
        .iter()
        .find(|requirement| !available.contains(requirement))
    {
        return Err(extract_error(
            "change_extract_missing_requirement",
            format!("selected subtree requires unavailable task requirement {missing:?}"),
        ));
    }
    Ok(requirements)
}

fn referenced_function_effect<B: CanonicalBaseRead + ?Sized>(
    reader: &CandidateRead<'_, B>,
    reference: DeclarationReference,
) -> Result<FunctionEffect, Diagnostic> {
    if reference.package == reader.package_id() {
        return match reader.owner(OwnerKey::Declaration(reference.declaration))? {
            Some(OwnerRecord::Declaration(DeclarationRecord {
                payload: DeclarationPayload::Function(function),
                ..
            })) => Ok(function.effect),
            Some(OwnerRecord::Declaration(DeclarationRecord {
                payload: DeclarationPayload::External(_),
                ..
            })) => Ok(FunctionEffect::Pure),
            _ => Err(extract_error(
                "change_extract_call_kind",
                "selected direct call does not name an exact local function",
            )),
        };
    }
    match reader.package_interface_owner(
        reference.package,
        OwnerKey::Declaration(reference.declaration),
    )? {
        Some(PackageInterfaceRecord::Declaration(declaration)) => match declaration.payload {
            PackageInterfaceDeclarationPayload::Function(function) => Ok(function.effect),
            PackageInterfaceDeclarationPayload::External(_) => Ok(FunctionEffect::Pure),
            _ => Err(extract_error(
                "change_extract_call_kind",
                "selected direct call does not name an exact dependency function",
            )),
        },
        _ => Err(extract_error(
            "change_extract_call_missing",
            "selected direct call names no exact dependency function",
        )),
    }
}

fn inferred_effect(
    caller: &FunctionEffect,
    required: &BTreeSet<RequirementReference>,
) -> Result<FunctionEffect, Diagnostic> {
    if required.is_empty() {
        return Ok(FunctionEffect::Pure);
    }
    let FunctionEffect::Task {
        requirements: caller_requirements,
    } = caller
    else {
        return Err(extract_error(
            "change_extract_missing_requirement",
            "pure target cannot supply the selected subtree task requirements",
        ));
    };
    Ok(FunctionEffect::Task {
        requirements: caller_requirements
            .iter()
            .copied()
            .filter(|requirement| required.contains(requirement))
            .collect(),
    })
}

fn assign_capture_names(captures: &mut [CaptureAnalysis]) -> Result<(), Diagnostic> {
    let mut used = BTreeSet::new();
    for capture in captures {
        if used.insert(capture.name.clone()) {
            continue;
        }
        let suffix = encode_hex(&EncodedOwnerKey::new(capture.owner).bytes());
        let prefix_length = capture
            .name
            .as_str()
            .len()
            .min(MAXIMUM_NAME_BYTES.saturating_sub(suffix.len().saturating_add(1)));
        let candidate = format!("{}-{suffix}", &capture.name.as_str()[..prefix_length]);
        let candidate = Name::new(candidate).map_err(|diagnostic| {
            extract_corrupt(
                "change_extract_capture_name",
                format!(
                    "identity-derived capture name is invalid: {}",
                    diagnostic.message
                ),
            )
        })?;
        if !used.insert(candidate.clone()) {
            return Err(extract_corrupt(
                "change_extract_capture_name",
                "identity-derived capture name is not unique",
            ));
        }
        capture.name = candidate;
    }
    Ok(())
}

fn moved_digest<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &AuthoredLowerer<'_, B, W>,
    owners: &[OwnerKey],
) -> Result<[u8; 32], Diagnostic> {
    let mut hasher = blake3::Hasher::new_derive_key(MOVED_DIGEST_DOMAIN);
    hasher.update(
        &u64::try_from(owners.len())
            .unwrap_or(u64::MAX)
            .to_be_bytes(),
    );
    for owner in owners {
        let record = lowerer.owners.get(owner).ok_or_else(|| {
            extract_corrupt(
                "change_extract_moved_digest",
                "moved owner disappeared before digest derivation",
            )
        })?;
        let (_, bytes) = encode_owner(&record.record)?;
        hasher.update(&EncodedOwnerKey::new(*owner).bytes());
        hasher.update(&u64::try_from(bytes.len()).unwrap_or(u64::MAX).to_be_bytes());
        hasher.update(&bytes);
    }
    Ok(*hasher.finalize().as_bytes())
}

fn replace_parent_edge<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    parent: StructuralParent,
    selected: ExpressionId,
    call: ExpressionId,
) -> Result<(), Diagnostic> {
    match parent {
        StructuralParent::Function(_) => Err(extract_error(
            "change_extract_whole_body",
            "extract.function cannot replace the complete function body",
        )),
        StructuralParent::Binding(binding) => {
            let OwnerRecord::Binding(record) = lowerer.candidate_mut(OwnerKey::Binding(binding))?
            else {
                return Err(extract_corrupt(
                    "change_extract_parent_kind",
                    "selected expression binding parent has a foreign owner kind",
                ));
            };
            if record.value != Some(selected) {
                return Err(extract_corrupt(
                    "change_extract_parent_edge",
                    "selected expression is not the exact binding-value root",
                ));
            }
            record.value = Some(call);
            Ok(())
        }
        StructuralParent::Expression(parent) => {
            let OwnerRecord::Expression(record) =
                lowerer.candidate_mut(OwnerKey::Expression(parent))?
            else {
                return Err(extract_corrupt(
                    "change_extract_parent_kind",
                    "selected expression parent has a foreign owner kind",
                ));
            };
            replace_expression_reference(&mut record.operation, selected, call)
        }
    }
}

fn replace_expression_reference(
    operation: &mut ExpressionOperation,
    selected: ExpressionId,
    call: ExpressionId,
) -> Result<(), Diagnostic> {
    let mut count = 0_u64;
    let mut replace = |value: &mut ExpressionId| {
        if *value == selected {
            *value = call;
            count = count.saturating_add(1);
        }
    };
    match operation {
        ExpressionOperation::If {
            condition,
            when_true,
            when_false,
        } => {
            replace(condition);
            replace(when_true);
            replace(when_false);
        }
        ExpressionOperation::Let { body, .. }
        | ExpressionOperation::Field { value: body, .. }
        | ExpressionOperation::Transaction { body, .. } => replace(body),
        ExpressionOperation::Sequence { items }
        | ExpressionOperation::List { items, .. }
        | ExpressionOperation::Call {
            arguments: items, ..
        }
        | ExpressionOperation::CapabilityCall {
            arguments: items, ..
        } => items.iter_mut().for_each(&mut replace),
        ExpressionOperation::Invoke { callee, arguments } => {
            replace(callee);
            arguments.iter_mut().for_each(&mut replace);
        }
        ExpressionOperation::Record { fields, .. } => {
            fields
                .iter_mut()
                .for_each(|field| replace(&mut field.value));
        }
        ExpressionOperation::Variant { payload, .. } => {
            if let Some(payload) = payload {
                replace(payload);
            }
        }
        ExpressionOperation::Map { entries, .. } => entries.iter_mut().for_each(|entry| {
            replace(&mut entry.key);
            replace(&mut entry.value);
        }),
        ExpressionOperation::Match { value, arms } => {
            replace(value);
            arms.iter_mut().for_each(|arm| replace(&mut arm.body));
        }
        ExpressionOperation::Unit {}
        | ExpressionOperation::Bool { .. }
        | ExpressionOperation::I64 { .. }
        | ExpressionOperation::Text { .. }
        | ExpressionOperation::StaticText { .. }
        | ExpressionOperation::Local { .. }
        | ExpressionOperation::Constant { .. }
        | ExpressionOperation::FunctionValue { .. } => {}
    }
    if count != 1 {
        return Err(extract_corrupt(
            "change_extract_parent_edge",
            format!("selected expression occurs {count} times in its asserted parent"),
        ));
    }
    Ok(())
}

fn reject_recursive_target<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    function: DeclarationId,
    _inventory: &BodyInventory,
) -> Result<(), Diagnostic> {
    let (result, read_work) = {
        let reader = CandidateRead::new(lowerer);
        let mut visiting = BTreeSet::new();
        let mut complete = BTreeSet::new();
        let mut expression_work = 0_u64;
        let result = require_acyclic_local_call_graph(
            &reader,
            function,
            &mut visiting,
            &mut complete,
            &mut expression_work,
            lowerer.budget.validation.maximum_expression_steps,
        );
        (result, reader.work())
    };
    lowerer.work.canonical.add(read_work);
    result
}

fn require_acyclic_local_call_graph<B: CanonicalBaseRead + ?Sized>(
    reader: &CandidateRead<'_, B>,
    function: DeclarationId,
    visiting: &mut BTreeSet<DeclarationId>,
    complete: &mut BTreeSet<DeclarationId>,
    expression_work: &mut u64,
    maximum_expression_work: u64,
) -> Result<(), Diagnostic> {
    if complete.contains(&function) {
        return Ok(());
    }
    if !visiting.insert(function) {
        return Err(extract_error(
            "change_extract_recursive_target",
            "extract.function does not admit a target with a recursive local call cycle",
        ));
    }
    let record = reader
        .owner(OwnerKey::Declaration(function))?
        .ok_or_else(|| {
            extract_corrupt(
                "change_extract_call_graph_function",
                "local call-graph declaration disappeared during extraction analysis",
            )
        })?;
    let OwnerRecord::Declaration(DeclarationRecord { payload, .. }) = record else {
        return Err(extract_corrupt(
            "change_extract_call_graph_function",
            "local call-graph identity is bound to another owner kind",
        ));
    };
    let body = match payload {
        DeclarationPayload::Function(function) => function.body,
        DeclarationPayload::External(_) => {
            visiting.remove(&function);
            complete.insert(function);
            return Ok(());
        }
        _ => {
            return Err(extract_corrupt(
                "change_extract_call_graph_function",
                "direct call names a local declaration that is not callable",
            ));
        }
    };
    let mut calls = BTreeSet::new();
    collect_local_calls(
        reader,
        body,
        &mut BTreeSet::new(),
        &mut calls,
        expression_work,
        maximum_expression_work,
        0,
    )?;
    for called in calls {
        require_acyclic_local_call_graph(
            reader,
            called,
            visiting,
            complete,
            expression_work,
            maximum_expression_work,
        )?;
    }
    visiting.remove(&function);
    complete.insert(function);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn collect_local_calls<B: CanonicalBaseRead + ?Sized>(
    reader: &CandidateRead<'_, B>,
    expression: ExpressionId,
    seen: &mut BTreeSet<OwnerKey>,
    calls: &mut BTreeSet<DeclarationId>,
    work: &mut u64,
    maximum_work: u64,
    depth: usize,
) -> Result<(), Diagnostic> {
    if depth > crate::platform::kernel::contract::MAXIMUM_EXPRESSION_DEPTH {
        return Err(extract_resource(
            "change_extract_call_graph_depth",
            "local call-graph expression traversal exceeds the structural-depth bound",
        ));
    }
    *work = work.checked_add(1).ok_or_else(|| {
        extract_resource(
            "change_extract_call_graph_work",
            "local call-graph expression work overflowed",
        )
    })?;
    if *work > maximum_work {
        return Err(extract_resource(
            "change_extract_call_graph_work",
            format!(
                "local call-graph traversal exceeds the {maximum_work}-expression request budget"
            ),
        ));
    }
    let owner = OwnerKey::Expression(expression);
    if !seen.insert(owner) {
        return Err(extract_corrupt(
            "change_extract_call_graph_alias",
            "local call-graph body is structurally shared or cyclic",
        ));
    }
    let Some(OwnerRecord::Expression(record)) = reader.owner(owner)? else {
        return Err(extract_corrupt(
            "change_extract_call_graph_expression",
            "local call-graph expression is missing or bound to another owner kind",
        ));
    };
    if let ExpressionOperation::Call {
        function: reference,
        ..
    } = record.operation
        && reference.package == reader.package_id()
    {
        calls.insert(reference.declaration);
    }
    let next = depth.saturating_add(1);
    match record.operation {
        ExpressionOperation::Let { bindings, body } => {
            for binding in bindings {
                collect_binding_local_calls(
                    reader,
                    binding,
                    seen,
                    calls,
                    work,
                    maximum_work,
                    next,
                )?;
            }
            collect_local_calls(reader, body, seen, calls, work, maximum_work, next)?;
        }
        ExpressionOperation::Match { value, arms } => {
            collect_local_calls(reader, value, seen, calls, work, maximum_work, next)?;
            for arm in arms {
                if let Some(binding) = arm.payload_binding {
                    collect_binding_local_calls(
                        reader,
                        binding,
                        seen,
                        calls,
                        work,
                        maximum_work,
                        next,
                    )?;
                }
                collect_local_calls(reader, arm.body, seen, calls, work, maximum_work, next)?;
            }
        }
        ExpressionOperation::Transaction { binding, body, .. } => {
            collect_binding_local_calls(reader, binding, seen, calls, work, maximum_work, next)?;
            collect_local_calls(reader, body, seen, calls, work, maximum_work, next)?;
        }
        operation => {
            for child in ExpressionRecord::new(expression, operation)?.children() {
                collect_local_calls(
                    reader,
                    child.expression,
                    seen,
                    calls,
                    work,
                    maximum_work,
                    next,
                )?;
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn collect_binding_local_calls<B: CanonicalBaseRead + ?Sized>(
    reader: &CandidateRead<'_, B>,
    binding: BindingId,
    seen: &mut BTreeSet<OwnerKey>,
    calls: &mut BTreeSet<DeclarationId>,
    work: &mut u64,
    maximum_work: u64,
    depth: usize,
) -> Result<(), Diagnostic> {
    let owner = OwnerKey::Binding(binding);
    if !seen.insert(owner) {
        return Err(extract_corrupt(
            "change_extract_call_graph_alias",
            "local call-graph binding is structurally shared or cyclic",
        ));
    }
    let Some(OwnerRecord::Binding(record)) = reader.owner(owner)? else {
        return Err(extract_corrupt(
            "change_extract_call_graph_binding",
            "local call-graph binding is missing or bound to another owner kind",
        ));
    };
    if let Some(value) = record.value {
        collect_local_calls(reader, value, seen, calls, work, maximum_work, depth)?;
    }
    Ok(())
}

fn validate_affine_candidate<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    function: DeclarationId,
    helper: DeclarationId,
) -> Result<(), Diagnostic> {
    let (result, read_work) = {
        let reader = CandidateRead::new(lowerer);
        let mut diagnostics = Vec::new();
        let mut work = 0_usize;
        let result = validate_affine_roots_with_limits(
            &reader,
            [
                OwnerKey::Declaration(function),
                OwnerKey::Declaration(helper),
            ],
            &mut diagnostics,
            &mut work,
            ExpressionValidationLimits {
                maximum_steps: usize::try_from(lowerer.budget.validation.maximum_expression_steps)
                    .unwrap_or(usize::MAX),
                maximum_diagnostics: 1,
            },
        );
        let result = result.map_err(|_| {
            extract_resource(
                "change_extract_affine_work",
                "affine extraction validation exhausted its explicit work bound",
            )
        });
        let result = match (result, diagnostics.into_iter().next()) {
            (Ok(()), None) => Ok(()),
            (Ok(()), Some(diagnostic)) | (Err(_), Some(diagnostic)) => Err(extract_error(
                "change_extract_affine_shape",
                format!(
                    "affine capture cannot cross the selected boundary: {}",
                    diagnostic.message
                ),
            )),
            (Err(diagnostic), None) => Err(diagnostic),
        };
        (result, reader.work())
    };
    lowerer.work.canonical.add(read_work);
    result
}

fn type_contains_parameter<B: CanonicalBaseRead + ?Sized>(
    reader: &CandidateRead<'_, B>,
    digest: TypeObjectDigest,
    active: &mut BTreeSet<TypeObjectDigest>,
) -> Result<bool, Diagnostic> {
    if !active.insert(digest) {
        return Ok(false);
    }
    let object = reader.type_object(digest)?.ok_or_else(|| {
        extract_corrupt(
            "change_extract_type_missing",
            format!("type object {digest} is missing during extraction analysis"),
        )
    })?;
    let result = match object.form {
        TypeForm::TypeParameter { .. } => true,
        _ => {
            let mut found = false;
            for child in object.child_types() {
                if type_contains_parameter(reader, child, active)? {
                    found = true;
                    break;
                }
            }
            found
        }
    };
    active.remove(&digest);
    Ok(result)
}

fn resource_class<B: CanonicalBaseRead + ?Sized>(
    reader: &CandidateRead<'_, B>,
    digest: TypeObjectDigest,
    active_types: &mut BTreeSet<TypeObjectDigest>,
    active_declarations: &mut BTreeSet<(PackageId, DeclarationId)>,
) -> Result<ResourceClass, Diagnostic> {
    if !active_types.insert(digest) {
        return Ok(ResourceClass::None);
    }
    let object = reader.type_object(digest)?.ok_or_else(|| {
        extract_corrupt(
            "change_extract_type_missing",
            format!("type object {digest} is missing during resource analysis"),
        )
    })?;
    let result = match object.form {
        TypeForm::CapabilityResource { interface } => ResourceClass::Direct(interface),
        TypeForm::Named { declaration } => {
            let key = (declaration.package, declaration.declaration);
            if !active_declarations.insert(key) {
                ResourceClass::None
            } else {
                let mut contained = false;
                for member in named_member_types(reader, declaration)? {
                    if resource_class(reader, member, active_types, active_declarations)?
                        != ResourceClass::None
                    {
                        contained = true;
                        break;
                    }
                }
                active_declarations.remove(&key);
                if contained {
                    ResourceClass::Contained
                } else {
                    ResourceClass::None
                }
            }
        }
        _ => {
            let mut contained = false;
            for child in object.child_types() {
                if resource_class(reader, child, active_types, active_declarations)?
                    != ResourceClass::None
                {
                    contained = true;
                    break;
                }
            }
            if contained {
                ResourceClass::Contained
            } else {
                ResourceClass::None
            }
        }
    };
    active_types.remove(&digest);
    Ok(result)
}

fn named_member_types<B: CanonicalBaseRead + ?Sized>(
    reader: &CandidateRead<'_, B>,
    reference: DeclarationReference,
) -> Result<Vec<TypeObjectDigest>, Diagnostic> {
    let payload = if reference.package == reader.package_id() {
        match reader.owner(OwnerKey::Declaration(reference.declaration))? {
            Some(OwnerRecord::Declaration(record)) => match record.payload {
                DeclarationPayload::Record { fields } => {
                    return fields
                        .into_iter()
                        .map(|field| match reader.owner(OwnerKey::Field(field))? {
                            Some(OwnerRecord::Field(record)) => Ok(record.ty),
                            _ => Err(extract_corrupt(
                                "change_extract_named_member",
                                "named record field is missing during resource analysis",
                            )),
                        })
                        .collect();
                }
                DeclarationPayload::Variant { cases } => {
                    return cases
                        .into_iter()
                        .map(|case| match reader.owner(OwnerKey::Case(case))? {
                            Some(OwnerRecord::Case(record)) => Ok(record.payload),
                            _ => Err(extract_corrupt(
                                "change_extract_named_member",
                                "named variant case is missing during resource analysis",
                            )),
                        })
                        .collect::<Result<Vec<_>, _>>()
                        .map(|values| values.into_iter().flatten().collect());
                }
                _ => return Ok(Vec::new()),
            },
            _ => {
                return Err(extract_corrupt(
                    "change_extract_named_type",
                    "named type declaration is missing during resource analysis",
                ));
            }
        }
    } else {
        match reader.package_interface_owner(
            reference.package,
            OwnerKey::Declaration(reference.declaration),
        )? {
            Some(PackageInterfaceRecord::Declaration(record)) => record.payload,
            _ => {
                return Err(extract_corrupt(
                    "change_extract_named_type",
                    "dependency named type is missing during resource analysis",
                ));
            }
        }
    };
    match payload {
        PackageInterfaceDeclarationPayload::Record { fields } => fields
            .into_iter()
            .map(|field| {
                match reader.package_interface_owner(reference.package, OwnerKey::Field(field))? {
                    Some(PackageInterfaceRecord::Field(record)) => Ok(record.ty),
                    _ => Err(extract_corrupt(
                        "change_extract_named_member",
                        "dependency record field is missing during resource analysis",
                    )),
                }
            })
            .collect(),
        PackageInterfaceDeclarationPayload::Variant { cases } => cases
            .into_iter()
            .map(|case| {
                match reader.package_interface_owner(reference.package, OwnerKey::Case(case))? {
                    Some(PackageInterfaceRecord::Case(record)) => Ok(record.payload),
                    _ => Err(extract_corrupt(
                        "change_extract_named_member",
                        "dependency variant case is missing during resource analysis",
                    )),
                }
            })
            .collect::<Result<Vec<_>, _>>()
            .map(|values| values.into_iter().flatten().collect()),
        _ => Ok(Vec::new()),
    }
}

fn local_owner(value: LocalValueReference) -> OwnerKey {
    match value {
        LocalValueReference::FunctionParameter(parameter)
        | LocalValueReference::OperationParameter(parameter) => OwnerKey::Parameter(parameter),
        LocalValueReference::LexicalBinding(binding)
        | LocalValueReference::MatchPayload(binding)
        | LocalValueReference::TransactionBinding(binding) => OwnerKey::Binding(binding),
    }
}

fn parent_owner(parent: StructuralParent) -> OwnerKey {
    match parent {
        StructuralParent::Function(function) => OwnerKey::Declaration(function),
        StructuralParent::Binding(binding) => OwnerKey::Binding(binding),
        StructuralParent::Expression(expression) => OwnerKey::Expression(expression),
    }
}

fn canonical_owners(owners: impl IntoIterator<Item = OwnerKey>) -> Vec<OwnerKey> {
    let mut owners = owners.into_iter().collect::<Vec<_>>();
    owners.sort_unstable_by_key(|owner| EncodedOwnerKey::new(*owner));
    owners.dedup();
    owners
}

struct CandidateRead<'a, B: ?Sized> {
    base: &'a B,
    package: PackageId,
    owners: RefCell<BTreeMap<OwnerKey, Option<OwnerRecord>>>,
    types: RefCell<BTreeMap<TypeObjectDigest, Option<TypeObject>>>,
    dependencies: RefCell<BTreeMap<PackageId, Option<DependencyRecord>>>,
    package_interfaces: RefCell<BTreeMap<(PackageId, OwnerKey), Option<PackageInterfaceRecord>>>,
    work: Cell<crate::platform::change::CanonicalReadWork>,
}

impl<'a, B: CanonicalBaseRead + ?Sized> CandidateRead<'a, B> {
    fn new<W: WitnessBaseRead + ?Sized>(lowerer: &AuthoredLowerer<'a, B, W>) -> Self {
        let owners = lowerer
            .owners
            .iter()
            .map(|(owner, working)| (*owner, (!working.deleted).then_some(working.record.clone())))
            .collect();
        let mut types = lowerer.base_types.clone();
        types.extend(
            lowerer
                .types
                .clone()
                .into_objects()
                .into_iter()
                .map(|(digest, object)| (digest, Some(object))),
        );
        let dependencies = lowerer
            .dependencies
            .iter()
            .map(|(package, working)| (*package, working.record.clone()))
            .collect();
        Self {
            base: lowerer.base,
            package: lowerer.base.package_id(),
            owners: RefCell::new(owners),
            types: RefCell::new(types),
            dependencies: RefCell::new(dependencies),
            package_interfaces: RefCell::new(BTreeMap::new()),
            work: Cell::new(crate::platform::change::CanonicalReadWork::default()),
        }
    }

    fn add_work(&self, observed: crate::platform::change::CanonicalReadWork) {
        let mut work = self.work.get();
        work.add(observed);
        self.work.set(work);
    }

    fn work(&self) -> crate::platform::change::CanonicalReadWork {
        self.work.get()
    }
}

impl<B: CanonicalBaseRead + ?Sized> ExpressionRead for CandidateRead<'_, B> {
    fn package_id(&self) -> PackageId {
        self.package
    }

    fn owner(&self, owner: OwnerKey) -> Result<Option<OwnerRecord>, Diagnostic> {
        if !self.owners.borrow().contains_key(&owner) {
            let read = self.base.read_owner(owner)?;
            self.add_work(read.work);
            self.owners.borrow_mut().insert(owner, read.value);
        }
        Ok(self.owners.borrow().get(&owner).cloned().flatten())
    }

    fn type_object(&self, digest: TypeObjectDigest) -> Result<Option<TypeObject>, Diagnostic> {
        if !self.types.borrow().contains_key(&digest) {
            let read = self.base.read_type_object(digest)?;
            self.add_work(read.work);
            self.types.borrow_mut().insert(digest, read.value);
        }
        Ok(self.types.borrow().get(&digest).cloned().flatten())
    }

    fn package_interface_owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<Option<PackageInterfaceRecord>, Diagnostic> {
        let key = (package, owner);
        if !self.package_interfaces.borrow().contains_key(&key) {
            let dependency = self.dependency(package)?.ok_or_else(|| {
                extract_error(
                    "change_extract_dependency_missing",
                    format!("package {package} is not bound by the exact base"),
                )
            })?;
            let read = self.base.read_package_interface_owner(&dependency, owner)?;
            self.add_work(read.work);
            self.package_interfaces.borrow_mut().insert(key, read.value);
        }
        Ok(self
            .package_interfaces
            .borrow()
            .get(&key)
            .cloned()
            .flatten())
    }

    fn has_dependency(&self, package: PackageId) -> Result<bool, Diagnostic> {
        Ok(self.dependency(package)?.is_some())
    }
}

impl<B: CanonicalBaseRead + ?Sized> CandidateRead<'_, B> {
    fn dependency(&self, package: PackageId) -> Result<Option<DependencyRecord>, Diagnostic> {
        if !self.dependencies.borrow().contains_key(&package) {
            let read = self.base.read_dependency(package)?;
            self.add_work(read.work);
            self.dependencies.borrow_mut().insert(package, read.value);
        }
        Ok(self.dependencies.borrow().get(&package).cloned().flatten())
    }
}

fn extract_error(code: &'static str, message: impl Into<String>) -> Diagnostic {
    request_error(DiagnosticClass::Semantic, code, message)
}

fn extract_resource(code: &'static str, message: impl Into<String>) -> Diagnostic {
    request_error(DiagnosticClass::Resource, code, message)
}

fn extract_corrupt(code: &'static str, message: impl Into<String>) -> Diagnostic {
    request_error(DiagnosticClass::Corrupt, code, message)
}
