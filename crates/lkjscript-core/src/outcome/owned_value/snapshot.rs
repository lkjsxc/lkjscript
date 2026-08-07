impl OwnedValue {
    #[doc(hidden)]
    pub fn from_value(root: Value) -> Result<Self> {
        Self::from_materialized_snapshot(root, Vec::new())
    }

    pub(crate) fn from_materialized_snapshot(
        root: Value,
        lists: Vec<OwnedListNode>,
    ) -> Result<Self> {
        if root.is_invalid() {
            return Err(Error::msg("cannot own an invalid VM value"));
        }
        validate_list_tails(&lists)?;
        let mut pending = vec![root];
        let mut visited = vec![false; lists.len()];
        while let Some(value) = pending.pop() {
            validate_reachable_value(value, &lists, &mut visited, &mut pending)?;
        }
        if visited.iter().any(|visited| !visited) {
            return Err(Error::msg("owned list snapshot contains unreachable nodes"));
        }
        validate_no_list_cycles(root, &lists)?;
        Ok(Self {
            root,
            lists,
            unique_byte_vector: None,
            unique_bytes: None,
            symbols: Vec::new(),
            structural: None,
            semantic_dag: None,
        })
    }
}

fn validate_list_tails(lists: &[OwnedListNode]) -> Result<()> {
    for (index, node) in lists.iter().enumerate() {
        if !node.tail.is_empty_list()
            && match node.tail.as_owned_list() {
                Some(tail) => usize::try_from(tail).map_or(true, |tail| tail >= index),
                None => true,
            }
        {
            return Err(Error::msg("owned list tail is not an earlier list node"));
        }
    }
    Ok(())
}

fn validate_reachable_value(
    value: Value,
    lists: &[OwnedListNode],
    visited: &mut [bool],
    pending: &mut Vec<Value>,
) -> Result<()> {
    if let Some(index) = value.as_owned_list() {
        let index = usize::try_from(index)
            .map_err(|_| Error::msg("owned value list index exceeds platform"))?;
        let node = lists
            .get(index)
            .ok_or_else(|| Error::msg("owned value list index out of range"))?;
        if !visited[index] {
            visited[index] = true;
            pending.push(node.tail);
            pending.push(node.head);
        }
        return Ok(());
    }
    if value.is_unit()
        || value.as_bool().is_some()
        || value.as_i64().is_some()
        || value.as_f64_bits().is_some()
        || value.is_empty_list()
        || value.as_symbol().is_some()
    {
        return Ok(());
    }
    Err(Error::msg(
        "owned snapshot retained a nontransportable runtime value",
    ))
}

fn validate_no_list_cycles(root: Value, lists: &[OwnedListNode]) -> Result<()> {
    let mut colors = vec![0_u8; lists.len()];
    let mut work = Vec::new();
    if let Some(index) = root.as_owned_list() {
        let index = usize::try_from(index)
            .map_err(|_| Error::msg("owned list root index exceeds platform"))?;
        work.push((index, false));
    }
    while let Some((index, exit)) = work.pop() {
        if exit {
            colors[index] = 2;
            continue;
        }
        match colors[index] {
            1 => return Err(Error::msg("owned list snapshot contains a cycle")),
            2 => continue,
            _ => {}
        }
        colors[index] = 1;
        work.push((index, true));
        let node = lists
            .get(index)
            .ok_or_else(|| Error::msg("owned list cycle index out of range"))?;
        for child in [node.tail, node.head] {
            if let Some(child) = child.as_owned_list() {
                let child = usize::try_from(child)
                    .map_err(|_| Error::msg("owned list child index exceeds platform"))?;
                work.push((child, false));
            }
        }
    }
    Ok(())
}
