use crate::*;

pub(super) struct CacheAttempt {
    cache: Option<NativeArtifactCache>,
    key: Option<ArtifactKey>,
    pub(super) status: CacheStatus,
    pub(super) lookup: Duration,
    pub(super) publication: Duration,
}

impl JitSession {
    pub(super) fn cached_lowering(
        &mut self,
        root: FunctionId,
    ) -> (Option<lower::LoweredGroup>, CacheAttempt) {
        let Some(context) = self.config.cache.as_ref() else {
            return (None, CacheAttempt::disabled());
        };
        self.cache_lookups = self.cache_lookups.saturating_add(1);
        let started = Instant::now();
        let attempt = (|| {
            let ssa = self.program.verified_digest().ok()?;
            let policy = optimization_policy(self.program.tier(), self.config.optimization_limits);
            let key = artifact_key(
                context,
                ssa,
                cache_tier(self.program.tier()),
                root.raw(),
                policy,
                self.config.backend_limits,
            )
            .ok()?;
            let cache =
                NativeArtifactCache::open(&context.package_root, self.config.cache_limits).ok()?;
            Some((cache, key))
        })();
        let Some((cache, key)) = attempt else {
            self.cache_misses = self.cache_misses.saturating_add(1);
            let lookup = started.elapsed();
            self.cache_lookup_time = self.cache_lookup_time.saturating_add(lookup);
            return (None, CacheAttempt::unavailable(lookup));
        };
        let lookup = match cache.lookup(&key) {
            Ok(lookup) => lookup,
            Err(_) => {
                self.cache_misses = self.cache_misses.saturating_add(1);
                let elapsed = started.elapsed();
                self.cache_lookup_time = self.cache_lookup_time.saturating_add(elapsed);
                return (None, CacheAttempt::unavailable(elapsed));
            }
        };
        let elapsed = started.elapsed();
        self.cache_lookup_time = self.cache_lookup_time.saturating_add(elapsed);
        match lookup {
            CacheLookup::Hit { image, bytes } => {
                match lower::cached_group(self.program.program(), root, *image) {
                    Ok(group) => {
                        self.cache_hits = self.cache_hits.saturating_add(1);
                        self.cache_bytes_read = self.cache_bytes_read.saturating_add(bytes);
                        (Some(group), CacheAttempt::hit(elapsed))
                    }
                    Err(_) => {
                        self.cache_misses = self.cache_misses.saturating_add(1);
                        self.cache_corruptions = self.cache_corruptions.saturating_add(1);
                        (
                            None,
                            CacheAttempt::miss(cache, key, CacheStatus::MissCorrupt, elapsed),
                        )
                    }
                }
            }
            CacheLookup::Miss(reason) => {
                self.cache_misses = self.cache_misses.saturating_add(1);
                let status = match reason {
                    MissReason::NotFound => CacheStatus::MissNotFound,
                    MissReason::Corrupt => {
                        self.cache_corruptions = self.cache_corruptions.saturating_add(1);
                        CacheStatus::MissCorrupt
                    }
                    MissReason::OverLimit => CacheStatus::MissOverLimit,
                };
                (None, CacheAttempt::miss(cache, key, status, elapsed))
            }
        }
    }

    pub(super) fn publish_cached_image(
        &mut self,
        attempt: &mut CacheAttempt,
        image: &lkjscript_native::InstallableImage,
    ) {
        let (Some(cache), Some(key)) = (attempt.cache.as_ref(), attempt.key.as_ref()) else {
            return;
        };
        let started = Instant::now();
        match cache.publish(key, image) {
            Ok(Publication::Published { bytes }) => {
                self.cache_publications = self.cache_publications.saturating_add(1);
                self.cache_bytes_written = self.cache_bytes_written.saturating_add(bytes);
            }
            Ok(Publication::Duplicate { .. }) => {}
            Ok(Publication::SkippedFull | Publication::SkippedBusy) | Err(_) => {
                self.cache_publication_skips = self.cache_publication_skips.saturating_add(1);
            }
        }
        attempt.publication = started.elapsed();
        self.cache_publication_time = self
            .cache_publication_time
            .saturating_add(attempt.publication);
    }
}

impl CacheAttempt {
    fn disabled() -> Self {
        Self::new(None, None, CacheStatus::Disabled, Duration::ZERO)
    }

    fn unavailable(lookup: Duration) -> Self {
        Self::new(None, None, CacheStatus::Unavailable, lookup)
    }

    fn hit(lookup: Duration) -> Self {
        Self::new(None, None, CacheStatus::Hit, lookup)
    }

    fn miss(
        cache: NativeArtifactCache,
        key: ArtifactKey,
        status: CacheStatus,
        lookup: Duration,
    ) -> Self {
        Self::new(Some(cache), Some(key), status, lookup)
    }

    fn new(
        cache: Option<NativeArtifactCache>,
        key: Option<ArtifactKey>,
        status: CacheStatus,
        lookup: Duration,
    ) -> Self {
        Self {
            cache,
            key,
            status,
            lookup,
            publication: Duration::ZERO,
        }
    }
}

fn cache_tier(tier: Tier) -> CacheTier {
    match tier {
        Tier::Baseline => CacheTier::Baseline,
        Tier::Optimizing => CacheTier::Optimizing,
    }
}

fn optimization_policy(tier: Tier, limits: OptimizationLimits) -> [u8; 32] {
    if tier == Tier::Baseline {
        return [0; 32];
    }
    let mut bytes = Vec::with_capacity(14 * 8);
    for value in [
        limits.max_work_units,
        limits.max_certificate_records,
        limits.max_certificate_bytes_estimate,
        limits.max_instruction_growth,
        limits.max_iterations,
        limits.max_functions,
        limits.max_blocks,
        limits.max_parameters,
        limits.max_instructions,
        limits.max_operands,
        limits.max_frame_facts,
        limits.max_type_nodes,
        limits.max_metadata_items,
        limits.max_string_and_metadata_bytes,
    ] {
        bytes.extend_from_slice(&value.to_be_bytes());
    }
    lkjscript_contracts::sha256(&bytes)
}
