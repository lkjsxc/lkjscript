use crate::codegen::*;

pub(crate) fn compile_program(verified: &VerifiedProgram) -> Result<(Chunk, BytecodeLinkMetadata)> {
    let program = verified.program();
    let mut chunk = Chunk::new();
    chunk.main.name = "main".into();
    install_enum_metadata(&mut chunk, program)?;
    for product in &program.products {
        if product.id.index() != Some(chunk.products.len()) {
            return Err(Error::msg(
                "SSA product IDs are inconsistent during bytecode lowering",
            ));
        }
        let region = program
            .region_products
            .iter()
            .any(|metadata| metadata.product == product.id);
        chunk.products.push(BytecodeProductMetadata {
            id: BytecodeProductId::new(product.id.raw()),
            identity: lkjscript_core::RuntimeLayoutId::new(
                lkjscript_ir::runtime_product_identity(program, product.id)
                    .map_err(|error| Error::msg(error.to_string()))?
                    .bytes(),
            ),
            region,
            name: product.name.clone(),
            fields: product
                .fields
                .iter()
                .map(|field| field.name.clone())
                .collect(),
            region_fields: if region {
                product
                    .fields
                    .iter()
                    .map(|field| region_product_field_kind(program, &field.ty))
                    .collect::<Result<Vec<_>>>()?
            } else {
                Vec::new()
            },
        });
    }
    install_structural_metadata(&mut chunk, program)?;

    let mut globals = HashMap::new();
    globals
        .try_reserve(program.functions.len())
        .map_err(|_| Error::host("bytecode global mapping allocation failed"))?;
    let mut prototypes = HashMap::new();
    prototypes
        .try_reserve(program.functions.len())
        .map_err(|_| Error::host("bytecode prototype mapping allocation failed"))?;
    for function in &program.functions {
        if function.id == program.main {
            continue;
        }
        let slot = chunk.intern_global(&function.name)?;
        if globals.insert(function.id, slot).is_some() {
            return Err(Error::msg("duplicate SSA function global mapping"));
        }
        let prototype = u64::try_from(prototypes.len())
            .map_err(|_| Error::host("bytecode prototype identity exceeds u64"))?;
        if prototypes.insert(function.id, prototype).is_some() {
            return Err(Error::msg("duplicate SSA function prototype mapping"));
        }
        let global_index = slot
            .index()
            .ok_or_else(|| Error::msg("bytecode global identity exceeds host usize"))?;
        let metadata = chunk
            .global_prototypes
            .get_mut(global_index)
            .ok_or_else(|| Error::msg("bytecode global prototype metadata is missing"))?;
        if metadata.replace(prototype).is_some() {
            return Err(Error::msg("duplicate bytecode global function name"));
        }
    }

    chunk
        .protos
        .try_reserve_exact(prototypes.len())
        .map_err(|_| Error::host("bytecode prototype table allocation failed"))?;
    let mut links = Vec::new();
    links
        .try_reserve_exact(program.functions.len())
        .map_err(|_| Error::host("bytecode function-link reservation failed"))?;
    for function in &program.functions {
        if function.id == program.main {
            continue;
        }
        let prototype = prototypes.get(&function.id).copied().ok_or_else(|| {
            Error::msg(format!(
                "SSA function {} has no bytecode prototype mapping",
                function.id.raw()
            ))
        })?;
        if usize::try_from(prototype).ok() != Some(chunk.protos.len()) {
            return Err(Error::msg("SSA prototype mapping is not dense"));
        }
        let (proto, mut link) =
            compile_function(&mut chunk, &globals, function, 0, Some(prototype))?;
        link.prototype = Some(prototype);
        chunk.protos.push(proto);
        links.push(link);
    }

    for function in &program.functions {
        if function.id == program.main {
            continue;
        }
        let prototype = prototypes
            .get(&function.id)
            .copied()
            .ok_or_else(|| Error::msg("SSA closure installation has no prototype mapping"))?;
        let global = globals
            .get(&function.id)
            .copied()
            .ok_or_else(|| Error::msg("SSA closure installation has no global mapping"))?;
        let constant = add_constant(&mut chunk, BytecodeConstant::Proto(prototype))?;
        chunk.main.try_emit_op_u64(Op::LoadConst, constant.0)?;
        chunk.main.try_emit_op_u64(Op::MakeClosure, 0)?;
        chunk.main.try_emit_op_u64(Op::StoreGlobal, global.0)?;
        chunk.main.try_emit(Op::Pop)?;
    }

    let main = program
        .functions
        .get(program.main.index().unwrap_or(usize::MAX))
        .filter(|function| function.id == program.main)
        .ok_or_else(|| Error::msg("SSA main function is missing"))?;
    chunk.required_capabilities = main
        .signature
        .parameters
        .iter()
        .map(|ty| match ty {
            SsaType::Capability(kind) => Ok(*kind),
            _ => Err(Error::msg(
                "SSA main parameters must be exact capability types",
            )),
        })
        .collect::<Result<Vec<_>>>()?;
    let code_base = u64::try_from(chunk.main.len())
        .map_err(|_| Error::msg("bytecode main closure prelude exceeds u64"))?;
    let (main_proto, main_link) = compile_function(&mut chunk, &globals, main, code_base, None)?;
    chunk.main.locals = main_proto.locals;
    chunk.main.arity = main_proto.arity;
    chunk.main.memory_plan = main_proto.memory_plan;
    chunk.main.memory_witness_parameters = main_proto.memory_witness_parameters;
    chunk.main.call_witnesses = main_proto.call_witnesses;
    chunk.main.parameter_structurals = main_proto.parameter_structurals;
    chunk.main.parameter_structural_places = main_proto.parameter_structural_places;
    chunk.main.parameter_type_variables = main_proto.parameter_type_variables;
    chunk.main.parameter_copy_kinds = main_proto.parameter_copy_kinds;
    chunk.main.return_copy_kind = main_proto.return_copy_kind;
    chunk.main.parameter_region_products = main_proto.parameter_region_products;
    chunk.main.return_region_product = main_proto.return_region_product;
    chunk.main.return_structural = main_proto.return_structural;
    chunk.main.return_type_variable = main_proto.return_type_variable;
    chunk.main.parameter_resources = main_proto.parameter_resources;
    chunk.main.parameter_resource_places = main_proto.parameter_resource_places;
    chunk.main.return_resource = main_proto.return_resource;
    chunk.main.parameter_uniques = main_proto.parameter_uniques;
    chunk.main.parameter_unique_places = main_proto.parameter_unique_places;
    chunk.main.return_unique = main_proto.return_unique;
    chunk.main.unique_places = main_proto.unique_places;
    chunk.main.failure_cleanups = main_proto.failure_cleanups;
    chunk.main.failure_cleanup_ranges = main_proto.failure_cleanup_ranges;
    chunk.main.try_reserve_code(main_proto.code.len())?;
    chunk.main.code.extend(main_proto.code);
    links.push(main_link);
    links.sort_by_key(|link| link.function);

    Ok((
        chunk,
        BytecodeLinkMetadata {
            main: program.main,
            functions: links,
        },
    ))
}

fn region_product_field_kind(
    program: &lkjscript_ir::Program,
    ty: &SsaType,
) -> Result<RegionProductFieldKind> {
    Ok(match ty {
        SsaType::Unit => RegionProductFieldKind::Unit,
        SsaType::Bool => RegionProductFieldKind::Bool,
        SsaType::I64 => RegionProductFieldKind::I64,
        SsaType::F64 => RegionProductFieldKind::F64,
        SsaType::List(inner)
            if matches!(
                inner.as_ref(),
                SsaType::Unit | SsaType::Bool | SsaType::I64 | SsaType::F64
            ) =>
        {
            RegionProductFieldKind::List
        }
        SsaType::Product(product)
            if program
                .region_products
                .iter()
                .any(|metadata| metadata.product == *product) =>
        {
            RegionProductFieldKind::Product(lkjscript_core::ProductId::new(product.raw()))
        }
        _ => return Err(Error::msg("region product field route is unsupported")),
    })
}
