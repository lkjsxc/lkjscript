#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum DeclarationKey {
    Product(hir::ProductId),
    Enum([u8; 32]),
}

struct DeclarationGraph {
    keys: Vec<DeclarationKey>,
    index: HashMap<DeclarationKey, usize>,
    components: Vec<usize>,
    recursive: Vec<bool>,
    edges: u64,
    scc_work: u64,
}

impl DeclarationGraph {
    fn new(program: &hir::Program) -> Result<Self> {
        let declaration_count = program
            .products
            .len()
            .checked_add(program.enums.len())
            .ok_or_else(|| Error::msg("memory type declaration count overflow"))?;
        let mut keys = Vec::new();
        keys.try_reserve(declaration_count)
            .map_err(|_| Error::msg("memory type key allocation failed"))?;
        keys.extend(
            program
                .products
                .iter()
                .map(|item| DeclarationKey::Product(item.id)),
        );
        keys.extend(
            program
                .enums
                .iter()
                .map(|item| DeclarationKey::Enum(item.id.bytes())),
        );
        let index: HashMap<DeclarationKey, usize> = keys
            .iter()
            .cloned()
            .enumerate()
            .map(|(index, key)| (key, index))
            .collect();
        let mut adjacency = Vec::new();
        adjacency
            .try_reserve(keys.len())
            .map_err(|_| Error::msg("memory type adjacency allocation failed"))?;
        adjacency.resize_with(keys.len(), Vec::new);

        for (node, product) in program.products.iter().enumerate() {
            add_declaration_edges(
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
                .ok_or_else(|| Error::msg("memory enum graph index overflow"))?;
            add_declaration_edges(
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
                    .map_err(|_| Error::msg("type edge count exceeds u64"))?,
            )
            .ok_or_else(|| Error::msg("type edge count overflow"))
        })?;
        let (components, recursive, scc_work) = components(&adjacency)?;
        Ok(Self {
            keys,
            index,
            components,
            recursive,
            edges,
            scc_work,
        })
    }

    fn component(&self, key: &DeclarationKey) -> Option<usize> {
        self.index
            .get(key)
            .and_then(|index| self.components.get(*index))
            .copied()
    }

    fn is_recursive(&self, key: &DeclarationKey) -> bool {
        self.component(key)
            .and_then(|id| self.recursive.get(id))
            .copied()
            .unwrap_or(false)
    }
}

fn add_declaration_edges<'a>(
    node: usize,
    types: impl IntoIterator<Item = &'a Type>,
    index: &HashMap<DeclarationKey, usize>,
    adjacency: &mut [Vec<usize>],
) -> Result<()> {
    let mut referenced = Vec::new();
    for ty in types {
        collect_declarations(ty, &mut referenced)?;
    }
    let targets = adjacency
        .get_mut(node)
        .ok_or_else(|| Error::msg("memory type graph node is missing"))?;
    targets
        .try_reserve(referenced.len())
        .map_err(|_| Error::msg("memory type edge allocation failed"))?;
    targets.extend(
        referenced
            .into_iter()
            .filter_map(|target| index.get(&target).copied()),
    );
    Ok(())
}

fn collect_declarations(ty: &Type, output: &mut Vec<DeclarationKey>) -> Result<()> {
    let mut pending = vec![ty];
    while let Some(ty) = pending.pop() {
        match ty {
            Type::Product(id) => {
                output
                    .try_reserve(1)
                    .map_err(|_| Error::msg("memory declaration output allocation failed"))?;
                output.push(DeclarationKey::Product(*id));
            }
            Type::Enum { id, arguments, .. } => {
                output
                    .try_reserve(1)
                    .map_err(|_| Error::msg("memory declaration output allocation failed"))?;
                output.push(DeclarationKey::Enum(id.bytes()));
                pending
                    .try_reserve(arguments.len())
                    .map_err(|_| Error::msg("memory declaration work allocation failed"))?;
                pending.extend(arguments.iter().rev());
            }
            Type::List(inner) | Type::Forall { body: inner, .. } => {
                pending
                    .try_reserve(1)
                    .map_err(|_| Error::msg("memory declaration work allocation failed"))?;
                pending.push(inner);
            }
            Type::Fn { params, ret } => {
                let additional = params
                    .len()
                    .checked_add(1)
                    .ok_or_else(|| Error::msg("memory declaration work size overflow"))?;
                pending
                    .try_reserve(additional)
                    .map_err(|_| Error::msg("memory declaration work allocation failed"))?;
                pending.push(ret);
                pending.extend(params.iter().rev());
            }
            _ => {}
        }
    }
    Ok(())
}

fn components(adjacency: &[Vec<usize>]) -> Result<(Vec<usize>, Vec<bool>, u64)> {
    let mut work = 0_u64;
    let mut order = Vec::new();
    order
        .try_reserve(adjacency.len())
        .map_err(|_| Error::msg("memory SCC order allocation failed"))?;
    let mut seen = vec![false; adjacency.len()];
    for root in 0..adjacency.len() {
        if seen[root] {
            continue;
        }
        seen[root] = true;
        let mut stack = vec![(root, 0_usize)];
        while let Some((node, edge)) = stack.pop() {
            charge_scc(&mut work)?;
            if let Some(next) = adjacency[node].get(edge).copied() {
                let next_edge = edge
                    .checked_add(1)
                    .ok_or_else(|| Error::msg("memory SCC edge index overflow"))?;
                stack.push((node, next_edge));
                if !seen[next] {
                    seen[next] = true;
                    stack.push((next, 0));
                }
            } else {
                order.push(node);
            }
        }
    }

    let mut reverse = Vec::new();
    reverse
        .try_reserve(adjacency.len())
        .map_err(|_| Error::msg("memory reverse type graph allocation failed"))?;
    reverse.resize_with(adjacency.len(), Vec::new);
    for (from, targets) in adjacency.iter().enumerate() {
        for target in targets {
            reverse[*target].push(from);
        }
    }

    let mut component = vec![usize::MAX; adjacency.len()];
    let mut sizes = Vec::new();
    sizes
        .try_reserve(adjacency.len())
        .map_err(|_| Error::msg("memory SCC size allocation failed"))?;
    while let Some(root) = order.pop() {
        if component[root] != usize::MAX {
            continue;
        }
        let id = sizes.len();
        let mut size = 0_usize;
        let mut stack = vec![root];
        component[root] = id;
        while let Some(node) = stack.pop() {
            charge_scc(&mut work)?;
            size = size
                .checked_add(1)
                .ok_or_else(|| Error::msg("memory SCC size overflow"))?;
            for next in &reverse[node] {
                if component[*next] == usize::MAX {
                    component[*next] = id;
                    stack.push(*next);
                }
            }
        }
        sizes.push(size);
    }
    let mut recursive: Vec<bool> = sizes.iter().map(|size| *size > 1).collect();
    for (node, targets) in adjacency.iter().enumerate() {
        if targets.contains(&node) {
            recursive[component[node]] = true;
        }
    }
    Ok((component, recursive, work))
}

fn charge_scc(work: &mut u64) -> Result<()> {
    *work = work
        .checked_add(1)
        .ok_or_else(|| Error::msg("HIR memory-plan SCC work overflow"))?;
    Ok(())
}
