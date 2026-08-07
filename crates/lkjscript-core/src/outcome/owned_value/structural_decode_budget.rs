struct StructuralDecodeBudget {
    metrics: StructuralSnapshotMetrics,
}

impl StructuralDecodeBudget {
    const fn new() -> Self {
        Self {
            metrics: StructuralSnapshotMetrics {
                nodes: 0,
                fields: 0,
                aggregate_bytes: 0,
                string_bytes: 0,
                path_bytes: 0,
                encode_work: 0,
                decode_work: 0,
            },
        }
    }

    fn node(&mut self) -> Result<()> {
        self.metrics.nodes = self
            .metrics
            .nodes
            .checked_add(1)
            .ok_or_else(|| Error::msg("structural snapshot node count exceeds u64"))?;
        self.work(1)
    }

    fn fields(&mut self, count: u64) -> Result<()> {
        self.metrics.fields = self
            .metrics
            .fields
            .checked_add(count)
            .ok_or_else(|| Error::msg("structural snapshot field count overflow"))?;
        self.work(count)
    }

    fn bytes(&mut self, length: usize, class: DecodeByteClass) -> Result<()> {
        let length = u64::try_from(length)
            .map_err(|_| Error::msg("structural snapshot byte count overflow"))?;
        self.metrics.aggregate_bytes = self
            .metrics
            .aggregate_bytes
            .checked_add(length)
            .ok_or_else(|| Error::msg("structural snapshot byte count overflow"))?;
        if matches!(class, DecodeByteClass::String) {
            self.metrics.string_bytes = self
                .metrics
                .string_bytes
                .checked_add(length)
                .ok_or_else(|| Error::msg("structural string byte count overflow"))?;
        }
        if matches!(class, DecodeByteClass::Path) {
            self.metrics.path_bytes = self
                .metrics
                .path_bytes
                .checked_add(length)
                .ok_or_else(|| Error::msg("structural path byte count overflow"))?;
        }
        self.work(length)
    }

    fn finish(mut self) -> StructuralSnapshotMetrics {
        self.metrics.decode_work = self.metrics.encode_work;
        self.metrics
    }

    fn work(&mut self, amount: u64) -> Result<()> {
        self.metrics.encode_work = self
            .metrics
            .encode_work
            .checked_add(amount)
            .ok_or_else(|| Error::msg("structural snapshot decode work overflow"))?;
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum DecodeByteClass {
    String,
    Path,
    Other,
}
