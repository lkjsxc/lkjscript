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
        chunk.products.push(BytecodeProductMetadata {
            id: BytecodeProductId::new(product.id.raw()),
            name: product.name.clone(),
            fields: product
                .fields
                .iter()
                .map(|field| field.name.clone())
                .collect(),
        });
    }

    let mut globals = HashMap::new();
    let mut prototypes = HashMap::new();
    for function in &program.functions {
        if function.id == program.main {
            continue;
        }
        let slot = u16::try_from(chunk.global_names.len())
            .map_err(|_| Error::msg("too many SSA functions for bytecode globals"))?;
        globals.insert(function.id, slot);
        chunk.global_names.push(function.name.clone());
        let prototype = u32::try_from(prototypes.len())
            .map_err(|_| Error::msg("too many SSA functions for bytecode prototypes"))?;
        prototypes.insert(function.id, prototype);
        chunk.global_prototypes.push(Some(prototype));
    }

    let mut links = Vec::with_capacity(program.functions.len());
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
        chunk.main.emit_op_u16(Op::LoadConst, constant);
        chunk.main.emit_op_u16(Op::MakeClosure, 0);
        chunk.main.emit_op_u16(Op::StoreGlobal, global);
        chunk.main.emit(Op::Pop);
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
    let code_base = u16::try_from(chunk.main.len())
        .map_err(|_| Error::msg("bytecode main closure prelude exceeds u16"))?;
    let (main_proto, main_link) = compile_function(&mut chunk, &globals, main, code_base, None)?;
    chunk.main.locals = main_proto.locals;
    chunk.main.arity = main_proto.arity;
    chunk.main.parameter_resources = main_proto.parameter_resources;
    chunk.main.return_resource = main_proto.return_resource;
    chunk.main.parameter_uniques = main_proto.parameter_uniques;
    chunk.main.parameter_unique_places = main_proto.parameter_unique_places;
    chunk.main.return_unique = main_proto.return_unique;
    chunk.main.unique_places = main_proto.unique_places;
    chunk.main.failure_cleanups = main_proto.failure_cleanups;
    chunk.main.failure_cleanup_ranges = main_proto.failure_cleanup_ranges;
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
