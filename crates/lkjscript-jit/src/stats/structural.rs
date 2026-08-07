#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NativeStructuralStats {
    pub calls: u64,
    pub roots_published: u64,
    pub roots_moved: u64,
    pub roots_dropped: u64,
    pub roots_released: u64,
    pub loans_started: u64,
    pub loans_ended: u64,
    pub destinations_created: u64,
    pub destinations_completed: u64,
    pub destinations_aborted: u64,
    pub views_created: u64,
    pub views_ended: u64,
    pub event_records: u64,
    pub events_overwritten: u64,
    pub releases: u64,
    pub release_work: u64,
    pub sealed_publications: u64,
    pub zero_copy_adoptions: u64,
    pub copied_publication_bytes: u64,
    pub sealed_acquisitions: u64,
    pub sealed_releases: u64,
    pub sealed_release_work: u64,
    pub sealed_nodes_reclaimed: u64,
    pub live_objects: u64,
    pub live_sealed_domains: u64,
    pub live_sealed_owners: u64,
    pub live_roots: u64,
    pub live_loans: u64,
    pub live_views: u64,
    pub live_destinations: u64,
    pub release_backlog: u64,
    pub empty_completions: u64,
    pub teardown_failures: u64,
}

impl NativeStructuralStats {
    pub(crate) fn add(&mut self, other: Self) {
        self.calls = self.calls.saturating_add(other.calls);
        self.roots_published = self.roots_published.saturating_add(other.roots_published);
        self.roots_moved = self.roots_moved.saturating_add(other.roots_moved);
        self.roots_dropped = self.roots_dropped.saturating_add(other.roots_dropped);
        self.roots_released = self.roots_released.saturating_add(other.roots_released);
        self.loans_started = self.loans_started.saturating_add(other.loans_started);
        self.loans_ended = self.loans_ended.saturating_add(other.loans_ended);
        self.destinations_created = self
            .destinations_created
            .saturating_add(other.destinations_created);
        self.destinations_completed = self
            .destinations_completed
            .saturating_add(other.destinations_completed);
        self.destinations_aborted = self
            .destinations_aborted
            .saturating_add(other.destinations_aborted);
        self.views_created = self.views_created.saturating_add(other.views_created);
        self.views_ended = self.views_ended.saturating_add(other.views_ended);
        self.event_records = self.event_records.saturating_add(other.event_records);
        self.events_overwritten = self
            .events_overwritten
            .saturating_add(other.events_overwritten);
        self.releases = self.releases.saturating_add(other.releases);
        self.release_work = self.release_work.saturating_add(other.release_work);
        self.sealed_publications = self
            .sealed_publications
            .saturating_add(other.sealed_publications);
        self.zero_copy_adoptions = self
            .zero_copy_adoptions
            .saturating_add(other.zero_copy_adoptions);
        self.copied_publication_bytes = self
            .copied_publication_bytes
            .saturating_add(other.copied_publication_bytes);
        self.sealed_acquisitions = self
            .sealed_acquisitions
            .saturating_add(other.sealed_acquisitions);
        self.sealed_releases = self.sealed_releases.saturating_add(other.sealed_releases);
        self.sealed_release_work = self
            .sealed_release_work
            .saturating_add(other.sealed_release_work);
        self.sealed_nodes_reclaimed = self
            .sealed_nodes_reclaimed
            .saturating_add(other.sealed_nodes_reclaimed);
        self.live_objects = other.live_objects;
        self.live_sealed_domains = other.live_sealed_domains;
        self.live_sealed_owners = other.live_sealed_owners;
        self.live_roots = other.live_roots;
        self.live_loans = other.live_loans;
        self.live_views = other.live_views;
        self.live_destinations = other.live_destinations;
        self.release_backlog = other.release_backlog;
        self.empty_completions = self
            .empty_completions
            .saturating_add(other.empty_completions);
        self.teardown_failures = self
            .teardown_failures
            .saturating_add(other.teardown_failures);
    }
}
