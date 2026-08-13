use super::*;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) enum VerifiedDeclarationKey {
    Product(hir::ProductId),
    Enum([u8; 32]),
}

pub(crate) struct VerifiedDeclarationGraph {
    pub(crate) keys: Vec<VerifiedDeclarationKey>,
    index: HashMap<VerifiedDeclarationKey, usize>,
    components: Vec<usize>,
    recursive: Vec<bool>,
    pub(crate) edges: u64,
    pub(crate) scc_work: u64,
}

impl VerifiedDeclarationGraph {
    pub(crate) fn new(program: &hir::Program) -> Result<Self> {
        let declaration_count = program
            .products
            .len()
            .checked_add(program.enums.len())
            .ok_or_else(|| Error::msg("verifier type declaration count overflow"))?;
        let mut keys = Vec::new();
        keys.try_reserve(declaration_count)
            .map_err(|_| Error::msg("verifier type key allocation failed"))?;
        keys.extend(
            program
                .products
                .iter()
                .map(|item| VerifiedDeclarationKey::Product(item.id)),
        );
        keys.extend(
            program
                .enums
                .iter()
                .map(|item| VerifiedDeclarationKey::Enum(item.id.bytes())),
        );
        let index: HashMap<_, _> = keys
            .iter()
            .cloned()
            .enumerate()
            .map(|(index, key)| (key, index))
            .collect();
        let mut adjacency = Vec::new();
        adjacency
            .try_reserve(keys.len())
            .map_err(|_| Error::msg("verifier type adjacency allocation failed"))?;
        adjacency.resize_with(keys.len(), Vec::new);

        for (node, product) in program.products.iter().enumerate() {
            verified_add_declaration_edges(
                node,
                product.fields.iter().map(|field| &field.ty),
                &index,
                &mut adjacency,
            )?;
        }
        let enum_offset = program.products.len();
        for (offset, definition) in program.enums.iter().enumerate() {
            let node = enum_offset
                .checked_add(offset)
                .ok_or_else(|| Error::msg("verifier enum graph index overflow"))?;
            verified_add_declaration_edges(
                node,
                definition
                    .variants
                    .iter()
                    .flat_map(|variant| variant.fields.iter().map(|field| &field.ty)),
                &index,
                &mut adjacency,
            )?;
        }

        let edges = adjacency.iter().try_fold(0_u64, |sum, item| {
            sum.checked_add(
                u64::try_from(item.len())
                    .map_err(|_| Error::msg("verifier type edges exceed u64"))?,
            )
            .ok_or_else(|| Error::msg("verifier type edge overflow"))
        })?;
        let (components, recursive, scc_work) = verified_components(&adjacency)?;
        Ok(Self {
            keys,
            index,
            components,
            recursive,
            edges,
            scc_work,
        })
    }

    pub(crate) fn component(&self, key: &VerifiedDeclarationKey) -> Option<usize> {
        self.index
            .get(key)
            .and_then(|index| self.components.get(*index))
            .copied()
    }

    pub(crate) fn is_recursive(&self, key: &VerifiedDeclarationKey) -> bool {
        self.component(key)
            .and_then(|id| self.recursive.get(id))
            .copied()
            .unwrap_or(false)
    }
}

fn verified_add_declaration_edges<'a>(
    node: usize,
    types: impl IntoIterator<Item = &'a Type>,
    index: &HashMap<VerifiedDeclarationKey, usize>,
    adjacency: &mut [Vec<usize>],
) -> Result<()> {
    let mut referenced = Vec::new();
    for ty in types {
        verified_collect_declarations(ty, &mut referenced)?;
    }
    let targets = adjacency
        .get_mut(node)
        .ok_or_else(|| Error::msg("verifier type graph node is missing"))?;
    targets
        .try_reserve(referenced.len())
        .map_err(|_| Error::msg("verifier type edge allocation failed"))?;
    targets.extend(
        referenced
            .into_iter()
            .filter_map(|target| index.get(&target).copied()),
    );
    Ok(())
}

pub(super) fn verified_collect_declarations(
    ty: &Type,
    output: &mut Vec<VerifiedDeclarationKey>,
) -> Result<()> {
    let mut pending = vec![ty];
    while let Some(ty) = pending.pop() {
        match ty {
            Type::Product(id) => {
                output.try_reserve(1).map_err(|_| {
                    Error::msg("memory verifier declaration output allocation failed")
                })?;
                output.push(VerifiedDeclarationKey::Product(*id));
            }
            Type::Enum { id, arguments, .. } => {
                output.try_reserve(1).map_err(|_| {
                    Error::msg("memory verifier declaration output allocation failed")
                })?;
                output.push(VerifiedDeclarationKey::Enum(id.bytes()));
                pending.try_reserve(arguments.len()).map_err(|_| {
                    Error::msg("memory verifier declaration work allocation failed")
                })?;
                pending.extend(arguments.iter().rev());
            }
            Type::List(inner) | Type::Forall { body: inner, .. } => {
                pending.try_reserve(1).map_err(|_| {
                    Error::msg("memory verifier declaration work allocation failed")
                })?;
                pending.push(inner);
            }
            Type::Fn { params, ret } => {
                let additional = params
                    .len()
                    .checked_add(1)
                    .ok_or_else(|| Error::msg("memory verifier declaration work size overflow"))?;
                pending.try_reserve(additional).map_err(|_| {
                    Error::msg("memory verifier declaration work allocation failed")
                })?;
                pending.push(ret);
                pending.extend(params.iter().rev());
            }
            _ => {}
        }
    }
    Ok(())
}
