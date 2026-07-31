#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum DeclarationKey {
    Product(String),
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
        let mut keys = Vec::with_capacity(program.products.len() + program.enums.len());
        keys.extend(program.products.iter().map(|item| DeclarationKey::Product(item.name.clone())));
        keys.extend(program.enums.iter().map(|item| DeclarationKey::Enum(item.id.bytes())));
        if u64::try_from(keys.len()).unwrap_or(u64::MAX) > MAX_MEMORY_PLAN_TYPE_NODES {
            return Err(Error::msg("HIR memory-plan type graph exceeds bounded nodes"));
        }
        let index: HashMap<DeclarationKey, usize> = keys.iter().cloned().enumerate()
            .map(|(i, key)| (key, i)).collect();
        let mut adjacency = vec![Vec::new(); keys.len()];
        for (node, key) in keys.iter().enumerate() {
            let fields = declaration_fields(program, key)?;
            let mut referenced = Vec::new();
            for ty in fields { collect_declarations(ty, &mut referenced); }
            for target in referenced {
                if let Some(target) = index.get(&target) { adjacency[node].push(*target); }
            }
        }
        let edges = adjacency.iter().try_fold(0_u64, |sum, item| {
            sum.checked_add(u64::try_from(item.len()).map_err(|_| Error::msg("type edge count exceeds u64"))?)
                .ok_or_else(|| Error::msg("type edge count overflow"))
        })?;
        if edges > MAX_MEMORY_PLAN_TYPE_EDGES {
            return Err(Error::msg("HIR memory-plan type graph exceeds bounded edges"));
        }
        let (components, recursive, scc_work) = components(&adjacency)?;
        Ok(Self { keys, index, components, recursive, edges, scc_work })
    }

    fn component(&self, key: &DeclarationKey) -> Option<usize> {
        self.index.get(key).and_then(|index| self.components.get(*index)).copied()
    }

    fn is_recursive(&self, key: &DeclarationKey) -> bool {
        self.component(key).and_then(|id| self.recursive.get(id)).copied().unwrap_or(false)
    }
}

fn declaration_fields<'a>(program: &'a hir::Program, key: &DeclarationKey) -> Result<Vec<&'a Type>> {
    match key {
        DeclarationKey::Product(name) => program.products.iter().find(|item| &item.name == name)
            .map(|item| item.fields.iter().map(|field| &field.ty).collect())
            .ok_or_else(|| Error::msg(format!("memory type graph lost product {name}"))),
        DeclarationKey::Enum(id) => program.enums.iter().find(|item| item.id.bytes() == *id)
            .map(|item| item.variants.iter().flat_map(|variant| variant.fields.iter().map(|field| &field.ty)).collect())
            .ok_or_else(|| Error::msg("memory type graph lost enum")),
    }
}

fn collect_declarations(ty: &Type, output: &mut Vec<DeclarationKey>) {
    match ty {
        Type::Product(name) => output.push(DeclarationKey::Product(name.clone())),
        Type::Enum { id, arguments, .. } => {
            output.push(DeclarationKey::Enum(id.bytes()));
            for argument in arguments { collect_declarations(argument, output); }
        }
        Type::List(inner) => collect_declarations(inner, output),
        Type::Fn { params, ret } => {
            for parameter in params { collect_declarations(parameter, output); }
            collect_declarations(ret, output);
        }
        Type::Forall { body, .. } => collect_declarations(body, output),
        _ => {}
    }
}

fn components(adjacency: &[Vec<usize>]) -> Result<(Vec<usize>, Vec<bool>, u64)> {
    let mut work = 0_u64;
    let mut order = Vec::with_capacity(adjacency.len());
    let mut seen = vec![false; adjacency.len()];
    for root in 0..adjacency.len() {
        if seen[root] { continue; }
        seen[root] = true;
        let mut stack = vec![(root, 0_usize)];
        while let Some((node, edge)) = stack.pop() {
            charge_scc(&mut work)?;
            if let Some(next) = adjacency[node].get(edge).copied() {
                stack.push((node, edge + 1));
                if !seen[next] { seen[next] = true; stack.push((next, 0)); }
            } else { order.push(node); }
        }
    }
    let mut reverse = vec![Vec::new(); adjacency.len()];
    for (from, targets) in adjacency.iter().enumerate() { for target in targets { reverse[*target].push(from); } }
    let mut component = vec![usize::MAX; adjacency.len()];
    let mut sizes = Vec::new();
    while let Some(root) = order.pop() {
        if component[root] != usize::MAX { continue; }
        let id = sizes.len();
        let mut size = 0_usize;
        let mut stack = vec![root]; component[root] = id;
        while let Some(node) = stack.pop() {
            charge_scc(&mut work)?; size += 1;
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
        if targets.contains(&node) { recursive[component[node]] = true; }
    }
    Ok((component, recursive, work))
}

fn charge_scc(work: &mut u64) -> Result<()> {
    *work = work.checked_add(1).ok_or_else(|| Error::msg("HIR memory-plan SCC work overflow"))?;
    if *work > MAX_MEMORY_PLAN_SCC_WORK { return Err(Error::msg("HIR memory-plan SCC work exceeds bounded maximum")); }
    Ok(())
}
