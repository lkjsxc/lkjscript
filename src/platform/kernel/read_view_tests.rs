//! Independent controls for finite symbolic scopes and the single callable result authority.
use super::*;
use crate::platform::kernel::{
    DeclarationPayload, DeclarationRecord, DeclarationVisibility, OwnerHeader, OwnerKey, OwnerKind,
    OwnerRecord,
};

fn declaration(serial: u8) -> DeclarationReference {
    DeclarationReference {
        package: "pkg_10000000000000000000000000000001".parse().unwrap(),
        declaration: format!("decl_{serial:032x}").parse().unwrap(),
    }
}

fn formal() -> ReadScopeParameter {
    ReadScopeParameter {
        id: "read_scope_00000000000000000000000000000001"
            .parse()
            .unwrap(),
        name: Name::new("source").unwrap(),
    }
}

fn scope(binder: ReadScopeBinder) -> ReadScopeArgument {
    ReadScopeArgument::Parameter(ReadScopeReference {
        binder,
        parameter: formal().id,
    })
}

#[test]
fn unused_scope_relationship_rejects_equal_id_in_foreign_binder() {
    let own = ReadScopeBinder::Declaration(declaration(1));
    let foreign = ReadScopeBinder::Declaration(declaration(2));
    let sources = [ReadScopeSource {
        source: 0_u32,
        scope: scope(foreign),
    }];
    let error = validate_read_scope_inventory_at(own, &[formal()], &sources, |_| true).unwrap_err();
    assert_eq!(error.code, "kernel_read_view_scope");
}

#[test]
fn scope_aliases_preserve_distinct_sources_and_reject_duplicate_pairs() {
    let binder = ReadScopeBinder::Declaration(declaration(1));
    let first = ReadScopeSource {
        source: 0_u32,
        scope: scope(binder),
    };
    let second = ReadScopeSource {
        source: 1_u32,
        scope: scope(binder),
    };
    validate_read_scope_inventory_at(binder, &[formal()], &[first.clone(), second], |id| id < 2)
        .unwrap();
    let error =
        validate_read_scope_inventory_at(binder, &[formal()], &[first.clone(), first], |_| true)
            .unwrap_err();
    assert_eq!(error.code, "kernel_read_view_source");
}

#[test]
fn fallible_result_mapping_preserves_relation_and_never_invents_source() {
    let binder = ReadScopeBinder::Declaration(declaration(1));
    let result = CallableResult {
        ty: 7_u32,
        relation: ResultRelation::ViewFrom {
            source: 1_u32,
            scope: scope(binder),
        },
    };
    let error = result
        .try_map_source::<u64>(|_| Err(view_error("mapping_refused", "missing exact argument")))
        .unwrap_err();
    assert_eq!(error.code, "mapping_refused");
    let mapped = result
        .try_map_source(|source| Ok(u64::from(source) + 10))
        .unwrap()
        .try_map_type(|ty| Ok(u64::from(ty)))
        .unwrap();
    assert_eq!(mapped.ty, 7_u64);
    assert_eq!(
        mapped.relation,
        ResultRelation::ViewFrom {
            source: 11_u64,
            scope: scope(binder)
        }
    );
    let value = CallableResult {
        ty: 7_u32,
        relation: ResultRelation::<u32>::Value,
    };
    let mapped = value
        .try_map_source::<u64>(|_| panic!("Value has no source to map"))
        .unwrap();
    assert_eq!(mapped.relation, ResultRelation::Value);
}

#[test]
fn metadata_only_view_remains_positive_view_with_explicit_scope() {
    let binder = ReadScopeBinder::Declaration(declaration(1));
    let view = ReadViewDeclaration {
        type_parameters: vec![],
        scope_parameter: formal(),
        fields: vec![],
    };
    view.validate_local_at(declaration(1)).unwrap();
    let owner = OwnerRecord::Declaration(DeclarationRecord {
        header: OwnerHeader::new(
            OwnerKey::Declaration(declaration(1).declaration),
            OwnerKind::ReadView,
        ),
        module: crate::platform::semantic_id::ModuleId::migrate(b"read-view-owner-domain", 0),
        name: Name::new("PublicView").unwrap(),
        visibility: DeclarationVisibility::Public,
        payload: DeclarationPayload::ReadView(view.clone()),
    });
    owner.validate_local().unwrap();
    let mut wrong_domain = owner.clone();
    let OwnerRecord::Declaration(record) = &mut wrong_domain else {
        unreachable!()
    };
    record.header.owner = OwnerKey::Field(crate::platform::semantic_id::FieldId::migrate(
        b"read-view-owner-domain",
        0,
    ));
    assert_eq!(
        wrong_domain.validate_local().unwrap_err().code,
        "kernel_owner_identity_domain"
    );
    let mut wrong_kind = owner.clone();
    let OwnerRecord::Declaration(record) = &mut wrong_kind else {
        unreachable!()
    };
    record.header.kind = OwnerKind::Record;
    assert_eq!(
        wrong_kind.validate_local().unwrap_err().code,
        "kernel_owner_header"
    );
    let mut predecessor = owner;
    let OwnerRecord::Declaration(record) = &mut predecessor else {
        unreachable!()
    };
    record.header.contract_version = 33;
    assert_eq!(
        predecessor.validate_local().unwrap_err().code,
        "kernel_owner_header"
    );
    let form = super::super::TypeForm::ReadViewApplied {
        declaration: declaration(1),
        arguments: vec![],
        read_scope: scope(binder),
    };
    assert!(form.is_read_view());
    assert!(!form.is_owned_associated());
    let object = super::super::TypeObject::new(form).unwrap();
    assert_eq!(object.child_type_count(), 0);
    assert_eq!(
        object.contract_version,
        super::super::contract::TYPE_OBJECT_CONTRACT_VERSION
    );
}
