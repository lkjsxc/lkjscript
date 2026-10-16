//! Finite decoded point-read pages. Every hit rereads and verifies current store bytes.
use super::*;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, OnceLock};

const SHARDS: usize = 16;
const SHARD_ENTRIES: usize = 256;
const SHARD_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug)]
pub(super) struct ReadPage {
    pub(super) page: Page,
    pub(super) summary: PageLinkSummary,
    bytes: Vec<u8>,
    visited: u64,
    retained: usize,
}

struct Cache {
    entries: BTreeMap<PageDigest, Arc<ReadPage>>,
    order: VecDeque<PageDigest>,
    retained: usize,
    maximum_entries: usize,
    maximum_bytes: usize,
}

impl Cache {
    fn new(maximum_entries: usize, maximum_bytes: usize) -> Self {
        Self {
            entries: BTreeMap::new(),
            order: VecDeque::new(),
            retained: 0,
            maximum_entries,
            maximum_bytes,
        }
    }

    fn retain(&mut self, digest: PageDigest, page: Arc<ReadPage>) {
        if self.maximum_entries == 0
            || page.retained > self.maximum_bytes
            || self.entries.contains_key(&digest)
        {
            return;
        }
        while self.entries.len() >= self.maximum_entries
            || self.retained > self.maximum_bytes - page.retained
        {
            let Some(old) = self.order.pop_front() else {
                return;
            };
            if let Some(evicted) = self.entries.remove(&old) {
                self.retained -= evicted.retained;
            }
        }
        self.retained += page.retained;
        self.entries.insert(digest, page);
        self.order.push_back(digest);
    }
}

pub(super) fn load<S: PageStore + ?Sized>(
    store: &S,
    digest: PageDigest,
    work: &mut MapWork,
) -> Result<Arc<ReadPage>, MapError> {
    static CACHES: OnceLock<[Mutex<Cache>; SHARDS]> = OnceLock::new();
    let caches = CACHES.get_or_init(|| {
        std::array::from_fn(|_| Mutex::new(Cache::new(SHARD_ENTRIES, SHARD_BYTES)))
    });
    load_with_cache(
        store,
        digest,
        work,
        &caches[usize::from(digest.bytes()[0]) % SHARDS],
    )
}

fn load_with_cache<S: PageStore + ?Sized>(
    store: &S,
    digest: PageDigest,
    work: &mut MapWork,
    cache: &Mutex<Cache>,
) -> Result<Arc<ReadPage>, MapError> {
    // Admission and current bytes precede cache access, including on a warm path.
    // A digest alone cannot authorize reuse, hide missing storage, or grant capacity.
    let bytes = read_page_bytes(store, digest, work)?;
    let hit = cache
        .lock()
        .ok()
        .and_then(|cache| cache.entries.get(&digest).cloned());
    if let Some(page) = hit.filter(|page| page.bytes == bytes) {
        work.admit_entries_visited(page.visited)?;
        return Ok(page);
    }
    let before = work.entries_visited;
    let page = decode_page_with_admission(&bytes, Some(work))?;
    add_counter(
        &mut work.pages_decoded,
        1,
        "persistent_map_work_pages_decoded",
    )?;
    let summary = PageLinkSummary::from_page(&page)?;
    let retained = retained_bytes(&page, &summary, bytes.capacity()).unwrap_or(usize::MAX);
    let page = Arc::new(ReadPage {
        page,
        summary,
        bytes,
        visited: work.entries_visited - before,
        retained,
    });
    // Expensive decoding never holds a cache lock. Concurrent misses may prepare
    // independently; only one immutable value is retained under each exact key.
    if let Ok(mut cache) = cache.lock() {
        cache.retain(digest, Arc::clone(&page));
    }
    Ok(page)
}

fn retained_bytes(
    page: &Page,
    summary: &PageLinkSummary,
    encoded_capacity: usize,
) -> Option<usize> {
    // Charge actual vector capacities plus per-allocation and index overhead.
    // The 64 MiB process ceiling models retained material, not process RSS.
    let mut retained = 512_usize
        .checked_add(std::mem::size_of::<ReadPage>())?
        .checked_add(encoded_capacity)?
        .checked_add(summary.prefix.capacity())?;
    let prefix = match page {
        Page::Leaf {
            prefix, entries, ..
        } => {
            retained = retained.checked_add(
                entries
                    .capacity()
                    .checked_mul(std::mem::size_of::<Entry>())?,
            )?;
            for entry in entries {
                retained = retained
                    .checked_add(entry.key.capacity())?
                    .checked_add(entry.value.capacity())?
                    .checked_add(64)?;
            }
            prefix
        }
        Page::Branch {
            prefix,
            terminal,
            children,
            ..
        } => {
            retained = retained
                .checked_add(
                    children
                        .capacity()
                        .checked_mul(std::mem::size_of::<ChildRef>())?,
                )?
                .checked_add(terminal.as_ref().map_or(0, Vec::capacity))?;
            prefix
        }
    };
    retained.checked_add(prefix.capacity())
}

#[cfg(test)]
#[path = "persistent_map_read_cache_tests.rs"]
mod tests;
