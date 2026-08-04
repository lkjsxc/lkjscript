pub struct Selection {
    pub nodes: BTreeSet<String>,
    pub edges: BTreeSet<usize>,
    pub work: u64,
    pub bytes: u64,
    pub stop_reasons: BTreeSet<String>,
    pub omitted_frontier: BTreeSet<String>,
    work_limit: u64,
    byte_limit: u64,
}

impl Selection {
    fn new(work_limit: u64, byte_limit: u64) -> Self {
        Self {
            nodes: BTreeSet::new(),
            edges: BTreeSet::new(),
            work: 0,
            bytes: 0,
            stop_reasons: BTreeSet::new(),
            omitted_frontier: BTreeSet::new(),
            work_limit,
            byte_limit,
        }
    }

    fn charge(&mut self, work: u64, bytes: u64) -> bool {
        let Some(next_work) = self.work.checked_add(work) else {
            self.stop_reasons.insert("arithmetic-overflow".into());
            return false;
        };
        let Some(next_bytes) = self.bytes.checked_add(bytes) else {
            self.stop_reasons.insert("arithmetic-overflow".into());
            return false;
        };
        if next_work > self.work_limit {
            self.stop_reasons.insert("work-budget".into());
            return false;
        }
        if next_bytes > self.byte_limit {
            self.stop_reasons.insert("retained-byte-budget".into());
            return false;
        }
        self.work = next_work;
        self.bytes = next_bytes;
        true
    }

    fn charge_edge(&mut self, bytes: u64) -> bool {
        let Some(next_bytes) = self.bytes.checked_add(bytes) else {
            self.stop_reasons.insert("arithmetic-overflow".into());
            return false;
        };
        if next_bytes > self.byte_limit / 2 {
            self.stop_reasons.insert("retained-byte-budget".into());
            return false;
        }
        self.charge(0, bytes)
    }

    pub fn include_required(&mut self, node: &Node) {
        if self.nodes.contains(&node.id) {
            return;
        }
        let bytes = (node.id.len() + node.label.len() + node.authority.len()) as u64;
        if self.charge(1, bytes) {
            self.nodes.insert(node.id.clone());
        } else {
            self.omitted_frontier.insert(node.id.clone());
        }
    }
}
