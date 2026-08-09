use std::collections::HashMap;

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MatchPlanId(u64);

impl MatchPlanId {
    pub(crate) const fn new(raw: u64) -> Self {
        Self(raw)
    }
    pub const fn raw(self) -> u64 {
        self.0
    }
    pub(crate) fn index(self) -> Option<usize> {
        usize::try_from(self.0).ok()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchLocal {
    pub binding: BindingId,
    pub place: PlaceId,
    pub slot: usize,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchFieldPattern {
    pub name: String,
    pub field_index: u64,
    pub projection: Option<MatchLocal>,
    pub pattern: MatchPattern,
}

#[derive(Debug)]
pub enum MatchPattern {
    Wildcard {
        ty: Type,
    },
    Binding {
        local: MatchLocal,
    },
    Bool(bool),
    I64(i64),
    Variant {
        ty: Type,
        enum_id: EnumId,
        variant: VariantId,
        layout: RuntimeLayoutId,
        fields: Vec<MatchFieldPattern>,
    },
    Product {
        ty: Type,
        product: ProductId,
        fields: Vec<MatchFieldPattern>,
    },
}

impl MatchPattern {
    pub fn ty(&self) -> Type {
        match self {
            Self::Wildcard { ty } | Self::Variant { ty, .. } | Self::Product { ty, .. } => {
                ty.clone()
            }
            Self::Binding { local } => local.ty.clone(),
            Self::Bool(_) => Type::Bool,
            Self::I64(_) => Type::I64,
        }
    }

    pub(crate) fn try_remap_dense_ids(
        &self,
        bindings: &HashMap<BindingId, BindingId>,
        local_slots: &HashMap<BindingId, usize>,
        local_places: &HashMap<BindingId, PlaceId>,
    ) -> lkjscript_core::Result<Self> {
        enum Work<'a> {
            Visit(&'a MatchPattern),
            Variant {
                ty: &'a Type,
                enum_id: EnumId,
                variant: VariantId,
                layout: RuntimeLayoutId,
                fields: &'a [MatchFieldPattern],
            },
            Product {
                ty: &'a Type,
                product: ProductId,
                fields: &'a [MatchFieldPattern],
            },
        }

        let mut work = Vec::new();
        work.try_reserve(1)
            .map_err(|_| lkjscript_core::Error::host("match remap work allocation failed"))?;
        work.push(Work::Visit(self));
        let mut completed = Vec::new();
        while let Some(item) = work.pop() {
            match item {
                Work::Visit(pattern) => match pattern {
                    MatchPattern::Wildcard { ty } => {
                        completed.push(MatchPattern::Wildcard { ty: ty.clone() });
                    }
                    MatchPattern::Binding { local } => completed.push(MatchPattern::Binding {
                        local: remap_match_local(local, bindings, local_slots, local_places)?,
                    }),
                    MatchPattern::Bool(value) => completed.push(MatchPattern::Bool(*value)),
                    MatchPattern::I64(value) => completed.push(MatchPattern::I64(*value)),
                    MatchPattern::Variant {
                        ty,
                        enum_id,
                        variant,
                        layout,
                        fields,
                    } => {
                        work.try_reserve(fields.len().checked_add(1).ok_or_else(|| {
                            lkjscript_core::Error::host("match remap work count overflow")
                        })?)
                        .map_err(|_| {
                            lkjscript_core::Error::host("match remap work allocation failed")
                        })?;
                        work.push(Work::Variant {
                            ty,
                            enum_id: *enum_id,
                            variant: *variant,
                            layout: *layout,
                            fields,
                        });
                        work.extend(fields.iter().rev().map(|field| Work::Visit(&field.pattern)));
                    }
                    MatchPattern::Product {
                        ty,
                        product,
                        fields,
                    } => {
                        work.try_reserve(fields.len().checked_add(1).ok_or_else(|| {
                            lkjscript_core::Error::host("match remap work count overflow")
                        })?)
                        .map_err(|_| {
                            lkjscript_core::Error::host("match remap work allocation failed")
                        })?;
                        work.push(Work::Product {
                            ty,
                            product: *product,
                            fields,
                        });
                        work.extend(fields.iter().rev().map(|field| Work::Visit(&field.pattern)));
                    }
                },
                Work::Variant {
                    ty,
                    enum_id,
                    variant,
                    layout,
                    fields,
                } => {
                    let start = completed.len().checked_sub(fields.len()).ok_or_else(|| {
                        lkjscript_core::Error::msg("match remap completion order is invalid")
                    })?;
                    let patterns: Vec<_> = completed.drain(start..).collect();
                    let mut remapped_fields = Vec::new();
                    remapped_fields.try_reserve(fields.len()).map_err(|_| {
                        lkjscript_core::Error::host("match remap field allocation failed")
                    })?;
                    for (field, pattern) in fields.iter().zip(patterns) {
                        remapped_fields.push(MatchFieldPattern {
                            name: field.name.clone(),
                            field_index: field.field_index,
                            projection: field
                                .projection
                                .as_ref()
                                .map(|local| {
                                    remap_match_local(local, bindings, local_slots, local_places)
                                })
                                .transpose()?,
                            pattern,
                        });
                    }
                    completed.push(MatchPattern::Variant {
                        ty: ty.clone(),
                        enum_id,
                        variant,
                        layout,
                        fields: remapped_fields,
                    });
                }
                Work::Product {
                    ty,
                    product,
                    fields,
                } => {
                    let start = completed.len().checked_sub(fields.len()).ok_or_else(|| {
                        lkjscript_core::Error::msg("match remap completion order is invalid")
                    })?;
                    let patterns: Vec<_> = completed.drain(start..).collect();
                    let mut remapped_fields = Vec::new();
                    remapped_fields.try_reserve(fields.len()).map_err(|_| {
                        lkjscript_core::Error::host("match remap field allocation failed")
                    })?;
                    for (field, pattern) in fields.iter().zip(patterns) {
                        remapped_fields.push(MatchFieldPattern {
                            name: field.name.clone(),
                            field_index: field.field_index,
                            projection: field
                                .projection
                                .as_ref()
                                .map(|local| {
                                    remap_match_local(local, bindings, local_slots, local_places)
                                })
                                .transpose()?,
                            pattern,
                        });
                    }
                    completed.push(MatchPattern::Product {
                        ty: ty.clone(),
                        product,
                        fields: remapped_fields,
                    });
                }
            }
        }
        let result = completed
            .pop()
            .ok_or_else(|| lkjscript_core::Error::msg("match remap omitted its root"))?;
        if completed.is_empty() {
            Ok(result)
        } else {
            Err(lkjscript_core::Error::msg(
                "match remap left disconnected results",
            ))
        }
    }
}

fn remap_match_local(
    local: &MatchLocal,
    bindings: &HashMap<BindingId, BindingId>,
    local_slots: &HashMap<BindingId, usize>,
    local_places: &HashMap<BindingId, PlaceId>,
) -> lkjscript_core::Result<MatchLocal> {
    Ok(MatchLocal {
        binding: bindings
            .get(&local.binding)
            .copied()
            .ok_or_else(|| lkjscript_core::Error::msg("match binding remap is incomplete"))?,
        place: local_places
            .get(&local.binding)
            .copied()
            .ok_or_else(|| lkjscript_core::Error::msg("match place remap is incomplete"))?,
        slot: local_slots
            .get(&local.binding)
            .copied()
            .ok_or_else(|| lkjscript_core::Error::msg("match slot remap is incomplete"))?,
        ty: local.ty.clone(),
    })
}

impl PartialEq for MatchPattern {
    fn eq(&self, other: &Self) -> bool {
        let mut pending = vec![(self, other)];
        while let Some((left, right)) = pending.pop() {
            match (left, right) {
                (MatchPattern::Wildcard { ty: left }, MatchPattern::Wildcard { ty: right })
                    if left == right => {}
                (MatchPattern::Binding { local: left }, MatchPattern::Binding { local: right })
                    if left == right => {}
                (MatchPattern::Bool(left), MatchPattern::Bool(right)) if left == right => {}
                (MatchPattern::I64(left), MatchPattern::I64(right)) if left == right => {}
                (
                    MatchPattern::Variant {
                        ty: left_ty,
                        enum_id: left_enum,
                        variant: left_variant,
                        layout: left_layout,
                        fields: left_fields,
                    },
                    MatchPattern::Variant {
                        ty: right_ty,
                        enum_id: right_enum,
                        variant: right_variant,
                        layout: right_layout,
                        fields: right_fields,
                    },
                ) if left_ty == right_ty
                    && left_enum == right_enum
                    && left_variant == right_variant
                    && left_layout == right_layout
                    && fields_equal(left_fields, right_fields, &mut pending) => {}
                (
                    MatchPattern::Product {
                        ty: left_ty,
                        product: left_product,
                        fields: left_fields,
                    },
                    MatchPattern::Product {
                        ty: right_ty,
                        product: right_product,
                        fields: right_fields,
                    },
                ) if left_ty == right_ty
                    && left_product == right_product
                    && fields_equal(left_fields, right_fields, &mut pending) => {}
                _ => return false,
            }
        }
        true
    }
}

impl Eq for MatchPattern {}

fn fields_equal<'a>(
    left: &'a [MatchFieldPattern],
    right: &'a [MatchFieldPattern],
    pending: &mut Vec<(&'a MatchPattern, &'a MatchPattern)>,
) -> bool {
    if left.len() != right.len() {
        return false;
    }
    for (left, right) in left.iter().zip(right) {
        if left.name != right.name
            || left.field_index != right.field_index
            || left.projection != right.projection
        {
            return false;
        }
        pending.push((&left.pattern, &right.pattern));
    }
    true
}

impl Clone for MatchPattern {
    fn clone(&self) -> Self {
        enum Work<'a> {
            Visit(&'a MatchPattern),
            Variant {
                ty: &'a Type,
                enum_id: EnumId,
                variant: VariantId,
                layout: RuntimeLayoutId,
                fields: &'a [MatchFieldPattern],
            },
            Product {
                ty: &'a Type,
                product: ProductId,
                fields: &'a [MatchFieldPattern],
            },
        }

        let mut work = vec![Work::Visit(self)];
        let mut completed = Vec::new();
        while let Some(item) = work.pop() {
            match item {
                Work::Visit(pattern) => match pattern {
                    MatchPattern::Wildcard { ty } => {
                        completed.push(MatchPattern::Wildcard { ty: ty.clone() });
                    }
                    MatchPattern::Binding { local } => completed.push(MatchPattern::Binding {
                        local: local.clone(),
                    }),
                    MatchPattern::Bool(value) => completed.push(MatchPattern::Bool(*value)),
                    MatchPattern::I64(value) => completed.push(MatchPattern::I64(*value)),
                    MatchPattern::Variant {
                        ty,
                        enum_id,
                        variant,
                        layout,
                        fields,
                    } => {
                        work.push(Work::Variant {
                            ty,
                            enum_id: *enum_id,
                            variant: *variant,
                            layout: *layout,
                            fields,
                        });
                        work.extend(fields.iter().rev().map(|field| Work::Visit(&field.pattern)));
                    }
                    MatchPattern::Product {
                        ty,
                        product,
                        fields,
                    } => {
                        work.push(Work::Product {
                            ty,
                            product: *product,
                            fields,
                        });
                        work.extend(fields.iter().rev().map(|field| Work::Visit(&field.pattern)));
                    }
                },
                Work::Variant {
                    ty,
                    enum_id,
                    variant,
                    layout,
                    fields,
                } => {
                    let Some(start) = completed.len().checked_sub(fields.len()) else {
                        unreachable!("match pattern clone completion order")
                    };
                    let patterns: Vec<_> = completed.drain(start..).collect();
                    let cloned_fields = fields
                        .iter()
                        .zip(patterns)
                        .map(|(field, pattern)| MatchFieldPattern {
                            name: field.name.clone(),
                            field_index: field.field_index,
                            projection: field.projection.clone(),
                            pattern,
                        })
                        .collect();
                    completed.push(MatchPattern::Variant {
                        ty: ty.clone(),
                        enum_id,
                        variant,
                        layout,
                        fields: cloned_fields,
                    });
                }
                Work::Product {
                    ty,
                    product,
                    fields,
                } => {
                    let Some(start) = completed.len().checked_sub(fields.len()) else {
                        unreachable!("match pattern clone completion order")
                    };
                    let patterns: Vec<_> = completed.drain(start..).collect();
                    let cloned_fields = fields
                        .iter()
                        .zip(patterns)
                        .map(|(field, pattern)| MatchFieldPattern {
                            name: field.name.clone(),
                            field_index: field.field_index,
                            projection: field.projection.clone(),
                            pattern,
                        })
                        .collect();
                    completed.push(MatchPattern::Product {
                        ty: ty.clone(),
                        product,
                        fields: cloned_fields,
                    });
                }
            }
        }
        match completed.pop() {
            Some(pattern) if completed.is_empty() => pattern,
            _ => unreachable!("match pattern clone omitted its root"),
        }
    }
}

impl Drop for MatchPattern {
    fn drop(&mut self) {
        let mut pending = Vec::new();
        take_pattern_children(self, &mut pending);
        while let Some(mut pattern) = pending.pop() {
            take_pattern_children(&mut pattern, &mut pending);
        }
    }
}

fn take_pattern_children(pattern: &mut MatchPattern, pending: &mut Vec<MatchPattern>) {
    let fields = match pattern {
        MatchPattern::Variant { fields, .. } | MatchPattern::Product { fields, .. } => fields,
        _ => return,
    };
    for field in fields {
        pending.push(std::mem::replace(
            &mut field.pattern,
            MatchPattern::Wildcard { ty: Type::Unit },
        ));
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchTestKind {
    Bool(bool),
    I64(i64),
    Variant {
        enum_id: EnumId,
        variant: VariantId,
        layout: RuntimeLayoutId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchTest {
    pub arm: u64,
    pub path: Vec<u64>,
    pub kind: MatchTestKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchProjection {
    pub arm: u64,
    pub path: Vec<u64>,
    pub local: MatchLocal,
    pub active_variant: Option<VariantId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchBindingAssignment {
    pub arm: u64,
    pub path: Vec<u64>,
    pub local: MatchLocal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchEdgeTarget {
    Arm(u64),
    Default,
    Unreachable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedMatchArm {
    pub id: u64,
    pub pattern: MatchPattern,
    pub body_type: Type,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchPlan {
    pub(crate) id: MatchPlanId,
    pub(crate) origin: Origin,
    pub(crate) scrutinee: MatchLocal,
    pub(crate) result_type: Type,
    pub(crate) arms: Vec<PlannedMatchArm>,
    pub(crate) tests: Vec<MatchTest>,
    pub(crate) projections: Vec<MatchProjection>,
    pub(crate) bindings: Vec<MatchBindingAssignment>,
    pub(crate) edges: Vec<MatchEdgeTarget>,
    pub(crate) exhaustive: bool,
    pub(crate) witness: Option<String>,
}
