//! Complete authored scope and result coordinates. Canonical admission grants authority.
use super::owned::associated_capacity;
use super::*;
use crate::platform::kernel::{
    CallableResult, ReadScopeArgument, ReadScopeBinder, ReadScopeParameter, ReadScopeReference,
    ReadScopeSource, ReadViewDeclaration, ReadViewField, ReadViewFieldKind, ResultRelation,
};
use crate::platform::semantic_id::ReadScopeParameterId;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoredCallableResult<S> {
    pub ty: AuthoredType,
    pub relation: AuthoredResultRelation<S>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthoredResultRelation<S> {
    Value,
    ReadFrom {
        source: S,
    },
    ViewFrom {
        source: S,
        scope: AuthoredReadScopeArgument,
    },
}

impl<S> AuthoredCallableResult<S> {
    pub fn value(ty: AuthoredType) -> Self {
        Self {
            ty,
            relation: AuthoredResultRelation::Value,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthoredReadScopeBinder {
    Declaration(AuthoredDeclarationReference),
    ContractMethod {
        contract: AuthoredDeclarationReference,
        method: crate::platform::semantic_id::MethodId,
    },
    AssociatedMember {
        contract: AuthoredDeclarationReference,
        member: AssociatedTypeMemberId,
    },
    AssociatedBinding {
        implementation: AuthoredDeclarationReference,
        member: AssociatedTypeMemberId,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthoredReadScopeArgument {
    Parameter {
        binder: AuthoredReadScopeBinder,
        parameter: ReadScopeParameterId,
    },
    Lexical {
        callable: AuthoredDeclarationReference,
        binding: AuthoredLocalReference,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoredReadScopeSource<S> {
    pub source: S,
    pub scope: AuthoredReadScopeArgument,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoredReadViewField {
    pub id: FieldId,
    pub name: Name,
    pub ty: AuthoredType,
    pub kind: AuthoredReadViewFieldKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthoredReadViewFieldKind {
    Ordinary,
    ReadReference {
        read_scope: AuthoredReadScopeArgument,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoredReadViewFieldReference {
    pub declaration: AuthoredDeclarationReference,
    pub field: FieldId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoredReadViewFieldInitializer {
    pub field: FieldId,
    pub value: AuthoredExpression,
}

impl<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized> AuthoredLowerer<'_, B, W> {
    pub(super) fn lower_read_scopes(
        &mut self,
        scopes: &[AuthoredReadScopeArgument],
    ) -> Result<Vec<ReadScopeArgument>, Diagnostic> {
        self.base.validation_checkpoint()?;
        self.admit_associated_inventory(scopes.len())?;
        let mut result = Vec::new();
        result
            .try_reserve_exact(scopes.len())
            .map_err(|_| associated_capacity())?;
        for scope in scopes {
            result.push(self.lower_read_scope(scope)?);
        }
        Ok(result)
    }

    pub(super) fn lower_view_field_reference(
        &mut self,
        field: &AuthoredReadViewFieldReference,
    ) -> Result<crate::platform::kernel::ReadViewFieldReference, Diagnostic> {
        Ok(crate::platform::kernel::ReadViewFieldReference {
            declaration: self.lower_declaration_reference(&field.declaration)?,
            field: field.field,
        })
    }

    pub(super) fn lower_view_read_binding(
        &mut self,
        binding: &AuthoredBindingDefinition,
        kind: crate::platform::kernel::BindingKind,
    ) -> Result<BindingId, Diagnostic> {
        let id = self.lexical_binding_symbol(&binding.symbol)?;
        let declared_type = binding
            .declared_type
            .as_ref()
            .map(|ty| self.lower_type(ty))
            .transpose()?;
        self.insert_created(OwnerRecord::Binding(
            crate::platform::kernel::BindingRecord {
                header: OwnerHeader::new(OwnerKey::Binding(id), OwnerKind::Binding),
                name: binding.name.clone(),
                kind,
                value: None,
                declared_type,
            },
        ))?;
        Ok(id)
    }
    pub(super) fn lower_read_scope(
        &mut self,
        scope: &AuthoredReadScopeArgument,
    ) -> Result<ReadScopeArgument, Diagnostic> {
        self.base.validation_checkpoint()?;
        Ok(match scope {
            AuthoredReadScopeArgument::Parameter { binder, parameter } => {
                let binder = match binder {
                    AuthoredReadScopeBinder::Declaration(declaration) => {
                        ReadScopeBinder::Declaration(self.lower_declaration_reference(declaration)?)
                    }
                    AuthoredReadScopeBinder::ContractMethod { contract, method } => {
                        ReadScopeBinder::ContractMethod {
                            contract: self.lower_declaration_reference(contract)?,
                            method: *method,
                        }
                    }
                    AuthoredReadScopeBinder::AssociatedMember { contract, member } => {
                        ReadScopeBinder::AssociatedMember {
                            contract: self.lower_declaration_reference(contract)?,
                            member: *member,
                        }
                    }
                    AuthoredReadScopeBinder::AssociatedBinding {
                        implementation,
                        member,
                    } => ReadScopeBinder::AssociatedBinding {
                        implementation: self.lower_declaration_reference(implementation)?,
                        member: *member,
                    },
                };
                ReadScopeArgument::Parameter(ReadScopeReference {
                    binder,
                    parameter: *parameter,
                })
            }
            AuthoredReadScopeArgument::Lexical { callable, binding } => {
                let LocalValueReference::LexicalBinding(binding) =
                    self.lower_local_reference(binding)?
                else {
                    return Err(request_error(
                        DiagnosticClass::Semantic,
                        "change_authored_read_scope_binding",
                        "a lexical read scope requires an exact lexical binding",
                    ));
                };
                ReadScopeArgument::Lexical {
                    callable: self.lower_declaration_reference(callable)?,
                    binding,
                }
            }
        })
    }

    pub(super) fn lower_named_result(
        &mut self,
        result: &AuthoredCallableResult<AuthoredLocalReference>,
    ) -> Result<CallableResult<ParameterId>, Diagnostic> {
        let ty = self.lower_type(&result.ty)?;
        let relation = match &result.relation {
            AuthoredResultRelation::Value => ResultRelation::Value,
            AuthoredResultRelation::ReadFrom { source } => ResultRelation::ReadFrom {
                source: self.lower_result_borrow(source)?,
            },
            AuthoredResultRelation::ViewFrom { source, scope } => ResultRelation::ViewFrom {
                source: self.lower_result_borrow(source)?,
                scope: self.lower_read_scope(scope)?,
            },
        };
        Ok(CallableResult { ty, relation })
    }

    pub(super) fn lower_method_result(
        &mut self,
        result: &AuthoredCallableResult<u32>,
    ) -> Result<CallableResult<u32>, Diagnostic> {
        let ty = self.lower_type(&result.ty)?;
        let relation = match &result.relation {
            AuthoredResultRelation::Value => ResultRelation::Value,
            AuthoredResultRelation::ReadFrom { source } => {
                ResultRelation::ReadFrom { source: *source }
            }
            AuthoredResultRelation::ViewFrom { source, scope } => ResultRelation::ViewFrom {
                source: *source,
                scope: self.lower_read_scope(scope)?,
            },
        };
        Ok(CallableResult { ty, relation })
    }

    pub(super) fn lower_named_scope_sources(
        &mut self,
        sources: &[AuthoredReadScopeSource<AuthoredLocalReference>],
    ) -> Result<Vec<ReadScopeSource<ParameterId>>, Diagnostic> {
        self.base.validation_checkpoint()?;
        self.admit_associated_inventory(sources.len())?;
        let mut result = Vec::new();
        result
            .try_reserve_exact(sources.len())
            .map_err(|_| associated_capacity())?;
        for source in sources {
            self.base.validation_checkpoint()?;
            result.push(ReadScopeSource {
                source: self.lower_result_borrow(&source.source)?,
                scope: self.lower_read_scope(&source.scope)?,
            });
        }
        Ok(result)
    }

    pub(super) fn lower_method_scope_sources(
        &mut self,
        sources: &[AuthoredReadScopeSource<u32>],
    ) -> Result<Vec<ReadScopeSource<u32>>, Diagnostic> {
        self.base.validation_checkpoint()?;
        self.admit_associated_inventory(sources.len())?;
        let mut result = Vec::new();
        result
            .try_reserve_exact(sources.len())
            .map_err(|_| associated_capacity())?;
        for source in sources {
            self.base.validation_checkpoint()?;
            result.push(ReadScopeSource {
                source: source.source,
                scope: self.lower_read_scope(&source.scope)?,
            });
        }
        Ok(result)
    }

    pub(super) fn lower_read_scope_parameters(
        &mut self,
        parameters: &[ReadScopeParameter],
    ) -> Result<Vec<ReadScopeParameter>, Diagnostic> {
        self.base.validation_checkpoint()?;
        self.admit_associated_inventory(parameters.len())?;
        let mut result = Vec::new();
        result
            .try_reserve_exact(parameters.len())
            .map_err(|_| associated_capacity())?;
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        for parameter in parameters {
            self.base.validation_checkpoint()?;
            if !ids.insert(parameter.id) || !names.insert(&parameter.name) {
                return Err(request_error(
                    DiagnosticClass::Semantic,
                    "change_authored_read_scope_duplicate",
                    "read scope formal identities and names must be distinct",
                ));
            }
            result.push(parameter.clone());
        }
        Ok(result)
    }

    pub(super) fn lower_view_field_kind(
        &mut self,
        kind: &AuthoredReadViewFieldKind,
    ) -> Result<ReadViewFieldKind, Diagnostic> {
        Ok(match kind {
            AuthoredReadViewFieldKind::Ordinary => ReadViewFieldKind::Ordinary,
            AuthoredReadViewFieldKind::ReadReference { read_scope } => {
                ReadViewFieldKind::ReadReference {
                    read_scope: self.lower_read_scope(read_scope)?,
                }
            }
        })
    }

    pub(super) fn lower_view_fields(
        &mut self,
        fields: &[AuthoredReadViewField],
    ) -> Result<Vec<ReadViewField>, Diagnostic> {
        self.base.validation_checkpoint()?;
        self.admit_associated_inventory(fields.len())?;
        let mut result = Vec::new();
        result
            .try_reserve_exact(fields.len())
            .map_err(|_| associated_capacity())?;
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        for field in fields {
            self.base.validation_checkpoint()?;
            if !ids.insert(field.id) || !names.insert(&field.name) {
                return Err(request_error(
                    DiagnosticClass::Semantic,
                    "change_authored_read_view_field_duplicate",
                    "read view field identities and names must be distinct",
                ));
            }
            result.push(ReadViewField {
                id: field.id,
                name: field.name.clone(),
                ty: self.lower_type(&field.ty)?,
                kind: self.lower_view_field_kind(&field.kind)?,
            });
        }
        Ok(result)
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "one complete inline view declaration owns these fields"
    )]
    pub(in crate::platform::change::request) fn lower_create_read_view(
        &mut self,
        symbol: &str,
        module: &ModuleSelector,
        name: &Name,
        visibility: DeclarationVisibility,
        type_parameters: &[AuthoredTypeParameter],
        scope_parameter: &ReadScopeParameter,
        fields: &[AuthoredReadViewField],
    ) -> Result<(), Diagnostic> {
        let declaration = self.declaration_symbol(symbol)?;
        let type_parameters =
            super::declarations::lower_nominal_parameters(self, declaration, type_parameters)?;
        let module = self.resolve_module(module)?;
        let fields = self.lower_view_fields(fields)?;
        self.insert_created(OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(declaration), OwnerKind::ReadView),
            module,
            name: name.clone(),
            visibility,
            payload: DeclarationPayload::ReadView(ReadViewDeclaration {
                type_parameters,
                scope_parameter: scope_parameter.clone(),
                fields,
            }),
        }))
    }

    pub(in crate::platform::change::request) fn lower_set_read_view(
        &mut self,
        selector: &DeclarationSelector,
        type_parameters: &[AuthoredTypeParameterReference],
        scope_parameter: &ReadScopeParameter,
        fields: &[AuthoredReadViewField],
    ) -> Result<(), Diagnostic> {
        let declaration = self.resolve_declaration(selector)?;
        self.admit_associated_inventory(type_parameters.len())?;
        let mut lowered_parameters = Vec::new();
        lowered_parameters
            .try_reserve_exact(type_parameters.len())
            .map_err(|_| associated_capacity())?;
        for parameter in type_parameters {
            self.base.validation_checkpoint()?;
            lowered_parameters.push(self.lower_type_parameter_reference(parameter)?);
        }
        let type_parameters = lowered_parameters;
        let fields = self.lower_view_fields(fields)?;
        let OwnerRecord::Declaration(record) =
            self.candidate_mut(OwnerKey::Declaration(declaration))?
        else {
            return Err(request_error(
                DiagnosticClass::Semantic,
                "change_read_view_kind",
                "read view update requires its exact declaration",
            ));
        };
        if !matches!(record.payload, DeclarationPayload::ReadView(_)) {
            return Err(request_error(
                DiagnosticClass::Semantic,
                "change_read_view_kind",
                "read view update cannot change another declaration category",
            ));
        }
        record.payload = DeclarationPayload::ReadView(ReadViewDeclaration {
            type_parameters,
            scope_parameter: scope_parameter.clone(),
            fields,
        });
        Ok(())
    }
}
