//! Closed copyable nominal variants over the existing immutable native values.
//! Prepared canonical meaning remains authoritative; this backend additionally
//! refuses affine, generic and recursive layouts before exposing a native image.

use super::asm::{Label, R::*, *};
use super::{Emitter, I, NormalizedCode, NormalizedProgram, Result, reserve, unsupported};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::execution::normalized::NormalizedValueSchema;
use crate::platform::execution::normalized::prepare::NormalizedVariantJump;
use crate::platform::kernel::{Name, TypeForm, TypeObjectDigest};
use std::collections::HashSet;

const VARIANT_TAG: u64 = 9;
const VARIANT_BYTES: u64 = 32;

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
enum Node {
    Type(TypeObjectDigest),
    Record(usize),
    Variant(usize),
}

pub(super) struct Admission<'a> {
    program: &'a NormalizedProgram,
    complete: HashSet<Node>,
}

fn set_reserve<T: Eq + std::hash::Hash>(set: &mut HashSet<T>) -> Result<()> {
    set.try_reserve(1).map_err(|_| {
        Diagnostic::new(
            DiagnosticClass::Resource,
            "native_elf_allocation",
            "cannot allocate native variant admission storage",
        )
    })
}

fn unique_names<'a>(names: impl Iterator<Item = &'a Name>) -> Result<()> {
    let mut seen = HashSet::new();
    for name in names {
        let name = name.as_str();
        set_reserve(&mut seen)?;
        if !seen.insert(name) {
            return Err(unsupported("native nominal member names are not unique"));
        }
    }
    Ok(())
}

impl<'a> Admission<'a> {
    pub(super) fn new(program: &'a NormalizedProgram) -> Self {
        Self {
            program,
            complete: HashSet::new(),
        }
    }

    pub(super) fn instruction(
        &mut self,
        code: &NormalizedCode,
        ip: usize,
        instruction: &I,
    ) -> Result<()> {
        match instruction {
            I::Variant {
                layout,
                type_arguments,
                case,
                has_payload,
            } => {
                if layout.1 != self.program.value_origin() || !type_arguments.is_empty() {
                    return Err(unsupported(
                        "native variant construction requires the exact closed layout",
                    ));
                }
                let index = usize::try_from(layout.0)
                    .map_err(|_| unsupported("native variant layout exceeds host indices"))?;
                self.copyable(Node::Variant(index))?;
                let selected = self
                    .program
                    .variants
                    .get(index)
                    .and_then(|variant| variant.cases.get(*case as usize))
                    .ok_or_else(|| unsupported("native variant case is absent"))?;
                if selected.payload.is_some() != *has_payload {
                    return Err(unsupported(
                        "native variant payload presence disagrees with its exact case",
                    ));
                }
            }
            I::SwitchVariant(jumps) => {
                let first = jumps
                    .first()
                    .ok_or_else(|| unsupported("native variant match has no arms"))?;
                if first.layout.1 != self.program.value_origin() {
                    return Err(unsupported("native variant match has a foreign origin"));
                }
                let index = usize::try_from(first.layout.0)
                    .map_err(|_| unsupported("native variant layout exceeds host indices"))?;
                self.copyable(Node::Variant(index))?;
                let variant = self
                    .program
                    .variants
                    .get(index)
                    .ok_or_else(|| unsupported("native match layout is absent"))?;
                if jumps.len() != variant.cases.len() {
                    return Err(unsupported(
                        "native match must cover its exact case inventory",
                    ));
                }
                let mut bindings = HashSet::new();
                let mut previous_target = ip;
                for (ordinal, (jump, case)) in jumps.iter().zip(variant.cases.iter()).enumerate() {
                    let target = usize::try_from(jump.target)
                        .map_err(|_| unsupported("native match target exceeds host indices"))?;
                    if jump.layout != first.layout
                        || usize::try_from(jump.case).ok() != Some(ordinal)
                        || target <= previous_target
                        || target >= code.instructions.len()
                        || case.payload.is_some() != jump.binding_local.is_some()
                    {
                        return Err(unsupported(
                            "native match cases, targets or payload bindings are not canonical",
                        ));
                    }
                    previous_target = target;
                    if let Some(local) = jump.binding_local {
                        if local < code.parameter_count || local >= code.local_count {
                            return Err(unsupported(
                                "native match payload local is outside its frame",
                            ));
                        }
                        set_reserve(&mut bindings)?;
                        if !bindings.insert(local) {
                            return Err(unsupported("native match arms share one payload local"));
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    // Enter/exit frames traverse the finite type/layout closure without using
    // the host call stack. Completed nodes share proof; an active-node revisit
    // identifies a recursive layout outside this backend slice, not a language
    // semantic error or a depth/size allowance.
    fn copyable(&mut self, root: Node) -> Result<()> {
        let mut pending = Vec::new();
        reserve(&mut pending, 1)?;
        pending.push((root, false));
        let mut active = HashSet::new();
        while let Some((node, exiting)) = pending.pop() {
            if exiting {
                active.remove(&node);
                set_reserve(&mut self.complete)?;
                self.complete.insert(node);
                continue;
            }
            if self.complete.contains(&node) {
                continue;
            }
            set_reserve(&mut active)?;
            if !active.insert(node) {
                return Err(unsupported(
                    "native variants currently require acyclic copyable layouts",
                ));
            }
            reserve(&mut pending, 1)?;
            pending.push((node, true));
            self.children(node, &mut pending)?;
        }
        Ok(())
    }

    fn children(&self, node: Node, pending: &mut Vec<(Node, bool)>) -> Result<()> {
        match node {
            Node::Type(ty) => {
                match self.program.types.get(&ty).map(|object| &object.form) {
                    Some(
                        TypeForm::Unit
                        | TypeForm::Bool
                        | TypeForm::I64
                        | TypeForm::Text
                        | TypeForm::StaticText
                        | TypeForm::Bytes,
                    ) => {}
                    Some(TypeForm::List { item }) => {
                        reserve(pending, 1)?;
                        pending.push((Node::Type(*item), false));
                    }
                    Some(TypeForm::StructuralRecord { fields }) => {
                        if fields.windows(2).any(|pair| pair[0].name >= pair[1].name) {
                            return Err(unsupported(
                                "native structural payload fields are not canonical",
                            ));
                        }
                        reserve(pending, fields.len())?;
                        pending.extend(
                            fields
                                .iter()
                                .rev()
                                .map(|field| (Node::Type(field.ty), false)),
                        );
                    }
                    Some(TypeForm::Function { parameters, result }) => {
                        let count = parameters.len().checked_add(1).ok_or_else(|| {
                            unsupported("native callable payload type count overflowed")
                        })?;
                        reserve(pending, count)?;
                        pending.push((Node::Type(*result), false));
                        pending.extend(parameters.iter().rev().map(|ty| (Node::Type(*ty), false)));
                    }
                    Some(TypeForm::Named { declaration }) => {
                        let child =
                            match (
                                self.program.record_index(ty),
                                self.program.variant_index(ty),
                            ) {
                                (Some(index), None)
                                    if self.program.records.get(index).is_some_and(|record| {
                                        record.declaration == *declaration
                                    }) =>
                                {
                                    Node::Record(index)
                                }
                                (None, Some(index))
                                    if self.program.variants.get(index).is_some_and(
                                        |variant| variant.declaration == *declaration,
                                    ) =>
                                {
                                    Node::Variant(index)
                                }
                                _ => {
                                    return Err(unsupported(
                                        "native nominal payload lacks its exact prepared layout",
                                    ));
                                }
                            };
                        reserve(pending, 1)?;
                        pending.push((child, false));
                    }
                    _ => {
                        return Err(unsupported(
                            "native variant payload must be closed copyable supported data",
                        ));
                    }
                }
            }
            Node::Record(index) => {
                let record = self
                    .program
                    .records
                    .get(index)
                    .ok_or_else(|| unsupported("native record payload layout is absent"))?;
                if record.owned
                    || !record.type_parameters.is_empty()
                    || !record.type_parameter_constraints.is_empty()
                    || !record.arguments.is_empty()
                    || record
                        .fields
                        .iter()
                        .any(|field| field.reference.package != record.declaration.package)
                    || record
                        .fields
                        .windows(2)
                        .any(|pair| pair[0].reference >= pair[1].reference)
                {
                    return Err(unsupported(
                        "native variant record payload must have an exact closed ordinary layout",
                    ));
                }
                unique_names(record.fields.iter().map(|field| &field.name))?;
                reserve(pending, record.fields.len())?;
                pending.extend(
                    record
                        .fields
                        .iter()
                        .rev()
                        .map(|field| (Node::Type(field.ty), false)),
                );
            }
            Node::Variant(index) => {
                let variant = self
                    .program
                    .variants
                    .get(index)
                    .ok_or_else(|| unsupported("native variant layout is absent"))?;
                if !variant.type_parameters.is_empty()
                    || !variant.type_parameter_constraints.is_empty()
                    || !variant.arguments.is_empty()
                    || variant.cases.is_empty()
                    || variant
                        .cases
                        .iter()
                        .any(|case| case.reference.package != variant.declaration.package)
                    || variant
                        .cases
                        .windows(2)
                        .any(|pair| pair[0].reference >= pair[1].reference)
                {
                    return Err(unsupported(
                        "native variant requires an exact closed case layout",
                    ));
                }
                unique_names(variant.cases.iter().map(|case| &case.name))?;
                reserve(pending, variant.cases.len())?;
                pending.extend(
                    variant
                        .cases
                        .iter()
                        .rev()
                        .filter_map(|case| case.payload.map(|ty| (Node::Type(ty), false))),
                );
            }
        }
        Ok(())
    }
}

impl Emitter<'_> {
    pub(super) fn variant(&mut self, layout: u32, case: u32, has_payload: bool) {
        // Keep the payload on the operand stack until checked allocation succeeds.
        self.a.imm(Di, VARIANT_BYTES);
        self.a.call(self.runtime.alloc);
        self.a.imm(Cx, VARIANT_TAG);
        self.a.store(Ax, 0, Cx);
        self.a.imm(Cx, u64::from(layout));
        self.a.store(Ax, 8, Cx);
        self.a.imm(Cx, u64::from(case));
        self.a.store(Ax, 16, Cx);
        if has_payload {
            self.a.pop(Cx);
        } else {
            self.a.imm(Cx, 0);
        }
        self.a.store(Ax, 24, Cx);
        self.a.push(Ax);
    }

    pub(super) fn switch_variant(
        &mut self,
        jumps: &[NormalizedVariantJump],
        labels: &[Label],
    ) -> Result<()> {
        let first = jumps
            .first()
            .ok_or_else(|| unsupported("native match has no dispatch layout"))?;
        let count = u64::try_from(jumps.len())
            .map_err(|_| unsupported("native match arm count exceeds the wire domain"))?;
        super::layout_extent(jumps.len(), 8, 0)?;
        let mut arms = Vec::new();
        reserve(&mut arms, jumps.len())?;
        for jump in jumps {
            let target = labels
                .get(jump.target as usize)
                .copied()
                .ok_or_else(|| unsupported("native match target is absent"))?;
            arms.push((self.a.label(), target, jump.binding_local));
        }
        let table = self.a.label();
        self.runtime.checkpoint(&mut self.a);
        self.a.pop(Ax);
        self.a.bin(0x85, Ax, Ax);
        self.a.when(E, self.runtime.variant);
        self.a.load(Cx, Ax, 0);
        self.a.group_wide(7, Cx, VARIANT_TAG);
        self.a.when(NE, self.runtime.variant);
        self.a.load(Cx, Ax, 8);
        self.a.group_wide(7, Cx, u64::from(first.layout.0));
        self.a.when(NE, self.runtime.variant);
        self.a.load(Cx, Ax, 16);
        self.a.group_wide(7, Cx, count);
        self.a.when(AE, self.runtime.variant);
        // Bounds and the checked table extent prove the scaling cannot overflow.
        self.a.shift(4, Cx, 3);
        self.a.address(Dx, table);
        self.a.bin(0x01, Dx, Cx);
        self.a.load(Dx, Dx, 0);
        self.a.load(Ax, Ax, 24);
        self.a.jump_register(Dx);
        self.a.align();
        self.a.bind(table);
        for (stub, _, _) in &arms {
            self.a.absolute(*stub);
        }
        for (stub, target, binding) in arms {
            self.a.bind(stub);
            self.a.bin(0x85, Ax, Ax);
            if let Some(local) = binding {
                self.a.when(E, self.runtime.variant);
                self.a.store_offset(Bp, -32 - i64::from(local) * 8, Ax);
            } else {
                self.a.when(NE, self.runtime.variant);
            }
            self.a.jump(target);
        }
        Ok(())
    }
}
