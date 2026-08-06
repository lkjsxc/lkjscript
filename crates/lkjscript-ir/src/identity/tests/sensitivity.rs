use super::*;

#[test]
fn every_program_table_changes_identity() {
    let baseline = identity(base());

    let mut sources = base();
    sources.sources.push(SourceMetadata {
        id: 0,
        path: "a.lkjscript".into(),
    });
    assert!(baseline != identity(sources));

    let mut products = base();
    products.products.push(ProductMetadata {
        id: ProductId::new(0),
        name: "point".into(),
        fields: Vec::new(),
    });
    assert!(baseline != identity(products));

    let mut enums = base();
    enums.enums.push(EnumMetadata {
        id: EnumId::new([1; 32]),
        name: "choice".into(),
        type_parameters: Vec::new(),
        variants: vec![EnumVariantMetadata {
            id: VariantId::new([2; 32]),
            name: "only".into(),
            source_order: 0,
            physical_tag: 0,
            fields: Vec::new(),
        }],
        layout: EnumLayoutFacts {
            identity: RuntimeLayoutId::new([3; 32]),
            recursive: false,
        },
    });
    assert!(baseline != identity(enums));

    let mut traits = base();
    traits.sources.push(SourceMetadata {
        id: 0,
        path: "a.lkjscript".into(),
    });
    traits.traits.push(TraitMetadata {
        id: TraitId::new(5),
        name: "marker".into(),
        role: TraitRole::User,
        source: Some(0),
    });
    assert!(baseline != identity(traits));

    let mut implementations = base();
    implementations.sources = vec![
        SourceMetadata {
            id: 0,
            path: "a.lkjscript".into(),
        },
        SourceMetadata {
            id: 1,
            path: "b.lkjscript".into(),
        },
    ];
    implementations.products.push(ProductMetadata {
        id: ProductId::new(0),
        name: "point".into(),
        fields: Vec::new(),
    });
    implementations.traits.push(TraitMetadata {
        id: TraitId::new(5),
        name: "marker".into(),
        role: TraitRole::User,
        source: Some(0),
    });
    implementations.implementations.push(ImplMetadata {
        id: ImplId::new(0),
        trait_id: TraitId::new(5),
        product: ProductId::new(0),
        source: 1,
    });
    assert!(baseline != identity(implementations));

    let mut functions = base();
    let mut helper = functions.functions[0].clone();
    helper.id = FunctionId::new(1);
    helper.name = "helper".into();
    functions.functions.push(helper);
    assert!(baseline != identity(functions.clone()));
    functions.main = FunctionId::new(1);
    assert!(identity(base()) != identity(functions));
}

#[test]
fn structural_memory_and_region_product_tables_change_identity() {
    fn regional(plan_byte: u8) -> Program {
        let mut program = base();
        program.memory.plan = MemoryPlanId::new([plan_byte; 32]);
        program.products.push(ProductMetadata {
            id: ProductId::new(0),
            name: "region-value".into(),
            fields: Vec::new(),
        });
        let identity = runtime_product_contract_identity(program.memory.plan, "region-value")
            .expect("identity must derive");
        program.region_products.push(RegionProductMetadata {
            product: ProductId::new(0),
            identity,
        });
        program
    }
    assert!(identity(regional(1)) != identity(regional(2)));
}

#[test]
fn semantic_sequence_order_changes_identity() {
    fn ordered(first: &str, second: &str) -> Program {
        let mut program = base();
        program.sources = vec![
            SourceMetadata {
                id: 0,
                path: first.into(),
            },
            SourceMetadata {
                id: 1,
                path: second.into(),
            },
        ];
        program
    }
    assert!(identity(ordered("a", "b")) != identity(ordered("b", "a")));
}
