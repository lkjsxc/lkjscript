impl OwnedValue {
    #[doc(hidden)]
    pub fn retain_symbols<'a>(
        mut self,
        mut resolve: impl FnMut(u32) -> Result<&'a str>,
    ) -> Result<Self> {
        let structural_symbols = self.structural_symbol_order()?;
        let mut pending = vec![self.root];
        let mut visited = vec![false; self.heap.len()];
        while let Some(value) = pending.pop() {
            if let Some(symbol) = value.as_symbol() {
                self.retain_symbol(symbol, resolve(symbol)?)?;
                continue;
            }
            let Some(index) = value.as_legacy_traced().map(|index| index as usize) else {
                continue;
            };
            if index >= self.heap.len() || visited[index] {
                continue;
            }
            let object = self.heap[index]
                .as_ref()
                .ok_or_else(|| Error::msg("owned value references a missing heap object"))?;
            visited[index] = true;
            object.trace(&mut |child| pending.push(child));
        }
        for symbol in structural_symbols {
            self.retain_symbol(symbol, resolve(symbol)?)?;
        }
        self.canonicalize_symbols()?;
        Ok(self)
    }

    fn structural_symbol_order(&self) -> Result<Vec<u32>> {
        let Some(structural) = self.structural.as_ref() else {
            return Ok(Vec::new());
        };
        let capacity = usize::try_from(structural.metrics.nodes)
            .map_err(|_| Error::msg("owned structural symbol count exceeds platform"))?;
        let mut pending = Vec::new();
        pending
            .try_reserve_exact(capacity)
            .map_err(|_| Error::msg("owned structural symbol traversal allocation failed"))?;
        pending.push(&structural.value);
        let mut symbols = Vec::new();
        symbols
            .try_reserve_exact(capacity)
            .map_err(|_| Error::msg("owned structural symbol traversal allocation failed"))?;
        while let Some(value) = pending.pop() {
            match &value.payload {
                SemanticPayload::Static(crate::StaticStructuralLeaf::Symbol(symbol)) => {
                    symbols.push(*symbol);
                }
                SemanticPayload::Product(fields)
                | SemanticPayload::Enum {
                    active_payload: fields,
                    ..
                } => pending.extend(fields.iter().rev()),
                _ => {}
            }
        }
        Ok(symbols)
    }

    fn canonicalize_symbols(&mut self) -> Result<()> {
        let mut order = Vec::new();
        order
            .try_reserve_exact(self.symbols.iter().flatten().count())
            .map_err(|_| Error::msg("owned symbol order allocation failed"))?;
        order.extend(
            self.symbols
                .iter()
                .enumerate()
                .filter_map(|(index, text)| text.as_ref().map(|_| index)),
        );
        order.sort_unstable_by(|left, right| {
            self.symbols[*left]
                .as_deref()
                .cmp(&self.symbols[*right].as_deref())
        });
        let mut mapping = Vec::new();
        mapping
            .try_reserve_exact(self.symbols.len())
            .map_err(|_| Error::msg("owned symbol mapping allocation failed"))?;
        mapping.resize(self.symbols.len(), None);
        let mut unique: Vec<usize> = Vec::new();
        unique
            .try_reserve_exact(order.len())
            .map_err(|_| Error::msg("owned symbol canonical allocation failed"))?;
        for old in order {
            let is_new = unique.last().is_none_or(|previous| {
                self.symbols[*previous].as_deref() != self.symbols[old].as_deref()
            });
            if is_new {
                unique.push(old);
            }
            mapping[old] = u32::try_from(unique.len() - 1).ok();
        }
        rewrite_symbol(&mut self.root, &mapping)?;
        for object in self.heap.iter_mut().flatten() {
            rewrite_object_symbols(object, &mapping)?;
        }
        if let Some(structural) = self.structural.as_mut() {
            rewrite_structural_symbols(&mut structural.value, &mapping)?;
        }
        let mut canonical = Vec::new();
        canonical
            .try_reserve_exact(unique.len())
            .map_err(|_| Error::msg("owned symbol table allocation failed"))?;
        for old in unique {
            canonical.push(self.symbols[old].take());
        }
        self.symbols = canonical;
        Ok(())
    }

    fn retain_symbol(&mut self, symbol: u32, source: &str) -> Result<()> {
        let index = symbol as usize;
        if index >= self.symbols.len() {
            let added = index
                .checked_add(1)
                .and_then(|needed| needed.checked_sub(self.symbols.len()))
                .ok_or_else(|| Error::msg("owned symbol index overflow"))?;
            self.symbols
                .try_reserve_exact(added)
                .map_err(|_| Error::msg("owned symbol table allocation failed"))?;
            self.symbols.resize_with(index + 1, || None);
        }
        if self.symbols[index].is_none() {
            let mut text = String::new();
            text.try_reserve_exact(source.len())
                .map_err(|_| Error::msg("owned symbol text allocation failed"))?;
            text.push_str(source);
            self.symbols[index] = Some(text);
        }
        Ok(())
    }
}

fn rewrite_object_symbols(object: &mut HeapObj, mapping: &[Option<u32>]) -> Result<()> {
    match object {
        HeapObj::Pair { car, cdr } => {
            rewrite_symbol(car, mapping)?;
            rewrite_symbol(cdr, mapping)?;
        }
        HeapObj::Product { fields, .. } => {
            for field in fields {
                rewrite_symbol(field, mapping)?;
            }
        }
        HeapObj::Enum { active_payload, .. } => {
            for field in active_payload {
                rewrite_symbol(field, mapping)?;
            }
        }
    }
    Ok(())
}

fn rewrite_structural_symbols(value: &mut SemanticValue, mapping: &[Option<u32>]) -> Result<()> {
    match &mut value.payload {
        SemanticPayload::Static(crate::StaticStructuralLeaf::Symbol(symbol)) => {
            *symbol = canonical_symbol(*symbol, mapping)?;
        }
        SemanticPayload::Product(fields)
        | SemanticPayload::Enum {
            active_payload: fields,
            ..
        } => {
            for field in fields {
                rewrite_structural_symbols(field, mapping)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn rewrite_symbol(value: &mut Value, mapping: &[Option<u32>]) -> Result<()> {
    let Some(old) = value.as_symbol() else {
        return Ok(());
    };
    *value = Value::from_symbol(canonical_symbol(old, mapping)?);
    Ok(())
}

fn canonical_symbol(old: u32, mapping: &[Option<u32>]) -> Result<u32> {
    mapping
        .get(old as usize)
        .copied()
        .flatten()
        .ok_or_else(|| Error::msg("owned symbol mapping is incomplete"))
}
