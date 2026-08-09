use std::collections::{HashMap, HashSet};

use lkjscript_core::{Error, Result};

use crate::hir::{
    BindingId, BindingKind, Expr, ExprKind, MatchLocal, MatchPattern, MatchPlan, MatchPlanId,
    PlaceId,
};

use super::program::SemanticProgram;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum RootKey {
    Main,
    Function(BindingId),
}

struct Locations {
    slots: HashMap<BindingId, usize>,
    places: HashMap<BindingId, PlaceId>,
    local_count: usize,
}

/// Compact all compiler-dense semantic identities after a staged structural edit.
///
/// The returned map describes old binding placement to new binding placement and
/// is consumed by workspace identity reconciliation. Downstream compiler IR is
/// derived only after this pass.
pub(super) fn compact(program: &mut SemanticProgram) -> Result<HashMap<BindingId, BindingId>> {
    let mut plan_roots = HashMap::new();
    plan_roots
        .try_reserve(program.match_plans.len())
        .map_err(|_| Error::host("match-plan owner allocation failed"))?;
    if let Some(main) = &program.main {
        collect_plan_roots(&main.body, RootKey::Main, &mut plan_roots)?;
    }
    for function in &program.functions {
        collect_plan_roots(
            &function.body,
            RootKey::Function(function.binding),
            &mut plan_roots,
        )?;
    }

    let mut plan_map = HashMap::new();
    plan_map
        .try_reserve(plan_roots.len())
        .map_err(|_| Error::host("match-plan remap allocation failed"))?;
    let mut retained_plan_ids = Vec::new();
    retained_plan_ids
        .try_reserve(plan_roots.len())
        .map_err(|_| Error::host("retained match-plan allocation failed"))?;
    for plan in &program.match_plans {
        if plan_roots.contains_key(&plan.id) {
            let raw = u64::try_from(retained_plan_ids.len())
                .map_err(|_| Error::host("match-plan identity exceeds u64"))?;
            let new = MatchPlanId::new(raw);
            if plan_map.insert(plan.id, new).is_some() {
                return Err(Error::msg("match-plan identity is duplicated"));
            }
            retained_plan_ids.push(plan.id);
        }
    }
    if retained_plan_ids.len() != plan_roots.len() {
        return Err(Error::msg(
            "semantic expression references a stale match plan",
        ));
    }

    let mut live_bindings = HashSet::new();
    live_bindings
        .try_reserve(program.bindings.len())
        .map_err(|_| Error::host("live binding allocation failed"))?;
    for binding in &program.bindings {
        if matches!(binding.kind, BindingKind::BuiltinOperation(_)) {
            live_bindings.insert(binding.id);
        }
    }
    if let Some(main) = &program.main {
        live_bindings.extend(main.params.iter().copied());
        collect_expression_definitions(&main.body, &mut live_bindings)?;
    }
    for function in &program.functions {
        live_bindings.insert(function.binding);
        live_bindings.extend(function.params.iter().copied());
        collect_expression_definitions(&function.body, &mut live_bindings)?;
    }
    for old in &retained_plan_ids {
        let plan = plan(program, *old)?;
        collect_plan_bindings(plan, &mut live_bindings)?;
    }

    let mut binding_map = HashMap::new();
    binding_map
        .try_reserve(live_bindings.len())
        .map_err(|_| Error::host("binding remap allocation failed"))?;
    let mut bindings = Vec::new();
    bindings
        .try_reserve(live_bindings.len())
        .map_err(|_| Error::host("binding compaction allocation failed"))?;
    for binding in &program.bindings {
        if !live_bindings.contains(&binding.id) {
            continue;
        }
        let raw = u64::try_from(bindings.len())
            .map_err(|_| Error::host("binding identity exceeds u64"))?;
        let new = BindingId::new(raw);
        if binding_map.insert(binding.id, new).is_some() {
            return Err(Error::msg("binding identity is duplicated"));
        }
        let mut binding = binding.clone();
        binding.id = new;
        bindings.push(binding);
    }
    if bindings.len() != live_bindings.len() {
        return Err(Error::msg(
            "semantic program references a stale binding definition",
        ));
    }

    let mut locations = HashMap::new();
    locations
        .try_reserve(program.functions.len().saturating_add(1))
        .map_err(|_| Error::host("root location allocation failed"))?;
    if let Some(main) = &program.main {
        locations.insert(
            RootKey::Main,
            build_locations(
                program,
                &main.body,
                &main.params,
                &main.param_places,
                main.local_count,
                &plan_roots,
                RootKey::Main,
            )?,
        );
    }
    for function in &program.functions {
        let root = RootKey::Function(function.binding);
        locations.insert(
            root,
            build_locations(
                program,
                &function.body,
                &function.params,
                &function.param_places,
                function.local_count,
                &plan_roots,
                root,
            )?,
        );
    }

    let mut match_plans = Vec::new();
    match_plans
        .try_reserve(retained_plan_ids.len())
        .map_err(|_| Error::host("match-plan compaction allocation failed"))?;
    for old in retained_plan_ids {
        let root = plan_roots
            .get(&old)
            .copied()
            .ok_or_else(|| Error::msg("match plan has no surviving owner"))?;
        let root_locations = locations
            .get(&root)
            .ok_or_else(|| Error::msg("match-plan owner locations are missing"))?;
        match_plans.push(remap_plan(
            plan(program, old)?,
            *plan_map
                .get(&old)
                .ok_or_else(|| Error::msg("match-plan remap is incomplete"))?,
            &binding_map,
            root_locations,
        )?);
    }

    if let Some(main) = &mut program.main {
        let root_locations = locations
            .get(&RootKey::Main)
            .ok_or_else(|| Error::msg("main locations are missing"))?;
        main.body = main.body.try_remap_dense_ids(
            &binding_map,
            &root_locations.slots,
            &root_locations.places,
            &plan_map,
        )?;
        main.param_places = main
            .params
            .iter()
            .map(|binding| {
                root_locations
                    .places
                    .get(binding)
                    .copied()
                    .ok_or_else(|| Error::msg("main parameter place remap is incomplete"))
            })
            .collect::<Result<Vec<_>>>()?;
        for parameter in &mut main.params {
            *parameter = remap_binding(&binding_map, *parameter)?;
        }
        main.local_count = root_locations.local_count;
    }
    for function in &mut program.functions {
        let old_binding = function.binding;
        let root_locations = locations
            .get(&RootKey::Function(old_binding))
            .ok_or_else(|| Error::msg("function locations are missing"))?;
        function.body = function.body.try_remap_dense_ids(
            &binding_map,
            &root_locations.slots,
            &root_locations.places,
            &plan_map,
        )?;
        function.param_places = function
            .params
            .iter()
            .map(|binding| {
                root_locations
                    .places
                    .get(binding)
                    .copied()
                    .ok_or_else(|| Error::msg("function parameter place remap is incomplete"))
            })
            .collect::<Result<Vec<_>>>()?;
        function.binding = remap_binding(&binding_map, old_binding)?;
        for parameter in &mut function.params {
            *parameter = remap_binding(&binding_map, *parameter)?;
        }
        function.local_count = root_locations.local_count;
    }
    for binding in &mut program.global_layout {
        *binding = remap_binding(&binding_map, *binding)?;
    }

    program.bindings = bindings;
    program.match_plans = match_plans;
    Ok(binding_map)
}

fn collect_plan_roots(
    root: &Expr,
    owner: RootKey,
    plans: &mut HashMap<MatchPlanId, RootKey>,
) -> Result<()> {
    let mut pending = Vec::new();
    pending
        .try_reserve(1)
        .map_err(|_| Error::host("match-plan traversal allocation failed"))?;
    pending.push(root);
    while let Some(expression) = pending.pop() {
        if let ExprKind::Match { plan, .. } | ExprKind::MatchUnreachable { plan } = &expression.kind
        {
            match plans.insert(*plan, owner) {
                None => {}
                Some(previous) if previous == owner => {
                    return Err(Error::msg("match plan has more than one semantic site"));
                }
                Some(_) => return Err(Error::msg("match plan is shared by callable roots")),
            }
        }
        push_expression_children(&mut pending, expression, "match-plan traversal")?;
    }
    Ok(())
}

fn collect_expression_definitions(root: &Expr, bindings: &mut HashSet<BindingId>) -> Result<()> {
    let mut pending = Vec::new();
    pending
        .try_reserve(1)
        .map_err(|_| Error::host("binding traversal allocation failed"))?;
    pending.push(root);
    while let Some(expression) = pending.pop() {
        match &expression.kind {
            ExprKind::Let {
                bindings: locals, ..
            } => {
                bindings.extend(locals.iter().map(|local| local.binding));
            }
            ExprKind::MutableLocal { binding, .. } => {
                bindings.insert(*binding);
            }
            _ => {}
        }
        push_expression_children(&mut pending, expression, "binding traversal")?;
    }
    Ok(())
}

fn push_expression_children<'a>(
    pending: &mut Vec<&'a Expr>,
    expression: &'a Expr,
    context: &str,
) -> Result<()> {
    let mut allocation_failed = false;
    crate::hir::for_each_expression_child(expression, &mut |child| {
        if allocation_failed {
            return;
        }
        if pending.try_reserve(1).is_err() {
            allocation_failed = true;
        } else {
            pending.push(child);
        }
    });
    if allocation_failed {
        Err(Error::host(format!("{context} work allocation failed")))
    } else {
        Ok(())
    }
}

fn collect_plan_bindings(plan: &MatchPlan, bindings: &mut HashSet<BindingId>) -> Result<()> {
    bindings.insert(plan.scrutinee.binding);
    for arm in &plan.arms {
        collect_pattern_locals(&arm.pattern, &mut |local| {
            bindings.insert(local.binding);
            Ok(())
        })?;
    }
    bindings.extend(plan.projections.iter().map(|item| item.local.binding));
    bindings.extend(plan.bindings.iter().map(|item| item.local.binding));
    Ok(())
}

fn build_locations(
    program: &SemanticProgram,
    expression: &Expr,
    parameters: &[BindingId],
    parameter_places: &[PlaceId],
    local_capacity: usize,
    plan_roots: &HashMap<MatchPlanId, RootKey>,
    root: RootKey,
) -> Result<Locations> {
    if parameters.len() != parameter_places.len() {
        return Err(Error::msg("callable parameter places are inconsistent"));
    }
    let mut facts = HashMap::new();
    facts
        .try_reserve(local_capacity)
        .map_err(|_| Error::host("local location allocation failed"))?;
    let mut place_order = Vec::new();
    place_order
        .try_reserve(local_capacity)
        .map_err(|_| Error::host("local place-order allocation failed"))?;
    for (slot, (binding, place)) in parameters.iter().zip(parameter_places).enumerate() {
        record_location(&mut facts, &mut place_order, *binding, slot, *place)?;
    }
    collect_expression_locations(
        program,
        expression,
        plan_roots,
        root,
        &mut facts,
        &mut place_order,
    )?;

    let mut slot_map = Vec::new();
    slot_map
        .try_reserve(local_capacity)
        .map_err(|_| Error::host("local slot compaction allocation failed"))?;
    slot_map.resize(local_capacity, None);
    for (slot, _) in facts.values() {
        let entry = slot_map
            .get_mut(*slot)
            .ok_or_else(|| Error::msg("local slot exceeds the callable layout"))?;
        *entry = Some(0);
    }
    let mut local_count = 0_usize;
    for entry in &mut slot_map {
        if entry.is_some() {
            *entry = Some(local_count);
            local_count = local_count
                .checked_add(1)
                .ok_or_else(|| Error::host("local slot count overflow"))?;
        }
    }
    let mut place_map = HashMap::new();
    place_map
        .try_reserve(place_order.len())
        .map_err(|_| Error::host("local place compaction allocation failed"))?;
    for (new, old) in place_order.into_iter().enumerate() {
        let compact =
            PlaceId::new(u64::try_from(new).map_err(|_| Error::host("local place exceeds u64"))?);
        if place_map.insert(old, compact).is_some() {
            return Err(Error::msg(
                "local place identity is reused by distinct bindings",
            ));
        }
    }
    if place_map.len() != facts.len() {
        return Err(Error::msg(
            "local place declaration order is incomplete or duplicated",
        ));
    }
    let mut slots = HashMap::new();
    let mut places = HashMap::new();
    slots
        .try_reserve(facts.len())
        .map_err(|_| Error::host("local binding slot allocation failed"))?;
    places
        .try_reserve(facts.len())
        .map_err(|_| Error::host("local binding place allocation failed"))?;
    for (binding, (slot, place)) in facts {
        slots.insert(
            binding,
            slot_map
                .get(slot)
                .copied()
                .flatten()
                .ok_or_else(|| Error::msg("local slot compaction is incomplete"))?,
        );
        places.insert(
            binding,
            *place_map
                .get(&place)
                .ok_or_else(|| Error::msg("local place compaction is incomplete"))?,
        );
    }
    Ok(Locations {
        slots,
        places,
        local_count,
    })
}

#[derive(Clone, Copy)]
struct LocationFact {
    binding: BindingId,
    slot: usize,
    place: PlaceId,
}

enum LocationWork<'a> {
    Visit(&'a Expr),
    Define(LocationFact),
}

fn collect_expression_locations(
    program: &SemanticProgram,
    expression: &Expr,
    plan_roots: &HashMap<MatchPlanId, RootKey>,
    root: RootKey,
    facts: &mut HashMap<BindingId, (usize, PlaceId)>,
    place_order: &mut Vec<PlaceId>,
) -> Result<()> {
    let mut work = Vec::new();
    work.try_reserve(1)
        .map_err(|_| Error::host("local location traversal allocation failed"))?;
    work.push(LocationWork::Visit(expression));
    while let Some(item) = work.pop() {
        match item {
            LocationWork::Define(local) => {
                record_location(facts, place_order, local.binding, local.slot, local.place)?
            }
            LocationWork::Visit(expression) => match &expression.kind {
                ExprKind::Let { bindings, body } => {
                    let additional = bindings
                        .len()
                        .checked_mul(2)
                        .and_then(|count| count.checked_add(1))
                        .ok_or_else(|| Error::host("local traversal work count overflow"))?;
                    work.try_reserve(additional)
                        .map_err(|_| Error::host("local location traversal allocation failed"))?;
                    work.push(LocationWork::Visit(body));
                    for local in bindings.iter().rev() {
                        work.push(LocationWork::Define(LocationFact {
                            binding: local.binding,
                            slot: local.slot,
                            place: local.place,
                        }));
                        work.push(LocationWork::Visit(&local.value));
                    }
                }
                ExprKind::MutableLocal {
                    binding,
                    place,
                    slot,
                    initial,
                    body,
                } => {
                    work.try_reserve(3)
                        .map_err(|_| Error::host("local location traversal allocation failed"))?;
                    work.push(LocationWork::Visit(body));
                    work.push(LocationWork::Visit(initial));
                    work.push(LocationWork::Define(LocationFact {
                        binding: *binding,
                        slot: *slot,
                        place: *place,
                    }));
                }
                ExprKind::Match {
                    plan: id,
                    scrutinee,
                    arms,
                } => {
                    if plan_roots.get(id).copied() != Some(root) {
                        return Err(Error::msg("match-plan callable ownership is stale"));
                    }
                    let plan = plan(program, *id)?;
                    if arms.len() != plan.arms.len() {
                        return Err(Error::msg("semantic match arm count is stale"));
                    }
                    for (body, arm) in arms.iter().zip(&plan.arms).rev() {
                        work.try_reserve(1).map_err(|_| {
                            Error::host("match location traversal allocation failed")
                        })?;
                        work.push(LocationWork::Visit(body));
                        let locals = pattern_location_order(&arm.pattern)?;
                        work.try_reserve(locals.len()).map_err(|_| {
                            Error::host("match location traversal allocation failed")
                        })?;
                        work.extend(locals.into_iter().rev().map(LocationWork::Define));
                    }
                    work.try_reserve(2)
                        .map_err(|_| Error::host("match location traversal allocation failed"))?;
                    work.push(LocationWork::Define(match_location(&plan.scrutinee)));
                    work.push(LocationWork::Visit(scrutinee));
                }
                ExprKind::MatchUnreachable { plan: id } => {
                    if plan_roots.get(id).copied() != Some(root) {
                        return Err(Error::msg("match-plan callable ownership is stale"));
                    }
                    let plan = plan(program, *id)?;
                    let mut locals = Vec::new();
                    locals
                        .try_reserve(1)
                        .map_err(|_| Error::host("match location allocation failed"))?;
                    locals.push(match_location(&plan.scrutinee));
                    for arm in &plan.arms {
                        let pattern = pattern_location_order(&arm.pattern)?;
                        locals
                            .try_reserve(pattern.len())
                            .map_err(|_| Error::host("match location allocation failed"))?;
                        locals.extend(pattern);
                    }
                    work.try_reserve(locals.len())
                        .map_err(|_| Error::host("match location traversal allocation failed"))?;
                    work.extend(locals.into_iter().rev().map(LocationWork::Define));
                }
                _ => push_expression_location_children(&mut work, expression)?,
            },
        }
    }
    Ok(())
}

fn push_expression_location_children<'a>(
    work: &mut Vec<LocationWork<'a>>,
    expression: &'a Expr,
) -> Result<()> {
    let mut children = Vec::new();
    let mut allocation_failed = false;
    crate::hir::for_each_expression_child(expression, &mut |child| {
        if !allocation_failed && children.try_reserve(1).is_err() {
            allocation_failed = true;
        }
        if !allocation_failed {
            children.push(child);
        }
    });
    if allocation_failed {
        return Err(Error::host("local expression-child allocation failed"));
    }
    work.try_reserve(children.len())
        .map_err(|_| Error::host("local location traversal allocation failed"))?;
    work.extend(children.into_iter().rev().map(LocationWork::Visit));
    Ok(())
}

fn pattern_location_order(root: &MatchPattern) -> Result<Vec<LocationFact>> {
    enum PatternWork<'a> {
        Visit(&'a MatchPattern),
        Define(&'a MatchLocal),
    }

    let mut work = Vec::new();
    work.try_reserve(1)
        .map_err(|_| Error::host("match location work allocation failed"))?;
    work.push(PatternWork::Visit(root));
    let mut result = Vec::new();
    while let Some(item) = work.pop() {
        match item {
            PatternWork::Define(local) => {
                result
                    .try_reserve(1)
                    .map_err(|_| Error::host("match location allocation failed"))?;
                result.push(match_location(local));
            }
            PatternWork::Visit(MatchPattern::Binding { local }) => {
                result
                    .try_reserve(1)
                    .map_err(|_| Error::host("match location allocation failed"))?;
                result.push(match_location(local));
            }
            PatternWork::Visit(MatchPattern::Variant { fields, .. })
            | PatternWork::Visit(MatchPattern::Product { fields, .. }) => {
                let additional = fields
                    .len()
                    .checked_mul(2)
                    .ok_or_else(|| Error::host("match location work count overflow"))?;
                work.try_reserve(additional)
                    .map_err(|_| Error::host("match location work allocation failed"))?;
                for field in fields.iter().rev() {
                    work.push(PatternWork::Visit(&field.pattern));
                    if let Some(local) = &field.projection {
                        work.push(PatternWork::Define(local));
                    }
                }
            }
            PatternWork::Visit(_) => {}
        }
    }
    Ok(result)
}

fn match_location(local: &MatchLocal) -> LocationFact {
    LocationFact {
        binding: local.binding,
        slot: local.slot,
        place: local.place,
    }
}

fn record_location(
    facts: &mut HashMap<BindingId, (usize, PlaceId)>,
    place_order: &mut Vec<PlaceId>,
    binding: BindingId,
    slot: usize,
    place: PlaceId,
) -> Result<()> {
    match facts.get(&binding).copied() {
        Some(previous) if previous == (slot, place) => Ok(()),
        Some(_) => Err(Error::msg("local binding has conflicting location facts")),
        None => {
            facts
                .try_reserve(1)
                .map_err(|_| Error::host("local location allocation failed"))?;
            place_order
                .try_reserve(1)
                .map_err(|_| Error::host("local place-order allocation failed"))?;
            facts.insert(binding, (slot, place));
            place_order.push(place);
            Ok(())
        }
    }
}

fn collect_pattern_locals(
    root: &MatchPattern,
    action: &mut impl FnMut(&MatchLocal) -> Result<()>,
) -> Result<()> {
    let mut pending = Vec::new();
    pending
        .try_reserve(1)
        .map_err(|_| Error::host("match-pattern traversal allocation failed"))?;
    pending.push(root);
    while let Some(pattern) = pending.pop() {
        match pattern {
            MatchPattern::Binding { local } => action(local)?,
            MatchPattern::Variant { fields, .. } | MatchPattern::Product { fields, .. } => {
                pending
                    .try_reserve(fields.len())
                    .map_err(|_| Error::host("match-pattern traversal allocation failed"))?;
                for field in fields {
                    if let Some(local) = &field.projection {
                        action(local)?;
                    }
                    pending.push(&field.pattern);
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn remap_plan(
    plan: &MatchPlan,
    id: MatchPlanId,
    bindings: &HashMap<BindingId, BindingId>,
    locations: &Locations,
) -> Result<MatchPlan> {
    let mut arms = Vec::new();
    arms.try_reserve(plan.arms.len())
        .map_err(|_| Error::host("match arm remap allocation failed"))?;
    for arm in &plan.arms {
        arms.push(crate::hir::PlannedMatchArm {
            id: arm.id,
            pattern: arm.pattern.try_remap_dense_ids(
                bindings,
                &locations.slots,
                &locations.places,
            )?,
            body_type: arm.body_type.clone(),
        });
    }
    let mut projections = plan.projections.clone();
    for item in &mut projections {
        item.local = remap_local(&item.local, bindings, locations)?;
    }
    let mut assignments = plan.bindings.clone();
    for item in &mut assignments {
        item.local = remap_local(&item.local, bindings, locations)?;
    }
    Ok(MatchPlan {
        id,
        origin: plan.origin,
        scrutinee: remap_local(&plan.scrutinee, bindings, locations)?,
        result_type: plan.result_type.clone(),
        arms,
        tests: plan.tests.clone(),
        projections,
        bindings: assignments,
        edges: plan.edges.clone(),
        exhaustive: plan.exhaustive,
        witness: plan.witness.clone(),
    })
}

fn remap_local(
    local: &MatchLocal,
    bindings: &HashMap<BindingId, BindingId>,
    locations: &Locations,
) -> Result<MatchLocal> {
    Ok(MatchLocal {
        binding: remap_binding(bindings, local.binding)?,
        place: locations
            .places
            .get(&local.binding)
            .copied()
            .ok_or_else(|| Error::msg("match-local place remap is incomplete"))?,
        slot: locations
            .slots
            .get(&local.binding)
            .copied()
            .ok_or_else(|| Error::msg("match-local slot remap is incomplete"))?,
        ty: local.ty.clone(),
    })
}

fn remap_binding(bindings: &HashMap<BindingId, BindingId>, old: BindingId) -> Result<BindingId> {
    bindings
        .get(&old)
        .copied()
        .ok_or_else(|| Error::msg("binding remap is incomplete"))
}

fn plan(program: &SemanticProgram, id: MatchPlanId) -> Result<&MatchPlan> {
    id.index()
        .and_then(|index| program.match_plans.get(index))
        .filter(|plan| plan.id == id)
        .ok_or_else(|| Error::msg("match-plan identity is stale"))
}
