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
        Ok(self)
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
