use crate::ssa::*;

pub(in crate::ssa) fn edge_arguments(
    result: ValueId,
    bindings: &[BindingId],
    env: &BTreeMap<BindingId, ValueId>,
) -> Result<Vec<ValueId>> {
    let mut arguments = Vec::with_capacity(bindings.len().saturating_add(1));
    arguments.push(result);
    for binding in bindings {
        arguments.push(
            env.get(binding)
                .copied()
                .ok_or_else(|| Error::msg("SSA branch lost mutable environment binding"))?,
        );
    }
    Ok(arguments)
}

pub(in crate::ssa) fn restore<K: Ord, V>(map: &mut BTreeMap<K, V>, key: K, previous: Option<V>) {
    if let Some(previous) = previous {
        map.insert(key, previous);
    } else {
        map.remove(&key);
    }
}

pub(in crate::ssa) fn signature_from_type(
    ty: &Type,
    products: &HashMap<String, ProductId>,
) -> Result<Signature> {
    match ty {
        Type::Fn { params, ret } => Ok(Signature::monomorphic(
            params
                .iter()
                .map(|parameter| lower_type(parameter, products))
                .collect::<Result<Vec<_>>>()?,
            lower_type(ret, products)?,
        )),
        Type::Forall { vars, body } => {
            let Type::Fn { params, ret } = body.as_ref() else {
                return Err(Error::msg("HIR forall body is not a function"));
            };
            Ok(Signature {
                type_parameters: vars.clone(),
                bounds: Vec::new(),
                parameters: params
                    .iter()
                    .map(|parameter| lower_type(parameter, products))
                    .collect::<Result<Vec<_>>>()?,
                result: Box::new(lower_type(ret, products)?),
            })
        }
        _ => Err(Error::msg("HIR callable does not have a function type")),
    }
}

pub(in crate::ssa) fn lower_type(
    ty: &Type,
    products: &HashMap<String, ProductId>,
) -> Result<SsaType> {
    Ok(match ty {
        Type::Never => return Err(Error::msg("Never has no SSA value representation")),
        Type::Unit => SsaType::Unit,
        Type::Bool => SsaType::Bool,
        Type::I64 => SsaType::I64,
        Type::F64 => SsaType::F64,
        Type::Str => SsaType::Str,
        Type::Buf => SsaType::Buf,
        Type::ByteVector => SsaType::ByteVector,
        Type::ByteSlice => SsaType::ByteSlice,
        Type::ByteSliceMut => SsaType::ByteSliceMut,
        Type::Path => SsaType::Path,
        Type::Capability(kind) => SsaType::Capability(*kind),
        Type::Symbol => SsaType::Symbol,
        Type::Resource(kind) => SsaType::Resource(*kind),
        Type::Product(name) => SsaType::Product(
            *products
                .get(name)
                .ok_or_else(|| Error::msg(format!("HIR type references unknown product {name}")))?,
        ),
        Type::Enum { id, arguments, .. } => SsaType::Enum {
            id: lkjscript_ir::EnumId::new(id.bytes()),
            arguments: arguments
                .iter()
                .map(|argument| lower_type(argument, products))
                .collect::<Result<Vec<_>>>()?,
        },
        Type::Param(name) => SsaType::TypeParameter(name.clone()),
        Type::List(item) => SsaType::List(Box::new(lower_type(item, products)?)),
        Type::Fn { .. } | Type::Forall { .. } => {
            SsaType::Function(Box::new(signature_from_type(ty, products)?))
        }
    })
}

pub(in crate::ssa) fn is_owned_value(ty: &SsaType) -> bool {
    matches!(ty, SsaType::ByteVector | SsaType::Resource(_))
}

pub(in crate::ssa) struct CleanupPlan {
    pub(in crate::ssa) next_expression: u32,
    pub(in crate::ssa) loan_ends: BTreeMap<u32, Vec<SsaLoanId>>,
    pub(in crate::ssa) place_glues: BTreeMap<SsaPlaceId, DropGlueIdentity>,
}

#[derive(Clone, Copy)]
pub(in crate::ssa) struct ActiveLoan {
    pub(in crate::ssa) place: SsaPlaceId,
    pub(in crate::ssa) value: ValueId,
}

impl CleanupPlan {
    pub(in crate::ssa) fn new(plan: &HirMemoryPlan, function: MemoryFunctionId) -> Result<Self> {
        let function_plan = plan
            .function(function)
            .ok_or_else(|| Error::msg("HIR memory plan lost an SSA function"))?;
        let mut loan_ends: BTreeMap<u32, Vec<SsaLoanId>> = BTreeMap::new();
        for loan in plan.loans.iter().filter(|loan| loan.function == function) {
            loan_ends
                .entry(loan.end_after.raw())
                .or_default()
                .push(SsaLoanId::new(loan.loan));
        }
        let mut place_glues = BTreeMap::new();
        for obligation in plan
            .obligations
            .iter()
            .filter(|obligation| obligation.function == function)
        {
            if matches!(obligation.kind, MemoryObligationKind::EndBorrow) {
                continue;
            }
            let entry = plan
                .entry(obligation.entry)
                .ok_or_else(|| Error::msg("HIR memory obligation lost its entry"))?;
            let MemorySubject::Place { place, .. } = entry.subject else {
                return Err(Error::msg("HIR drop obligation does not name a place"));
            };
            let glue = obligation
                .drop_glue
                .and_then(|id| plan.drop_glues.iter().find(|glue| glue.id == id))
                .ok_or_else(|| Error::msg("HIR drop obligation lost closed glue identity"))?;
            let glue = match glue.kind {
                MemoryDropGlueKind::ByteVector => DropGlueIdentity::ByteVector,
                MemoryDropGlueKind::Resource(kind) => DropGlueIdentity::Resource(kind),
            };
            if place_glues.insert(SsaPlaceId::new(place), glue).is_some() {
                return Err(Error::msg(
                    "HIR memory plan duplicates a place drop obligation",
                ));
            }
        }
        Ok(Self {
            next_expression: function_plan.body.raw(),
            loan_ends,
            place_glues,
        })
    }

    pub(in crate::ssa) fn begin_expression(&mut self) -> Result<MemoryExpressionId> {
        let id = MemoryExpressionId::new(self.next_expression);
        self.next_expression = self
            .next_expression
            .checked_add(1)
            .ok_or_else(|| Error::msg("SSA HIR memory expression identity overflow"))?;
        Ok(id)
    }
}
