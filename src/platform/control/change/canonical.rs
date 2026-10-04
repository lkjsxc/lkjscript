//! Complete canonical reads for disposable declaration proposals, with the maintained aggregate
//! definition admission. No historical authoring source or rendered inspection text is consumed.
use super::*;
use crate::platform::change::AuthoredReadFootprint;
use crate::platform::execution::ExecutionControl;
use crate::platform::kernel::{self as k, OwnerRecord, TypeForm, TypeObjectDigest};
use crate::platform::publication::{RepositoryDefinitionReader, RepositoryView};

pub(super) struct Reader<'a> {
    pub view: &'a RepositoryView,
    pub reader: RepositoryDefinitionReader<'a>,
    pub owners: BTreeMap<OwnerKey, OwnerRecord>,
    types: BTreeMap<TypeObjectDigest, AuthoredType>,
    active_types: BTreeSet<TypeObjectDigest>,
    active_expressions: BTreeSet<ExpressionId>,
    interface_names: BTreeMap<PackageId, BTreeMap<OwnerKey, Name>>,
    remaining_type_nodes: u64,
    control: ExecutionControl,
    intent_reads: Option<AuthoredReadFootprint>,
}

pub(super) fn error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Source, "change_draft_definition", message)
}

impl<'a> Reader<'a> {
    pub fn new(view: &'a RepositoryView, control: ExecutionControl) -> Self {
        Self {
            view,
            reader: view.definition_reader(),
            owners: BTreeMap::new(),
            types: BTreeMap::new(),
            active_types: BTreeSet::new(),
            active_expressions: BTreeSet::new(),
            interface_names: BTreeMap::new(),
            remaining_type_nodes: crate::platform::change::MAXIMUM_CHANGE_AUTHORED_TYPE_NODES,
            control,
            intent_reads: None,
        }
    }

    pub fn for_intent(view: &'a RepositoryView, control: ExecutionControl) -> Self {
        let mut reader = Self::new(view, control);
        reader.intent_reads = Some(AuthoredReadFootprint::default());
        reader
    }

    pub fn take_intent_reads(&mut self) -> AuthoredReadFootprint {
        self.intent_reads.take().unwrap_or_default()
    }

    pub fn check(&self) -> Result<(), Diagnostic> {
        self.control
            .check()
            .map_err(|e| Diagnostic::new(DiagnosticClass::Cancelled, e.code, e.message))
    }

    pub fn owner(&mut self, id: OwnerKey) -> Result<OwnerRecord, Diagnostic> {
        self.check()?;
        if let Some(owner) = self.owners.get(&id) {
            return Ok(owner.clone());
        }
        let owner = self.reader.owner(id)?;
        if let Some(reads) = &mut self.intent_reads {
            reads.record_owner(id, owner.as_ref())?;
        }
        let owner = owner
            .ok_or_else(|| error(format!("selected owner {id} is absent at the bound base")))?;
        self.owners.insert(id, owner.clone());
        Ok(owner)
    }

    pub fn incoming(
        &mut self,
        owner: OwnerKey,
        kind: k::RelationKind,
        maximum: usize,
    ) -> Result<Vec<k::RelationEdge>, Diagnostic> {
        self.check()?;
        // The maintained reader rejects truncated ranges before this guard is admitted.
        // Empty ranges are retained as dependencies on the absence of implicit children.
        let edges = self.reader.incoming(owner, kind, maximum)?;
        if let Some(reads) = &mut self.intent_reads {
            reads.record_relations(owner, Some(kind), true, maximum, &edges, false)?;
        }
        Ok(edges)
    }

    pub fn reference_name(
        &mut self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<String, Diagnostic> {
        if package == self.view.package() {
            return self
                .owner(owner)?
                .name()
                .map(ToString::to_string)
                .ok_or_else(|| error("reference has no canonical name"));
        }
        if !self.interface_names.contains_key(&package) {
            let names = self.reader.interface_names(package)?;
            self.interface_names.insert(package, names);
        }
        self.interface_names[&package]
            .get(&owner)
            .map(ToString::to_string)
            .ok_or_else(|| error("reference is absent from its exact public supplier"))
    }

    pub fn ty(&mut self, digest: TypeObjectDigest) -> Result<AuthoredType, Diagnostic> {
        self.ty_at(digest, 1)
    }

    fn ty_at(
        &mut self,
        digest: TypeObjectDigest,
        depth: usize,
    ) -> Result<AuthoredType, Diagnostic> {
        self.check()?;
        self.remaining_type_nodes = self.remaining_type_nodes.checked_sub(1).ok_or_else(|| {
            Diagnostic::new(
                DiagnosticClass::Resource,
                "change_draft_capacity",
                "canonical type expansion exceeds complete draft admission",
            )
        })?;
        if depth > k::contract::MAXIMUM_TYPE_DEPTH || !self.active_types.insert(digest) {
            return Err(error(
                "canonical type is cyclic or exceeds type-depth admission",
            ));
        }
        if let Some(ty) = self.types.get(&digest) {
            let mut pending = vec![(ty, depth)];
            let mut nodes = 0_u64;
            while let Some((ty, depth)) = pending.pop() {
                self.check()?;
                if depth > k::contract::MAXIMUM_TYPE_DEPTH {
                    return Err(error("expanded type exceeds type-depth admission"));
                }
                nodes = nodes
                    .checked_add(1)
                    .ok_or_else(|| error("type node accounting overflow"))?;
                if nodes > self.remaining_type_nodes {
                    return Err(Diagnostic::new(
                        DiagnosticClass::Resource,
                        "change_draft_capacity",
                        "cached type expansion exceeds draft admission",
                    ));
                }
                use AuthoredType as T;
                match ty {
                    T::Applied { arguments, .. } => {
                        pending.extend(arguments.iter().map(|t| (t, depth + 1)))
                    }
                    T::List { item } | T::Option { item } | T::Stream { item } => {
                        pending.push((item, depth + 1))
                    }
                    T::Map { key, value } => {
                        pending.extend([(key.as_ref(), depth + 1), (value.as_ref(), depth + 1)])
                    }
                    T::Result { ok, error } => {
                        pending.extend([(ok.as_ref(), depth + 1), (error.as_ref(), depth + 1)])
                    }
                    T::StructuralRecord { fields } => {
                        pending.extend(fields.iter().map(|f| (&f.ty, depth + 1)))
                    }
                    T::Function { parameters, result }
                    | T::TaskFunction {
                        parameters, result, ..
                    } => {
                        pending.extend(parameters.iter().map(|t| (t, depth + 1)));
                        pending.push((result, depth + 1));
                    }
                    _ => {}
                }
            }
            self.remaining_type_nodes -= nodes;
            self.active_types.remove(&digest);
            return Ok(ty.clone());
        }
        let object = self
            .reader
            .type_object(digest)?
            .ok_or_else(|| error("canonical type object is absent"))?;
        let ty = match object.form {
            TypeForm::Unit => AuthoredType::Unit {},
            TypeForm::Bool => AuthoredType::Bool {},
            TypeForm::I64 => AuthoredType::I64 {},
            TypeForm::F64 => AuthoredType::F64 {},
            TypeForm::ByteBuffer => AuthoredType::ByteBuffer {},
            TypeForm::OwnedI64Cell => AuthoredType::OwnedI64Cell {},
            TypeForm::OwnedChoice { cases } => AuthoredType::OwnedChoice {
                cases: cases
                    .into_iter()
                    .map(|case| {
                        Ok(AuthoredStructuralTypeField {
                            name: case.name,
                            ty: self.ty_at(case.ty, depth + 1)?,
                        })
                    })
                    .collect::<Result<_, Diagnostic>>()?,
            },
            TypeForm::OwnedProduct { fields } => AuthoredType::OwnedProduct {
                fields: fields
                    .into_iter()
                    .map(|f| {
                        Ok(AuthoredStructuralTypeField {
                            name: f.name,
                            ty: self.ty_at(f.ty, depth + 1)?,
                        })
                    })
                    .collect::<Result<_, Diagnostic>>()?,
            },
            TypeForm::Bytes => AuthoredType::Bytes {},
            TypeForm::Text => AuthoredType::Text {},
            TypeForm::StaticText => AuthoredType::StaticText {},
            TypeForm::Secret => AuthoredType::Secret {},
            TypeForm::TypeParameter { parameter } => AuthoredType::TypeParameter {
                parameter: AuthoredTypeParameterReference::Id { parameter },
            },
            TypeForm::Named { declaration: d } => AuthoredType::Named {
                declaration: declaration(d),
            },
            TypeForm::Applied {
                declaration: d,
                arguments,
            } => AuthoredType::Applied {
                declaration: declaration(d),
                arguments: arguments
                    .into_iter()
                    .map(|t| self.ty_at(t, depth + 1))
                    .collect::<Result<_, _>>()?,
            },
            TypeForm::CapabilityResource { interface } => AuthoredType::CapabilityResource {
                interface: declaration(interface),
            },
            TypeForm::List { item } => AuthoredType::List {
                item: Box::new(self.ty_at(item, depth + 1)?),
            },
            TypeForm::Option { item } => AuthoredType::Option {
                item: Box::new(self.ty_at(item, depth + 1)?),
            },
            TypeForm::Stream { item } => AuthoredType::Stream {
                item: Box::new(self.ty_at(item, depth + 1)?),
            },
            TypeForm::Map { key, value } => AuthoredType::Map {
                key: Box::new(self.ty_at(key, depth + 1)?),
                value: Box::new(self.ty_at(value, depth + 1)?),
            },
            TypeForm::Result { ok, error } => AuthoredType::Result {
                ok: Box::new(self.ty_at(ok, depth + 1)?),
                error: Box::new(self.ty_at(error, depth + 1)?),
            },
            TypeForm::StructuralRecord { fields } => AuthoredType::StructuralRecord {
                fields: fields
                    .into_iter()
                    .map(|f| {
                        Ok(AuthoredStructuralTypeField {
                            name: f.name,
                            ty: self.ty_at(f.ty, depth + 1)?,
                        })
                    })
                    .collect::<Result<_, Diagnostic>>()?,
            },
            TypeForm::Function { parameters, result } => AuthoredType::Function {
                parameters: parameters
                    .into_iter()
                    .map(|t| self.ty_at(t, depth + 1))
                    .collect::<Result<_, _>>()?,
                result: Box::new(self.ty_at(result, depth + 1)?),
            },
            TypeForm::TaskFunction {
                parameters,
                result,
                effect,
            } => AuthoredType::TaskFunction {
                parameters: parameters
                    .into_iter()
                    .map(|t| self.ty_at(t, depth + 1))
                    .collect::<Result<_, _>>()?,
                result: Box::new(self.ty_at(result, depth + 1)?),
                effect: row(effect),
            },
        };
        self.active_types.remove(&digest);
        self.types.insert(digest, ty.clone());
        Ok(ty)
    }

    fn text(&mut self, text: k::TextValue) -> Result<String, Diagnostic> {
        match text {
            k::TextValue::Inline { text } => Ok(text),
            k::TextValue::Blob { digest, bytes } => {
                let content = self.reader.blob(digest)?;
                if content.len() as u64 != bytes {
                    return Err(error(
                        "canonical text blob length differs from its expression",
                    ));
                }
                String::from_utf8(content).map_err(|_| error("canonical text blob is not UTF-8"))
            }
        }
    }

    fn binding(&mut self, id: BindingId) -> Result<k::BindingRecord, Diagnostic> {
        match self.owner(OwnerKey::Binding(id))? {
            OwnerRecord::Binding(record) => Ok(record),
            _ => Err(error("binding identity names another owner kind")),
        }
    }

    pub fn expression(&mut self, id: ExpressionId) -> Result<AuthoredExpression, Diagnostic> {
        self.expression_at(id, 1)
    }

    fn expression_at(
        &mut self,
        id: ExpressionId,
        depth: usize,
    ) -> Result<AuthoredExpression, Diagnostic> {
        self.check()?;
        if depth > k::contract::MAXIMUM_EXPRESSION_DEPTH || !self.active_expressions.insert(id) {
            return Err(error(
                "canonical body is cyclic or exceeds expression-depth admission",
            ));
        }
        let OwnerRecord::Expression(expression) = self.owner(OwnerKey::Expression(id))? else {
            return Err(error("expression identity names another owner kind"));
        };
        use AuthoredExpressionOperation as A;
        use k::ExpressionOperation as E;
        let borrowed_choice = matches!(expression.operation, E::MatchBorrowedOwned { .. });
        let operation = match expression.operation {
            E::Parallel { left, right } => A::Parallel {
                left: Box::new(self.expression_at(left, depth + 1)?),
                right: Box::new(self.expression_at(right, depth + 1)?),
            },
            E::Unit {} => A::Unit {},
            E::Bool { value } => A::Bool { value },
            E::I64 { value } => A::I64 { value },
            E::F64 { value } => A::F64 { value },
            E::Text { value } => A::Text {
                value: self.text(value)?,
            },
            E::StaticText { value } => A::StaticText {
                value: self.text(value)?,
            },
            E::Local { value } => A::Local {
                value: match value {
                    k::LocalValueReference::FunctionParameter(parameter) => {
                        AuthoredLocalReference::FunctionParameter { parameter }
                    }
                    k::LocalValueReference::OperationParameter(parameter) => {
                        AuthoredLocalReference::OperationParameter { parameter }
                    }
                    k::LocalValueReference::LexicalBinding(binding)
                    | k::LocalValueReference::MatchPayload(binding)
                    | k::LocalValueReference::TransactionBinding(binding) => {
                        AuthoredLocalReference::Symbol {
                            symbol: binding_symbol(binding),
                        }
                    }
                },
            },
            E::Constant { declaration: d } => A::Constant {
                declaration: declaration(d),
            },
            E::If {
                condition,
                when_true,
                when_false,
            } => A::If {
                condition: Box::new(self.expression_at(condition, depth + 1)?),
                when_true: Box::new(self.expression_at(when_true, depth + 1)?),
                when_false: Box::new(self.expression_at(when_false, depth + 1)?),
            },
            E::ChooseOwned {
                choice_type,
                case,
                value,
            } => A::ChooseOwned {
                choice_type: self.ty(choice_type)?,
                case,
                value: Box::new(self.expression_at(value, depth + 1)?),
            },
            E::MatchOwned {
                choice_type,
                source,
                arms,
            }
            | E::MatchBorrowedOwned {
                choice_type,
                source,
                arms,
            } => {
                let mut authored = Vec::new();
                for arm in arms {
                    let binding = self.binding(arm.binding)?;
                    authored.push((
                        arm.name,
                        AuthoredBindingDefinition {
                            symbol: binding_symbol(arm.binding),
                            name: binding.name,
                            declared_type: binding
                                .declared_type
                                .map(|ty| self.ty(ty))
                                .transpose()?,
                        },
                        self.expression_at(arm.body, depth + 1)?,
                    ));
                }
                let choice_type = self.ty(choice_type)?;
                let source = Box::new(self.expression_at(source, depth + 1)?);
                if borrowed_choice {
                    A::MatchBorrowedOwned {
                        choice_type,
                        source,
                        arms: authored,
                    }
                } else {
                    A::MatchOwned {
                        choice_type,
                        source,
                        arms: authored,
                    }
                }
            }
            E::PackOwned {
                product_type,
                fields,
            } => A::PackOwned {
                product_type: self.ty(product_type)?,
                fields: fields
                    .into_iter()
                    .map(|f| Ok((f.name, self.expression_at(f.value, depth + 1)?)))
                    .collect::<Result<_, Diagnostic>>()?,
            },
            E::UnpackOwned {
                product_type,
                source,
                fields,
                body,
            } => {
                let mut authored = Vec::new();
                for f in fields {
                    let b = self.binding(f.binding)?;
                    authored.push((
                        f.name,
                        AuthoredBindingDefinition {
                            symbol: binding_symbol(f.binding),
                            name: b.name,
                            declared_type: b.declared_type.map(|ty| self.ty(ty)).transpose()?,
                        },
                    ));
                }
                A::UnpackOwned {
                    product_type: self.ty(product_type)?,
                    source: Box::new(self.expression_at(source, depth + 1)?),
                    fields: authored,
                    body: Box::new(self.expression_at(body, depth + 1)?),
                }
            }
            E::BorrowOwnedField {
                product_type,
                source,
                field,
                binding,
                body,
            } => A::BorrowOwnedField {
                product_type: self.ty(product_type)?,
                source: Box::new(self.expression_at(source, depth + 1)?),
                field,
                binding: Box::new(self.binding_definition(binding)?),
                body: Box::new(self.expression_at(body, depth + 1)?),
            },
            E::Let { bindings, body } => {
                let mut authored = Vec::new();
                for id in bindings {
                    let b = self.binding(id)?;
                    authored.push(AuthoredLetBinding {
                        symbol: binding_symbol(id),
                        name: b.name,
                        value: self.expression_at(
                            b.value
                                .ok_or_else(|| error("let binder has no initializer"))?,
                            depth + 1,
                        )?,
                        declared_type: b.declared_type.map(|ty| self.ty(ty)).transpose()?,
                    });
                }
                A::Let {
                    bindings: authored,
                    body: Box::new(self.expression_at(body, depth + 1)?),
                }
            }
            E::Sequence { items } => A::Sequence {
                items: self.expressions(items, depth)?,
            },
            E::ImplementationCall {
                function,
                type_arguments,
                effect_arguments,
                requirement_arguments,
                implementations,
                arguments,
            } => A::ImplementationCall {
                function: declaration(function),
                type_arguments: self.types(type_arguments)?,
                effect_arguments: effect_arguments.into_iter().map(row).collect(),
                requirement_arguments: requirement_arguments.into_iter().map(requirement).collect(),
                implementations: implementations
                    .into_iter()
                    .map(implementation_operand)
                    .collect(),
                arguments: self.expressions(arguments, depth)?,
            },
            E::MethodCall {
                witness,
                contract,
                method,
                arguments,
            } => A::MethodCall {
                witness: implementation_operand(witness),
                contract: declaration(contract),
                method,
                arguments: self.expressions(arguments, depth)?,
            },
            E::Call {
                function,
                type_arguments,
                effect_arguments,
                requirement_arguments,
                arguments,
            } => A::Call {
                function: declaration(function),
                type_arguments: self.types(type_arguments)?,
                effect_arguments: effect_arguments.into_iter().map(row).collect(),
                requirement_arguments: requirement_arguments.into_iter().map(requirement).collect(),
                arguments: self.expressions(arguments, depth)?,
            },
            E::FunctionValue {
                function,
                type_arguments,
                effect_arguments,
                requirement_arguments,
            } => A::FunctionValue {
                function: declaration(function),
                type_arguments: self.types(type_arguments)?,
                effect_arguments: effect_arguments.into_iter().map(row).collect(),
                requirement_arguments: requirement_arguments.into_iter().map(requirement).collect(),
            },
            E::Invoke { callee, arguments } => A::Invoke {
                callee: Box::new(self.expression_at(callee, depth + 1)?),
                arguments: self.expressions(arguments, depth)?,
            },
            E::Bind { callee, arguments } => A::Bind {
                callee: Box::new(self.expression_at(callee, depth + 1)?),
                arguments: self.expressions(arguments, depth)?,
            },
            E::Record {
                nominal_type,
                type_arguments,
                fields,
            } => A::Record {
                nominal_type: nominal_type.map(declaration),
                type_arguments: self.types(type_arguments)?,
                fields: fields
                    .into_iter()
                    .map(|f| {
                        Ok(AuthoredRecordExpressionField {
                            selector: field(f.selector),
                            value: self.expression_at(f.value, depth + 1)?,
                        })
                    })
                    .collect::<Result<_, Diagnostic>>()?,
            },
            E::Variant {
                case: c,
                type_arguments,
                payload,
            } => A::Variant {
                case: case(c),
                type_arguments: self.types(type_arguments)?,
                payload: payload
                    .map(|e| self.expression_at(e, depth + 1).map(Box::new))
                    .transpose()?,
            },
            E::Field { value, selector } => A::Field {
                value: Box::new(self.expression_at(value, depth + 1)?),
                selector: field(selector),
            },
            E::List { item_type, items } => A::List {
                item_type: self.ty(item_type)?,
                items: self.expressions(items, depth)?,
            },
            E::Map {
                key_type,
                value_type,
                entries,
            } => A::Map {
                key_type: self.ty(key_type)?,
                value_type: self.ty(value_type)?,
                entries: entries
                    .into_iter()
                    .map(|e| {
                        Ok(AuthoredMapExpressionEntry {
                            key: self.expression_at(e.key, depth + 1)?,
                            value: self.expression_at(e.value, depth + 1)?,
                        })
                    })
                    .collect::<Result<_, Diagnostic>>()?,
            },
            E::Match { value, arms } => {
                let value = Box::new(self.expression_at(value, depth + 1)?);
                let mut authored = Vec::new();
                for arm in arms {
                    let payload_binding = arm
                        .payload_binding
                        .map(|id| self.binding_definition(id))
                        .transpose()?;
                    authored.push(AuthoredMatchExpressionArm {
                        case: case(arm.case),
                        payload_binding,
                        body: self.expression_at(arm.body, depth + 1)?,
                    });
                }
                A::Match {
                    value,
                    arms: authored,
                }
            }
            E::CapabilityCall {
                requirement: r,
                operation,
                arguments,
            } => A::CapabilityCall {
                requirement: requirement(r),
                operation: AuthoredOperationReference::Exact {
                    package: operation.package,
                    operation: operation.operation,
                },
                arguments: self.expressions(arguments, depth)?,
            },
            E::Transaction {
                requirement: r,
                binding,
                body,
            } => A::Transaction {
                requirement: requirement(r),
                binding: self.binding_definition(binding)?,
                body: Box::new(self.expression_at(body, depth + 1)?),
            },
            E::TransactionOutcome {
                requirement: r,
                binding,
                body,
                type_argument,
                outcome,
            } => A::TransactionOutcome {
                requirement: requirement(r),
                binding: self.binding_definition(binding)?,
                body: Box::new(self.expression_at(body, depth + 1)?),
                type_argument: Box::new(self.ty(type_argument)?),
                outcome: Box::new(AuthoredTransactionOutcomeContract {
                    outcome: declaration(outcome.outcome),
                    abort_reason: declaration(outcome.abort_reason),
                    committed: case(outcome.committed),
                    aborted: case(outcome.aborted),
                    condition_failed: case(outcome.condition_failed),
                    conflict: case(outcome.conflict),
                }),
            },
        };
        self.active_expressions.remove(&id);
        Ok(AuthoredExpression {
            symbol: Some(format!("$e_{id}")),
            operation,
        })
    }

    fn expressions(
        &mut self,
        values: Vec<ExpressionId>,
        depth: usize,
    ) -> Result<Vec<AuthoredExpression>, Diagnostic> {
        values
            .into_iter()
            .map(|e| self.expression_at(e, depth + 1))
            .collect()
    }
    fn types(&mut self, values: Vec<TypeObjectDigest>) -> Result<Vec<AuthoredType>, Diagnostic> {
        values.into_iter().map(|t| self.ty(t)).collect()
    }
    fn binding_definition(
        &mut self,
        id: BindingId,
    ) -> Result<AuthoredBindingDefinition, Diagnostic> {
        let b = self.binding(id)?;
        Ok(AuthoredBindingDefinition {
            symbol: binding_symbol(id),
            name: b.name,
            declared_type: b.declared_type.map(|t| self.ty(t)).transpose()?,
        })
    }
}

fn binding_symbol(id: BindingId) -> String {
    format!("$b_{id}")
}
pub(super) fn declaration(d: k::DeclarationReference) -> AuthoredDeclarationReference {
    AuthoredDeclarationReference::Exact {
        package: d.package,
        declaration: d.declaration,
    }
}
fn case(c: k::CaseReference) -> AuthoredCaseReference {
    AuthoredCaseReference::Exact {
        package: c.package,
        case: c.case,
    }
}
pub(super) fn requirement(r: k::RequirementOperand) -> AuthoredRequirementReference {
    match r {
        k::RequirementOperand::Concrete(r) => AuthoredRequirementReference::Exact {
            package: r.package,
            requirement: r.requirement,
        },
        k::RequirementOperand::Parameter(p) => AuthoredRequirementReference::ParameterExact {
            package: p.package,
            parameter: p.parameter,
        },
    }
}
pub(super) fn row(row: k::EffectRow) -> AuthoredEffectRow {
    AuthoredEffectRow {
        requirements: row.requirements.into_iter().map(requirement).collect(),
        parameters: row
            .parameters
            .into_iter()
            .map(|p| AuthoredEffectParameterReference::Exact {
                package: p.package,
                parameter: p.parameter,
            })
            .collect(),
    }
}
fn field(selector: k::FieldSelector) -> AuthoredFieldSelector {
    match selector {
        k::FieldSelector::Nominal(field) => AuthoredFieldSelector::Nominal {
            field: AuthoredFieldReference::Exact {
                package: field.package,
                field: field.field,
            },
        },
        k::FieldSelector::Structural(name) => AuthoredFieldSelector::Structural { name },
    }
}

pub(super) fn implementation_operand(
    operand: k::ImplementationOperand,
) -> AuthoredImplementationOperand {
    match operand {
        k::ImplementationOperand::Concrete { implementation } => {
            AuthoredImplementationOperand::Concrete {
                implementation: declaration(implementation),
            }
        }
        k::ImplementationOperand::Parameter {
            function,
            parameter,
        } => AuthoredImplementationOperand::Parameter {
            function: declaration(function),
            parameter,
        },
    }
}

#[cfg(test)]
mod intent_read_tests {
    use super::*;
    use crate::platform::change::{CanonicalBaseRead, ChangeBudget};
    use crate::platform::publication::GraphRepository;

    fn publish(repository: &GraphRepository, input: &str) {
        let request = decode_compact_change_in_repository(
            "concurrent-change.lkjc",
            input.as_bytes(),
            repository,
        )
        .unwrap();
        let prepared = repository
            .prepare_authored_change(&request.semantic, request.options)
            .unwrap();
        repository.publish(&prepared.publication).unwrap();
    }

    #[test]
    fn native_origin_decode_retains_body_reads_without_unrelated_module_members() {
        let temporary = tempfile::tempdir().unwrap();
        let initial = crate::platform::kernel::tests::witness_snapshot();
        let created =
            GraphRepository::create(&temporary.path().join("meaning"), &initial, None).unwrap();
        let input = format!(
            "request base={}\ndeclarations.begin\n\
             (units (module create native_refresh_reads (as $module)\n\
               (function create chosen (as $chosen) (visibility private)\n\
                 (returns I64) (effect pure) (body (sequence (i64 1) (i64 7))))))\n\
             declarations.end\n",
            created.current.head.revision
        );
        let request = decode_compact_change_in_repository(
            "native-origin.lkjc",
            input.as_bytes(),
            &created.repository,
        )
        .unwrap();
        let prepared = created
            .repository
            .prepare_authored_change(&request.semantic, request.options)
            .unwrap();
        created.repository.publish(&prepared.publication).unwrap();
        let function = prepared.allocated["$chosen"];
        let module = prepared.allocated["$module"];
        let origin = created.repository.view_current().unwrap();
        let draft = render_native_draft(
            &origin,
            &[function.into()],
            4 * 1_048_576,
            ExecutionControl::uncancelled(),
        )
        .unwrap();
        let edit = String::from_utf8(draft)
            .unwrap()
            .replace("(i64 7)", "(i64 8)");
        let original =
            decode_compact_change_in_view("edit.lkjc", edit.as_bytes(), &origin).unwrap();
        // The exact module, function and three body expressions are consumed by native
        // normalization. Its module's aggregate summaries and other members are not.
        assert_eq!(original.native_reads.len(), 5);

        publish(
            &created.repository,
            &format!(
                "request base={}\nexpression.i64 as=$value value=99\n\
                 create.constant as=$neighbor module={module} name=neighbor visibility=private type=i64 value=$value\n",
                origin.revision()
            ),
        );
        let current = created.repository.view_current().unwrap();
        original
            .native_reads
            .check_against(&current, &current, ChangeBudget::default())
            .unwrap();
        let pinned =
            decode_compact_change_in_view("pinned.lkjc", edit.as_bytes(), &origin).unwrap();
        assert_eq!(pinned.request_commitment, original.request_commitment);
        assert_eq!(pinned.native_reads.digest(), original.native_reads.digest());
        assert_eq!(
            decode_compact_change_in_repository("stale.lkjc", edit.as_bytes(), &created.repository)
                .unwrap_err()[0]
                .code,
            "change_unit_base"
        );
        assert_eq!(
            decode_compact_change_in_view("wrong-view.lkjc", edit.as_bytes(), &current)
                .unwrap_err()[0]
                .code,
            "change_unit_base"
        );

        let draft = render_native_draft(
            &current,
            &[function.into()],
            4 * 1_048_576,
            ExecutionControl::uncancelled(),
        )
        .unwrap();
        publish(
            &created.repository,
            &String::from_utf8(draft)
                .unwrap()
                .replace("(i64 1)", "(i64 2)"),
        );
        let changed = created.repository.view_current().unwrap();
        assert_eq!(
            original
                .native_reads
                .check_against(&changed, &changed, ChangeBudget::default())
                .unwrap_err()
                .code,
            "change_refresh_conflict"
        );
    }

    #[test]
    fn native_target_decode_guards_the_complete_empty_incoming_route_range() {
        let temporary = tempfile::tempdir().unwrap();
        let initial = crate::platform::kernel::tests::witness_snapshot();
        let created =
            GraphRepository::create(&temporary.path().join("meaning"), &initial, None).unwrap();
        let target = *initial
            .owners
            .keys()
            .find(|owner| matches!(owner, OwnerKey::Target(_)))
            .unwrap();
        let view = created.repository.view_current().unwrap();
        let draft = render_native_draft(
            &view,
            &[target.into()],
            4 * 1_048_576,
            ExecutionControl::uncancelled(),
        )
        .unwrap();
        let decoded = decode_compact_change_in_view("target.lkjc", &draft, &view).unwrap();
        let mut expected = AuthoredReadFootprint::default();
        expected
            .record_owner(target, view.read_owner(target).unwrap().value.as_ref())
            .unwrap();
        expected
            .record_relations(
                target,
                Some(k::RelationKind::HttpRouteTarget),
                true,
                crate::platform::change::MAXIMUM_AUTHORED_CHANGES,
                &[],
                false,
            )
            .unwrap();
        assert_eq!(decoded.native_reads.len(), 2);
        assert_eq!(decoded.native_reads.digest(), expected.digest());
    }

    #[test]
    fn refresh_base_header_requires_one_bounded_request_with_valid_fields() {
        let base = RevisionId::from_digest([17; 32]);
        assert_eq!(
            compact_change_origin("base.lkjc", format!("request base={base}\n").as_bytes())
                .unwrap(),
            (base, None)
        );
        for (input, code) in [
            (
                format!("request base={base}\nrequest base={base}\n"),
                "change_request_duplicate",
            ),
            (
                format!("request base={base} unknown=value\n"),
                "change_field_unknown",
            ),
        ] {
            assert_eq!(
                compact_change_origin("invalid.lkjc", input.as_bytes()).unwrap_err()[0].code,
                code
            );
        }
    }
}
