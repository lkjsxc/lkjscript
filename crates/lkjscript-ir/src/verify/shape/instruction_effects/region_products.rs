use std::collections::{HashMap, HashSet, VecDeque};

use crate::verify::*;
use crate::Program;

pub(super) fn verify(program: &Program) -> crate::Result<()> {
    let mut seen = HashSet::new();
    for region in &program.region_products {
        if !seen.insert(region.product) || !region.identity.is_resolved() {
            return fail("SSA region product identity is duplicate or unresolved");
        }
        let product = program
            .products
            .get(region.product.index().unwrap_or(usize::MAX))
            .filter(|product| product.id == region.product)
            .ok_or_else(|| crate::IrError::new("SSA region product is missing"))?;
        if program
            .memory
            .type_for(&crate::SsaType::Product(region.product))
            .is_some()
        {
            return fail("SSA product overlaps structural and region metadata");
        }
        let expected =
            crate::runtime_product_contract_identity(program.memory.plan, &product.name)?;
        if region.identity != expected {
            return fail("SSA region product identity is noncanonical");
        }
        if product
            .fields
            .iter()
            .any(|field| !region_field(program, &field.ty))
        {
            return fail("SSA region product has an unsupported field route");
        }
    }
    verify_acyclic(program)
}

fn region_field(program: &Program, ty: &crate::SsaType) -> bool {
    match ty {
        crate::SsaType::Unit | crate::SsaType::Bool | crate::SsaType::I64 | crate::SsaType::F64 => {
            true
        }
        crate::SsaType::List(inner) => matches!(
            inner.as_ref(),
            crate::SsaType::Unit | crate::SsaType::Bool | crate::SsaType::I64 | crate::SsaType::F64
        ),
        crate::SsaType::Product(product) => program
            .region_products
            .iter()
            .any(|metadata| metadata.product == *product),
        _ => false,
    }
}

fn verify_acyclic(program: &Program) -> crate::Result<()> {
    let indexes: HashMap<_, _> = program
        .region_products
        .iter()
        .enumerate()
        .map(|(index, metadata)| (metadata.product, index))
        .collect();
    let mut edges = vec![Vec::new(); indexes.len()];
    let mut indegree = vec![0_usize; indexes.len()];
    for metadata in &program.region_products {
        let source = indexes[&metadata.product];
        for field in &program.products[metadata.product.index().unwrap_or(usize::MAX)].fields {
            let crate::SsaType::Product(target) = field.ty else {
                continue;
            };
            let target = indexes[&target];
            edges[source].push(target);
            indegree[target] = indegree[target]
                .checked_add(1)
                .ok_or_else(|| crate::IrError::new("SSA region product indegree overflow"))?;
        }
    }
    let mut ready: VecDeque<_> = indegree
        .iter()
        .enumerate()
        .filter_map(|(index, degree)| (*degree == 0).then_some(index))
        .collect();
    let mut visited = 0_usize;
    while let Some(source) = ready.pop_front() {
        visited = visited
            .checked_add(1)
            .ok_or_else(|| crate::IrError::new("SSA region product visit count overflow"))?;
        for target in &edges[source] {
            indegree[*target] = indegree[*target]
                .checked_sub(1)
                .ok_or_else(|| crate::IrError::new("SSA region product indegree underflow"))?;
            if indegree[*target] == 0 {
                ready.push_back(*target);
            }
        }
    }
    if visited == indexes.len() {
        Ok(())
    } else {
        fail("SSA region product dependency graph is cyclic")
    }
}
