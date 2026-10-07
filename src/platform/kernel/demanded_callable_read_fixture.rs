//! Two packages: F<T; P0..Pn> invokes every Pi; S<X> maps its method back to
//! F<X; S<X>..S<X>>. Self is a fixed cell, so phantom X stays independently typed.
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::*;
use crate::platform::semantic_id::{
    DeclarationId, ExpressionId, ImplementationParameterId, MethodId, ModuleId, TypeParameterId,
};
use std::cell::Cell;
use std::collections::BTreeMap;

#[path = "demanded_callable_read_source.rs"]
mod source;

const SEED: &[u8] = b"demanded-call-input-regression";

pub(super) struct Fixture {
    owners: BTreeMap<(PackageId, OwnerKey), OwnerRecord>,
    types: BTreeMap<TypeObjectDigest, TypeObject>,
    mapping: DeclarationReference,
    pub(super) mapping_reads: Cell<usize>,
    pub(super) proof_checkpoints: Cell<usize>,
    stop_at: Cell<Option<usize>>,
}

impl Fixture {
    pub(super) fn reset(&self, stop_at: Option<usize>) {
        self.mapping_reads.set(0);
        self.proof_checkpoints.set(0);
        self.stop_at.set(stop_at);
    }

    fn put(&mut self, package: PackageId, record: OwnerRecord) {
        self.owners.insert((package, record.owner()), record);
    }

    fn ty(&mut self, form: TypeForm) -> Result<TypeObjectDigest, Diagnostic> {
        let object = TypeObject::new(form)?;
        let (digest, _) = encode_type_object(&object)?;
        self.types.insert(digest, object);
        Ok(digest)
    }

    fn parameter(
        &mut self,
        owner: DeclarationReference,
        ordinal: u64,
    ) -> Result<TypeParameterId, Diagnostic> {
        let parameter = TypeParameterId::migrate(SEED, ordinal);
        self.put(
            owner.package,
            OwnerRecord::TypeParameter(TypeParameterRecord {
                header: OwnerHeader::new(
                    OwnerKey::TypeParameter(parameter),
                    OwnerKind::TypeParameter,
                ),
                declaration: owner.declaration,
                name: Name::new(format!("T{ordinal}"))?,
                constraints: TypeParameterConstraints::Owned,
            }),
        );
        Ok(parameter)
    }

    fn declaration(
        &mut self,
        owner: DeclarationReference,
        kind: OwnerKind,
        payload: DeclarationPayload,
    ) -> Result<(), Diagnostic> {
        self.put(
            owner.package,
            OwnerRecord::Declaration(DeclarationRecord {
                header: OwnerHeader::new(OwnerKey::Declaration(owner.declaration), kind),
                module: ModuleId::migrate(SEED, 1),
                name: Name::new(format!("d{}", self.owners.len()))?,
                visibility: DeclarationVisibility::Public,
                payload,
            }),
        );
        Ok(())
    }

    pub(super) fn new(width: usize, growing_operand: Option<usize>) -> Result<Self, Diagnostic> {
        let caller = DeclarationReference {
            package: PackageId::migrate(SEED, 1),
            declaration: DeclarationId::migrate(SEED, 1),
        };
        let mapping = DeclarationReference {
            package: PackageId::migrate(SEED, 2),
            declaration: DeclarationId::migrate(SEED, 2),
        };
        let contract = DeclarationReference {
            package: mapping.package,
            declaration: DeclarationId::migrate(SEED, 3),
        };
        let mut fixture = Self {
            owners: BTreeMap::new(),
            types: BTreeMap::new(),
            mapping,
            mapping_reads: Cell::new(0),
            proof_checkpoints: Cell::new(0),
            stop_at: Cell::new(None),
        };
        let result = fixture.ty(TypeForm::I64)?;
        let cell = fixture.ty(TypeForm::OwnedI64Cell)?;
        let method = MethodId::migrate(SEED, 1);
        let self_parameter = fixture.parameter(contract, 3)?;
        fixture.declaration(
            contract,
            OwnerKind::OwnedContract,
            DeclarationPayload::OwnedContract(OwnedContract {
                self_parameter,
                type_parameters: Vec::new(),
                methods: vec![OwnedMethod {
                    id: method,
                    name: Name::new("read")?,
                    parameters: Vec::new(),
                    result,
                    result_borrow: None,
                    effect: FunctionEffect::Pure,
                }],
            }),
        )?;
        let parameter = fixture.parameter(caller, 1)?;
        let mut prerequisites = Vec::new();
        let mut items = Vec::new();
        for index in 0..width {
            let id = ImplementationParameterId::migrate(SEED, index as u64 + 1);
            prerequisites.push(ImplementationParameter {
                id,
                name: Name::new(format!("P{index}"))?,
                contract,
                self_type: cell,
                type_arguments: Vec::new(),
            });
            let expression = ExpressionId::migrate(SEED, index as u64 + 1);
            fixture.put(
                caller.package,
                OwnerRecord::Expression(ExpressionRecord::new(
                    expression,
                    ExpressionOperation::MethodCall {
                        witness: ImplementationOperand::Parameter {
                            scope: caller,
                            parameter: id,
                        },
                        contract,
                        method,
                        arguments: Vec::new(),
                    },
                )?),
            );
            items.push(expression);
        }
        let body = ExpressionId::migrate(SEED, 10_000);
        let operation = if items.is_empty() {
            ExpressionOperation::I64 { value: 0 }
        } else {
            ExpressionOperation::Sequence { items }
        };
        fixture.put(
            caller.package,
            OwnerRecord::Expression(ExpressionRecord::new(body, operation)?),
        );
        fixture.declaration(
            caller,
            OwnerKind::PureFunction,
            DeclarationPayload::Function(FunctionDeclaration {
                implementation_parameters: prerequisites,
                requirement_parameters: Vec::new(),
                effect_parameters: Vec::new(),
                type_parameters: vec![parameter],
                parameters: Vec::new(),
                result,
                result_borrow: None,
                effect: FunctionEffect::Pure,
                body,
            }),
        )?;
        let parameter = fixture.parameter(mapping, 2)?;
        let variable = fixture.ty(TypeForm::TypeParameter { parameter })?;
        let nested = fixture.ty(TypeForm::OwnedSequence { item: variable })?;
        let operands = (0..width)
            .map(|index| ImplementationOperand::Concrete {
                implementation: mapping,
                type_arguments: vec![if growing_operand == Some(index) {
                    nested
                } else {
                    variable
                }],
                implementations: Vec::new(),
            })
            .collect();
        fixture.declaration(
            mapping,
            OwnerKind::OwnedImplementation,
            DeclarationPayload::OwnedImplementation(OwnedImplementation {
                type_parameters: vec![parameter],
                implementation_parameters: Vec::new(),
                contract,
                self_type: cell,
                type_arguments: Vec::new(),
                methods: vec![OwnedMethodImplementation {
                    method,
                    function: caller,
                    type_arguments: vec![variable],
                    implementations: operands,
                }],
            }),
        )?;
        Ok(fixture)
    }
}
