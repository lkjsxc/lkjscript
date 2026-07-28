#[derive(Default)]
pub(super) struct Samples {
    elapsed_ns: Vec<u128>,
    steals: u64,
    same_group_steals: u64,
    cross_group_steals: u64,
    cross_numa_steals: u64,
    parks: u64,
    queue_wait_ns: u64,
    max_queue_wait_ns: u64,
    task_time_ns: u64,
    wakeups: u64,
    transfers: u64,
    remote_releases: u64,
    allocated_bytes: u64,
    peak_live_bytes: u64,
    live_objects: u64,
    checksum: u64,
}

impl Samples {
    pub(super) fn push(
        &mut self,
        elapsed_ns: u128,
        steals: [u64; 4],
        parks: u64,
        telemetry: [u64; 4],
        homes: [u64; 5],
        checksum: u64,
    ) {
        self.elapsed_ns.push(elapsed_ns);
        self.steals = self.steals.saturating_add(steals[0]);
        self.same_group_steals = self.same_group_steals.saturating_add(steals[1]);
        self.cross_group_steals = self.cross_group_steals.saturating_add(steals[2]);
        self.cross_numa_steals = self.cross_numa_steals.saturating_add(steals[3]);
        self.parks = self.parks.saturating_add(parks);
        self.queue_wait_ns = self.queue_wait_ns.saturating_add(telemetry[0]);
        self.max_queue_wait_ns = self.max_queue_wait_ns.max(telemetry[1]);
        self.task_time_ns = self.task_time_ns.saturating_add(telemetry[2]);
        self.wakeups = self.wakeups.saturating_add(telemetry[3]);
        self.transfers = self.transfers.saturating_add(homes[0]);
        self.remote_releases = self.remote_releases.saturating_add(homes[1]);
        self.allocated_bytes = self.allocated_bytes.saturating_add(homes[2]);
        self.peak_live_bytes = self.peak_live_bytes.max(homes[3]);
        self.live_objects = self.live_objects.saturating_add(homes[4]);
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
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t",
                "{}\t{}\t{}\t{}\t{}"
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
            self.queue_wait_ns,
            self.max_queue_wait_ns,
            self.task_time_ns,
            self.wakeups,
            self.transfers,
            self.remote_releases,
            self.allocated_bytes,
            self.peak_live_bytes,
            self.live_objects,
            self.checksum,
        );
    }
}
