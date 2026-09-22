//! Complete canonical reads for disposable declaration proposals, with the maintained aggregate
//! definition admission. No historical authoring source or rendered inspection text is consumed.
use super::*;
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
        }
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
        let owner = self
            .reader
            .owner(id)?
            .ok_or_else(|| error(format!("selected owner {id} is absent at the bound base")))?;
        self.owners.insert(id, owner.clone());
        Ok(owner)
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
        let operation = match expression.operation {
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
