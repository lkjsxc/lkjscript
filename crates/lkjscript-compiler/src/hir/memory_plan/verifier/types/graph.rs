use super::*;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) enum VerifiedDeclarationKey {
    Product(String),
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
        let mut keys = Vec::new();
        keys.extend(
            program
                .products
                .iter()
                .map(|item| VerifiedDeclarationKey::Product(item.name.clone())),
        );
        keys.extend(
            program
                .enums
                .iter()
                .map(|item| VerifiedDeclarationKey::Enum(item.id.bytes())),
        );
        if u64::try_from(keys.len()).unwrap_or(u64::MAX) > MAX_MEMORY_PLAN_TYPE_NODES {
            return Err(Error::msg(
                "memory verifier type graph exceeds bounded nodes",
            ));
        }
        let index: HashMap<_, _> = keys
            .iter()
            .cloned()
            .enumerate()
            .map(|(i, key)| (key, i))
            .collect();
        let mut adjacency = vec![Vec::new(); keys.len()];
        for (node, key) in keys.iter().enumerate() {
            for ty in verified_declaration_fields(program, key)? {
                let mut targets = Vec::new();
                verified_collect_declarations(ty, &mut targets);
                for target in targets {
                    if let Some(target) = index.get(&target) {
                        adjacency[node].push(*target);
                    }
                }
            }
        }
        let edges = adjacency.iter().try_fold(0_u64, |sum, item| {
            sum.checked_add(
                u64::try_from(item.len())
                    .map_err(|_| Error::msg("verifier type edges exceed u64"))?,
            )
            .ok_or_else(|| Error::msg("verifier type edge overflow"))
        })?;
        if edges > MAX_MEMORY_PLAN_TYPE_EDGES {
            return Err(Error::msg("memory verifier type edges exceed maximum"));
        }
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

pub(crate) fn verified_declaration_fields<'a>(
    program: &'a hir::Program,
    key: &VerifiedDeclarationKey,
) -> Result<Vec<&'a Type>> {
    match key {
        VerifiedDeclarationKey::Product(name) => program
            .products
            .iter()
            .find(|item| &item.name == name)
            .map(|item| item.fields.iter().map(|field| &field.ty).collect())
            .ok_or_else(|| Error::msg("memory verifier lost product declaration")),
        VerifiedDeclarationKey::Enum(id) => program
            .enums
            .iter()
            .find(|item| item.id.bytes() == *id)
            .map(|item| {
                item.variants
                    .iter()
                    .flat_map(|variant| variant.fields.iter().map(|field| &field.ty))
                    .collect()
            })
            .ok_or_else(|| Error::msg("memory verifier lost enum declaration")),
    }
}

pub(super) fn verified_collect_declarations(ty: &Type, output: &mut Vec<VerifiedDeclarationKey>) {
    match ty {
        Type::Product(name) => output.push(VerifiedDeclarationKey::Product(name.clone())),
        Type::Enum { id, arguments, .. } => {
            output.push(VerifiedDeclarationKey::Enum(id.bytes()));
            for argument in arguments {
                verified_collect_declarations(argument, output);
            }
        }
        Type::List(inner) => verified_collect_declarations(inner, output),
        Type::Fn { params, ret } => {
            for parameter in params {
                verified_collect_declarations(parameter, output);
            }
            verified_collect_declarations(ret, output);
        }
        Type::Forall { body, .. } => verified_collect_declarations(body, output),
        _ => {}
    }
}
