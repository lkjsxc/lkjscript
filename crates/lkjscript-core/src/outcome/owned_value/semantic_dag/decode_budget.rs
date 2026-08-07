fn semantic_dag_kind_tag(kind: SemanticDagKind) -> u8 {
    match kind {
        SemanticDagKind::Unit => 0,
        SemanticDagKind::Bool => 1,
        SemanticDagKind::I64 => 2,
        SemanticDagKind::F64 => 3,
        SemanticDagKind::Static => 4,
        SemanticDagKind::String => 5,
        SemanticDagKind::Path => 6,
        SemanticDagKind::Bytes => 7,
        SemanticDagKind::Product => 8,
        SemanticDagKind::Enum => 9,
        SemanticDagKind::EmptyList => 10,
        SemanticDagKind::List => 11,
    }
}

fn decode_semantic_dag_kind(tag: u8) -> Result<SemanticDagKind> {
    Ok(match tag {
        0 => SemanticDagKind::Unit,
        1 => SemanticDagKind::Bool,
        2 => SemanticDagKind::I64,
        3 => SemanticDagKind::F64,
        4 => SemanticDagKind::Static,
        5 => SemanticDagKind::String,
        6 => SemanticDagKind::Path,
        7 => SemanticDagKind::Bytes,
        8 => SemanticDagKind::Product,
        9 => SemanticDagKind::Enum,
        10 => SemanticDagKind::EmptyList,
        11 => SemanticDagKind::List,
        _ => return Err(Error::msg("unknown semantic DAG node kind")),
    })
}

struct SemanticDagDecodeBudget {
    metrics: StructuralSnapshotMetrics,
}

impl SemanticDagDecodeBudget {
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
            .ok_or_else(|| Error::msg("semantic DAG node count overflow"))?;
        self.work(1)
    }

    fn fields(&mut self, count: u64) -> Result<()> {
        self.metrics.fields = self
            .metrics
            .fields
            .checked_add(count)
            .ok_or_else(|| Error::msg("semantic DAG edge count overflow"))?;
        self.work(count)
    }

    fn bytes(&mut self, length: usize, class: DagWireByteClass) -> Result<()> {
        let length =
            u64::try_from(length).map_err(|_| Error::msg("semantic DAG byte count overflow"))?;
        self.metrics.aggregate_bytes = self
            .metrics
            .aggregate_bytes
            .checked_add(length)
            .ok_or_else(|| Error::msg("semantic DAG byte count overflow"))?;
        if matches!(class, DagWireByteClass::String) {
            self.metrics.string_bytes = self
                .metrics
                .string_bytes
                .checked_add(length)
                .ok_or_else(|| Error::msg("semantic DAG string byte count overflow"))?;
        }
        if matches!(class, DagWireByteClass::Path) {
            self.metrics.path_bytes = self
                .metrics
                .path_bytes
                .checked_add(length)
                .ok_or_else(|| Error::msg("semantic DAG path byte count overflow"))?;
        }
        self.work(length)
    }

    fn work(&mut self, amount: u64) -> Result<()> {
        self.metrics.encode_work = self
            .metrics
            .encode_work
            .checked_add(amount)
            .ok_or_else(|| Error::msg("semantic DAG decode work overflow"))?;
        Ok(())
    }

    fn finish(mut self) -> StructuralSnapshotMetrics {
        self.metrics.decode_work = self.metrics.encode_work;
        self.metrics
    }
}

#[derive(Clone, Copy)]
enum DagWireByteClass {
    String,
    Path,
    Other,
}
