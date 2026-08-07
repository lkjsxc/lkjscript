use std::collections::HashMap;

impl OwnedValue {
    #[doc(hidden)]
    pub fn from_segmented_list_snapshot(
        root: Value,
        mut resolve: impl FnMut(u64) -> Result<Vec<Value>>,
    ) -> Result<Self> {
        let mut lists = Vec::new();
        let mut cache = HashMap::new();
        let root = materialize_segmented_value(root, &mut lists, &mut cache, &mut resolve)?;
        let mut visited = Vec::new();
        let mut pending = Vec::new();
        pending
            .try_reserve(1)
            .map_err(|_| Error::msg("owned-list snapshot allocation failed"))?;
        pending.push(root);
        while let Some(value) = pending.pop() {
            let Some(index) = value.as_owned_list().map(|value| value as usize) else {
                continue;
            };
            if visited.len() < lists.len() {
                visited
                    .try_reserve(lists.len() - visited.len())
                    .map_err(|_| Error::msg("owned-list snapshot allocation failed"))?;
                visited.resize(lists.len(), false);
            }
            if visited.get(index).copied().unwrap_or(false) {
                continue;
            }
            let mut node = lists
                .get(index)
                .cloned()
                .ok_or_else(|| Error::msg("segmented snapshot lost owned-list node"))?;
            node.head = materialize_segmented_value(
                node.head,
                &mut lists,
                &mut cache,
                &mut resolve,
            )?;
            node.tail = materialize_segmented_value(
                node.tail,
                &mut lists,
                &mut cache,
                &mut resolve,
            )?;
            if visited.len() < lists.len() {
                visited
                    .try_reserve(lists.len() - visited.len())
                    .map_err(|_| Error::msg("owned-list snapshot allocation failed"))?;
                visited.resize(lists.len(), false);
            }
            visited[index] = true;
            pending
                .try_reserve(2)
                .map_err(|_| Error::msg("owned-list snapshot allocation failed"))?;
            pending.push(node.tail);
            pending.push(node.head);
            lists[index] = node;
        }
        Self::from_materialized_snapshot(root, lists)
    }
}

fn materialize_segmented_value(
    value: Value,
    lists: &mut Vec<OwnedListNode>,
    cache: &mut HashMap<u64, Value>,
    resolve: &mut impl FnMut(u64) -> Result<Vec<Value>>,
) -> Result<Value> {
    let Some(word) = value.as_segmented_list() else {
        return Ok(value);
    };
    if let Some(value) = cache.get(&word).copied() {
        return Ok(value);
    }
    let elements = resolve(word)?;
    lists
        .len()
        .checked_add(elements.len())
        .ok_or_else(|| Error::msg("segmented snapshot work overflow"))?;
    lists
        .try_reserve(elements.len())
        .map_err(|_| Error::msg("owned-list snapshot allocation failed"))?;
    let mut list = Value::EMPTY_LIST;
    for element in elements.into_iter().rev() {
        let index = u32::try_from(lists.len())
            .map_err(|_| Error::msg("owned-list snapshot exceeds u32 nodes"))?;
        lists.push(OwnedListNode {
            head: element,
            tail: list,
        });
        list = Value::from_owned_list(index);
    }
    cache
        .try_reserve(1)
        .map_err(|_| Error::msg("segmented snapshot cache allocation failed"))?;
    cache.insert(word, list);
    Ok(list)
}
