impl OwnedValue {
    #[doc(hidden)]
    pub fn retain_symbols<'a>(
        mut self,
        mut resolve: impl FnMut(u32) -> Result<&'a str>,
    ) -> Result<Self> {
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
        self.canonicalize_symbols()?;
        Ok(self)
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
        HeapObj::Str(_) | HeapObj::Buf(_) | HeapObj::Path(_) => {}
    }
    Ok(())
}

fn rewrite_symbol(value: &mut Value, mapping: &[Option<u32>]) -> Result<()> {
    let Some(old) = value.as_symbol() else {
        return Ok(());
    };
    let canonical = mapping
        .get(old as usize)
        .copied()
        .flatten()
        .ok_or_else(|| Error::msg("owned symbol mapping is incomplete"))?;
    *value = Value::from_symbol(canonical);
    Ok(())
}
