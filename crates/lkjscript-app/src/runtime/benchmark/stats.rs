#[derive(Default)]
pub(super) struct Samples {
    elapsed_ns: Vec<u128>,
    steals: u64,
    same_group_steals: u64,
    cross_group_steals: u64,
    cross_numa_steals: u64,
    parks: u64,
    checksum: u64,
}

impl Samples {
    pub(super) fn push(&mut self, elapsed_ns: u128, steals: [u64; 4], parks: u64, checksum: u64) {
        self.elapsed_ns.push(elapsed_ns);
        self.steals = self.steals.saturating_add(steals[0]);
        self.same_group_steals = self.same_group_steals.saturating_add(steals[1]);
        self.cross_group_steals = self.cross_group_steals.saturating_add(steals[2]);
        self.cross_numa_steals = self.cross_numa_steals.saturating_add(steals[3]);
        self.parks = self.parks.saturating_add(parks);
        self.checksum ^= checksum;
    }

    pub(super) fn print(&mut self, workload: &str, policy: &str, affinity: &str, workers: usize) {
        self.elapsed_ns.sort_unstable();
        let count = self.elapsed_ns.len();
        let percentile = |numerator: usize| {
            let index = ((count - 1) * numerator).div_ceil(100);
            self.elapsed_ns[index]
        };
        let total: u128 = self.elapsed_ns.iter().sum();
        let throughput = (super::workload::TASKS as u128 * count as u128 * 1_000_000_000)
            .checked_div(total)
            .unwrap_or(0);
        println!(
            concat!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t",
                "{}\t{}\t{}\t{}\t{}\t{}"
            ),
            workload,
            policy,
            affinity,
            workers,
            count,
            percentile(50),
            percentile(95),
            percentile(99),
            throughput,
            self.steals,
            self.same_group_steals,
            self.cross_group_steals,
            self.cross_numa_steals,
            self.parks,
            self.checksum,
        );
    }
}
