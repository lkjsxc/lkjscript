struct StructuralDecodeBudget {
    limits: StructuralSnapshotLimits,
    metrics: StructuralSnapshotMetrics,
}

impl StructuralDecodeBudget {
    fn new(limits: StructuralSnapshotLimits) -> Result<Self> {
        Ok(Self {
            limits: limits.validate()?,
            metrics: StructuralSnapshotMetrics::default(),
        })
    }

    fn node(&mut self, depth: u16) -> Result<()> {
        if depth > self.limits.max_depth {
            return Err(Error::msg("structural snapshot depth exceeds bound"));
        }
        self.metrics.nodes = self
            .metrics
            .nodes
            .checked_add(1)
            .ok_or_else(|| Error::msg("structural snapshot node count overflow"))?;
        if self.metrics.nodes > self.limits.max_nodes {
            return Err(Error::msg("structural snapshot nodes exceed bound"));
        }
        self.work(1)
    }

    fn fields(&mut self, count: u32) -> Result<()> {
        self.metrics.fields = self
            .metrics
            .fields
            .checked_add(count)
            .ok_or_else(|| Error::msg("structural snapshot field count overflow"))?;
        if self.metrics.fields > self.limits.max_fields {
            return Err(Error::msg("structural snapshot fields exceed bound"));
        }
        self.work(u64::from(count))
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
        if self.metrics.aggregate_bytes > self.limits.max_aggregate_bytes
            || self.metrics.string_bytes > self.limits.max_string_bytes
            || self.metrics.path_bytes > self.limits.max_path_bytes
        {
            return Err(Error::msg("structural snapshot bytes exceed bound"));
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
            .ok_or_else(|| Error::msg("structural snapshot work overflow"))?;
        if self.metrics.encode_work > self.limits.max_decode_work {
            return Err(Error::msg("structural snapshot decode work exceeds bound"));
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum DecodeByteClass {
    String,
    Path,
    Other,
}
