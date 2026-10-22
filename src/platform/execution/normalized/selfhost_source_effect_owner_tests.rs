//! Rehashed owners reach endpoint, containment and namespace layers.
use super::*;

fn requirement_key(source: &KernelSnapshot, name: &str) -> OwnerKey {
    let parent = declaration(source, "clocks");
    owner_key(source, |owner| {
        matches!(owner, OwnerRecord::Requirement(record)
        if record.declaration == parent && record.name.as_str() == name)
    })
}

#[test]
fn selfhost_source_effect_owners_reject_incompatible_and_missing_endpoints() {
    let base = fixture();
    for requirement_parameter in [false, true] {
        let mut source = base.clone();
        let first = declaration(&source, "first");
        if requirement_parameter {
            let key = OwnerKey::RequirementParameter(
                function(&source, "first").requirement_parameters[0],
            );
            let OwnerRecord::RequirementParameter(record) = source.owners.get_mut(&key).unwrap()
            else {
                panic!("R")
            };
            record.constraint.interface.declaration = first;
        } else {
            let key = requirement_key(&source, "clock");
            let OwnerRecord::Requirement(record) = source.owners.get_mut(&key).unwrap() else {
                panic!("requirement")
            };
            record.interface.declaration = first;
        }
        expect(&source, 14, None);
    }
    for requirement_parameter in [false, true] {
        let mut source = base.clone();
        let clock = declaration(&source, "Clock");
        if requirement_parameter {
            let key = OwnerKey::RequirementParameter(
                function(&source, "first").requirement_parameters[0],
            );
            let OwnerRecord::RequirementParameter(record) = source.owners.get_mut(&key).unwrap()
            else {
                panic!("R")
            };
            record.declaration = clock;
        } else {
            let key = OwnerKey::EffectParameter(function(&source, "first").effect_parameters[0]);
            let OwnerRecord::EffectParameter(record) = source.owners.get_mut(&key).unwrap() else {
                panic!("E")
            };
            record.declaration = clock;
        }
        expect(&source, 14, None);
    }
    for key in [
        OwnerKey::Operation(operation(&base, "Clock", "now")),
        requirement_key(&base, "other"),
        OwnerKey::RequirementParameter(function(&base, "first").requirement_parameters[0]),
        OwnerKey::EffectParameter(function(&base, "first").effect_parameters[0]),
    ] {
        let mut source = base.clone();
        assert!(source.owners.remove(&key).is_some());
        expect(&source, 14, None);
    }
}

#[test]
fn selfhost_source_effect_constraints_require_exact_interface_membership() {
    let base = fixture();
    let foreign = operation(&base, "OtherClock", "now");
    for parameter in [false, true] {
        let mut source = base.clone();
        let operations = if parameter {
            let key = OwnerKey::RequirementParameter(
                function(&source, "first").requirement_parameters[0],
            );
            let OwnerRecord::RequirementParameter(record) = source.owners.get_mut(&key).unwrap()
            else {
                panic!("R")
            };
            &mut record.constraint.operations
        } else {
            let key = requirement_key(&source, "clock");
            let OwnerRecord::Requirement(record) = source.owners.get_mut(&key).unwrap() else {
                panic!("requirement")
            };
            &mut record.operations
        };
        operations[1].operation = foreign;
        operations.sort_unstable();
        expect(&source, 14, Some("kernel_full_requirement_operation_owner"));
    }
}

#[test]
fn selfhost_source_effect_owners_reject_reverse_parent_mismatches() {
    let base = fixture();
    let mut source = base.clone();
    let clock = declaration(&source, "Clock");
    // This unused operation is not referenced by a requirement constraint.
    let other = operation(&source, "OtherClock", "now");
    let OwnerRecord::Operation(record) =
        source.owners.get_mut(&OwnerKey::Operation(other)).unwrap()
    else {
        panic!("operation")
    };
    record.declaration = clock;
    expect(&source, 15, None);

    let mut source = base.clone();
    let tick = operation(&source, "Clock", "tick");
    let now = operation(&source, "Clock", "now");
    let key = parameter_key(&source, kernel::ParameterParent::Operation(tick), "left");
    let OwnerRecord::Parameter(record) = source.owners.get_mut(&key).unwrap() else {
        panic!("parameter")
    };
    record.parent = kernel::ParameterParent::Operation(now);
    expect(&source, 15, None);

    let mut source = base.clone();
    let other_clocks = declaration(&source, "other-clocks");
    let key = requirement_key(&source, "other");
    let OwnerRecord::Requirement(record) = source.owners.get_mut(&key).unwrap() else {
        panic!("requirement")
    };
    record.declaration = other_clocks;
    expect(&source, 15, None);

    for parameter in [false, true] {
        let mut source = base.clone();
        let second = declaration(&source, "second");
        if parameter {
            let key = OwnerKey::RequirementParameter(
                function(&source, "first").requirement_parameters[0],
            );
            let OwnerRecord::RequirementParameter(record) = source.owners.get_mut(&key).unwrap()
            else {
                panic!("R")
            };
            record.declaration = second;
        } else {
            let key = OwnerKey::EffectParameter(function(&source, "first").effect_parameters[0]);
            let OwnerRecord::EffectParameter(record) = source.owners.get_mut(&key).unwrap() else {
                panic!("E")
            };
            record.declaration = second;
        }
        expect(&source, 15, None);
    }
}

#[test]
fn selfhost_source_effect_owners_require_exact_forward_membership() {
    let base = fixture();
    for parameter in [false, true] {
        let mut source = base.clone();
        let function = function_mut(&mut source, "first");
        if parameter {
            function.requirement_parameters.pop();
        } else {
            function.effect_parameters.pop();
        }
        expect(&source, 15, None);
    }
    let mut source = base;
    let clock = declaration(&source, "Clock");
    let OwnerRecord::Declaration(record) = source
        .owners
        .get_mut(&OwnerKey::Declaration(clock))
        .unwrap()
    else {
        panic!("interface")
    };
    let kernel::DeclarationPayload::Interface { operations } = &mut record.payload else {
        panic!("interface")
    };
    operations.pop();
    expect(&source, 15, None);
}

#[test]
fn selfhost_source_effect_owners_cover_all_twelve_namespace_classes() {
    use kernel::NamespaceClass;
    let base = byte_buffer_tests::author_only(&format!(
        "{}\n{}",
        include_str!("../../../../tests/fixtures/selfhost-source-namespace.lkjc"),
        include_str!("../../../../tests/fixtures/selfhost-source-effect-owners.lkjc")
    ))
    .unwrap();
    expect(&base, 0, None);
    for class in [
        NamespaceClass::Module,
        NamespaceClass::Declaration,
        NamespaceClass::TypeParameter,
        NamespaceClass::Field,
        NamespaceClass::Case,
        NamespaceClass::Operation,
        NamespaceClass::Parameter,
        NamespaceClass::Requirement,
        NamespaceClass::Port,
        NamespaceClass::Target,
        NamespaceClass::EffectParameter,
        NamespaceClass::RequirementParameter,
    ] {
        let mut groups = BTreeMap::<Option<OwnerKey>, Vec<(OwnerKey, kernel::Name)>>::new();
        for (key, record) in &base.owners {
            if let Some(entry) = kernel::owner_namespace(record)
                && entry.class == class
            {
                groups
                    .entry(entry.parent)
                    .or_default()
                    .push((*key, entry.name.clone()));
            }
        }
        let pair = groups.values().find(|owners| owners.len() >= 2).unwrap();
        let mut changed = base.clone();
        *name_mut(changed.owners.get_mut(&pair[1].0).unwrap()) = pair[0].1.clone();
        expect(&changed, 16, Some("kernel_full_namespace_duplicate"));
    }
    // The shared Parameter class also uses Operation as an exact parent domain.
    let mut source = fixture();
    let tick = operation(&source, "Clock", "tick");
    let right = parameter_key(&source, kernel::ParameterParent::Operation(tick), "right");
    *name_mut(source.owners.get_mut(&right).unwrap()) = kernel::Name::new("left").unwrap();
    expect(&source, 16, Some("kernel_full_namespace_duplicate"));
}

#[test]
fn selfhost_source_effect_owners_preserve_class_separation_and_full_u64_limits() {
    let base = fixture();
    let mut source = base.clone();
    let effect = OwnerKey::EffectParameter(function(&source, "first").effect_parameters[0]);
    let requirement =
        OwnerKey::RequirementParameter(function(&source, "first").requirement_parameters[0]);
    let name = name_mut(source.owners.get_mut(&requirement).unwrap()).clone();
    *name_mut(source.owners.get_mut(&effect).unwrap()) = name;
    expect(&source, 0, None);
    for maximum in [i64::MAX as u64 + 1, u64::MAX] {
        let mut source = base.clone();
        let key = requirement_key(&source, "clock");
        let OwnerRecord::Requirement(record) = source.owners.get_mut(&key).unwrap() else {
            panic!("requirement")
        };
        assert_eq!(record.limits.len(), 1);
        record.limits[0].maximum = maximum;
        expect(&source, 0, None);
    }
}

#[test]
fn selfhost_source_effect_owners_require_compatible_operation_and_requirement_parents() {
    let mut source = fixture();
    let first = declaration(&source, "first");
    let key = OwnerKey::Operation(operation(&source, "OtherClock", "now"));
    let OwnerRecord::Operation(record) = source.owners.get_mut(&key).unwrap() else {
        panic!("operation")
    };
    record.declaration = first;
    expect(&source, 14, None);

    let mut source = fixture();
    let clock = declaration(&source, "Clock");
    let key = requirement_key(&source, "other");
    let OwnerRecord::Requirement(record) = source.owners.get_mut(&key).unwrap() else {
        panic!("requirement")
    };
    record.declaration = clock;
    expect(&source, 14, None);
}
