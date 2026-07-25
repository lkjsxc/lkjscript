use std::collections::HashSet;

use crate::verify::*;
use crate::{InstructionKind, Program, TraitRole};

pub(crate) fn verify_program(program: &Program) -> crate::Result<()> {
    if program.sources.iter().enumerate().any(|(index, source)| {
        source.id != u32::try_from(index).unwrap_or(u32::MAX) || source.path.is_empty()
    }) {
        return fail("SSA source metadata must have dense IDs and non-empty paths");
    }
    if program
        .products
        .iter()
        .enumerate()
        .any(|(index, product)| product.id.index() != Some(index) || product.name.is_empty())
    {
        return fail("SSA products must have dense IDs and non-empty names");
    }
    let mut product_names = HashSet::new();
    for product in &program.products {
        if !product_names.insert(product.name.as_str()) {
            return fail(format!("duplicate SSA product name {}", product.name));
        }
        let mut fields = HashSet::new();
        for field in &product.fields {
            if field.name.is_empty() || !fields.insert(field.name.as_str()) {
                return fail(format!(
                    "SSA product {} has an empty or duplicate field",
                    product.name
                ));
            }
            verify_type(program, &field.ty, &[])?;
            if contains_ownership_type(&field.ty) {
                return fail("SSA ownership/reference type cannot be stored in a product field");
            }
        }
    }
    verify_trait_metadata(program)?;
    if program.functions.is_empty() {
        return fail("SSA program has no functions");
    }
    if program
        .main
        .index()
        .is_none_or(|index| index >= program.functions.len())
    {
        return fail("SSA program has an invalid main FunctionId");
    }
    let mut global_loan_ids = HashSet::new();
    let mut function_names = HashSet::new();
    for (index, function) in program.functions.iter().enumerate() {
        if function.id.index() != Some(index) {
            return fail("SSA functions must have dense IDs in storage order");
        }
        if function.name.is_empty() || !function_names.insert(function.name.as_str()) {
            return fail(format!(
                "SSA function {} has an empty or duplicate name",
                function.id.raw()
            ));
        }
        verify_function(program, function)?;
        for instruction in function.blocks.iter().flat_map(|block| &block.instructions) {
            if let InstructionKind::Borrow { loan, .. } = instruction.kind {
                if !global_loan_ids.insert(loan) {
                    return fail("SSA has a duplicate LoanId anywhere in the program");
                }
            }
        }
    }
    let main = function(program, program.main)?;
    if !main.signature.type_parameters.is_empty() || !main.signature.parameters.is_empty() {
        return fail("SSA main must be monomorphic and have no parameters");
    }
    Ok(())
}

pub(crate) fn verify_trait_metadata(program: &Program) -> crate::Result<()> {
    let core = [
        (TraitRole::Copy, "Copy"),
        (TraitRole::Clone, "Clone"),
        (TraitRole::Drop, "Drop"),
        (TraitRole::Send, "Send"),
        (TraitRole::Sync, "Sync"),
    ];
    if program.traits.len() < core.len() {
        return fail("SSA trait metadata is missing compiler-owned core traits");
    }
    let mut names = HashSet::new();
    for (index, trait_metadata) in program.traits.iter().enumerate() {
        if trait_metadata.id.index() != Some(index)
            || trait_metadata.name.is_empty()
            || !names.insert(trait_metadata.name.as_str())
        {
            return fail("SSA traits must have dense IDs and unique non-empty names");
        }
        if let Some((role, name)) = core.get(index) {
            if trait_metadata.role != *role
                || trait_metadata.name != *name
                || trait_metadata.source.is_some()
            {
                return fail("SSA compiler-owned trait identity is not canonical");
            }
        } else if trait_metadata.role != TraitRole::User
            || trait_metadata
                .source
                .is_none_or(|source| source as usize >= program.sources.len())
        {
            return fail("SSA source trait has invalid role or source identity");
        }
    }
    let mut coherent = HashSet::new();
    for (index, implementation) in program.implementations.iter().enumerate() {
        if implementation.id.index() != Some(index) {
            return fail("SSA implementations must have dense IDs");
        }
        let trait_metadata = trait_by_id(program, implementation.trait_id)?;
        if trait_metadata.role != TraitRole::User {
            return fail("SSA explicit implementation targets a compiler-owned core trait");
        }
        let _product = product_by_id(program, implementation.product)?;
        if implementation.source as usize >= program.sources.len() {
            return fail("SSA implementation has an invalid source identity");
        }
        if !coherent.insert((implementation.trait_id, implementation.product)) {
            return fail("SSA has overlapping marker implementations");
        }
    }
    Ok(())
}
