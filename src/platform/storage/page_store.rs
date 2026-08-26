//! Persistent-map adapter over the generic immutable object store.

use super::object::{
    ImmutableObjectStore, ObjectDomain, ObjectKey, StageOutcome, StoreError, StoreErrorClass,
    StoreReadAdmission, StoreReadLimits, StoreWork,
};
use crate::platform::persistent_map::{MapError, MapErrorClass, PageDigest, PageStore, PageWrite};
use std::cell::RefCell;

pub struct ObjectPageStore<S> {
    objects: S,
    work: RefCell<StoreWork>,
}

/// Read-only persistent-map adapter. It permits revision-pinned repository views to perform
/// concurrent semantic reads without acquiring a mutable object-store reference or exposing page
/// placement outside the storage layer.
pub struct ObjectPageReader<'a, S: ?Sized> {
    objects: &'a S,
    admission: Option<PageReadAdmission<'a>>,
    work: RefCell<StoreWork>,
}

enum PageReadAdmission<'a> {
    Owned(RefCell<StoreReadAdmission>),
    Shared(&'a RefCell<StoreReadAdmission>),
}

impl PageReadAdmission<'_> {
    fn cell(&self) -> &RefCell<StoreReadAdmission> {
        match self {
            Self::Owned(admission) => admission,
            Self::Shared(admission) => admission,
        }
    }
}

impl<'a, S: ?Sized> ObjectPageReader<'a, S> {
    pub fn new(objects: &'a S) -> Self {
        Self {
            objects,
            admission: None,
            work: RefCell::new(StoreWork::default()),
        }
    }

    /// Creates a reader whose aggregate accepted-object work is admitted before payload access.
    pub fn new_admitted(objects: &'a S, admission: StoreReadAdmission) -> Self {
        Self {
            objects,
            admission: Some(PageReadAdmission::Owned(RefCell::new(admission))),
            work: RefCell::new(StoreWork::default()),
        }
    }

    /// Shares one aggregate object-store admission with canonical reads performed by the owning
    /// revision-pinned operation between map visits.
    pub(crate) fn new_shared_admission(
        objects: &'a S,
        admission: &'a RefCell<StoreReadAdmission>,
    ) -> Self {
        Self {
            objects,
            admission: Some(PageReadAdmission::Shared(admission)),
            work: RefCell::new(StoreWork::default()),
        }
    }

    pub fn work(&self) -> StoreWork {
        *self.work.borrow()
    }

    pub fn remaining_read_admission(&self) -> Option<StoreReadLimits> {
        self.admission
            .as_ref()
            .map(|admission| admission.cell().borrow().remaining())
    }
}

impl<S: ImmutableObjectStore + ?Sized> PageStore for ObjectPageReader<'_, S> {
    fn read_page(
        &self,
        digest: PageDigest,
        maximum_bytes: usize,
    ) -> Result<Option<Vec<u8>>, MapError> {
        let key = ObjectKey::from_digest(ObjectDomain::MapPage, digest.bytes());
        let maximum_bytes = maximum_bytes.min(crate::platform::persistent_map::MAXIMUM_PAGE_BYTES);
        let mut work = self.work.borrow_mut();
        match &self.admission {
            Some(admission) => self.objects.read_admitted(
                key,
                maximum_bytes,
                &mut admission.cell().borrow_mut(),
                &mut work,
            ),
            None => self.objects.read(key, maximum_bytes, &mut work),
        }
        .map_err(map_error)
    }

    fn write_page(&mut self, _digest: PageDigest, _bytes: &[u8]) -> Result<PageWrite, MapError> {
        Err(MapError {
            class: MapErrorClass::Input,
            code: "object_page_reader_write",
            message: "read-only object-page adapter cannot stage a persistent-map page".to_owned(),
        })
    }
}

impl<S> ObjectPageStore<S> {
    pub fn new(objects: S) -> Self {
        Self {
            objects,
            work: RefCell::new(StoreWork::default()),
        }
    }

    pub fn objects(&self) -> &S {
        &self.objects
    }

    pub fn objects_mut(&mut self) -> &mut S {
        &mut self.objects
    }

    pub fn work(&self) -> StoreWork {
        *self.work.borrow()
    }

    pub fn into_inner(self) -> S {
        self.objects
    }
}

impl<S: ImmutableObjectStore> PageStore for ObjectPageStore<S> {
    fn read_page(
        &self,
        digest: PageDigest,
        maximum_bytes: usize,
    ) -> Result<Option<Vec<u8>>, MapError> {
        let key = ObjectKey::from_digest(ObjectDomain::MapPage, digest.bytes());
        self.objects
            .read(
                key,
                maximum_bytes.min(crate::platform::persistent_map::MAXIMUM_PAGE_BYTES),
                &mut self.work.borrow_mut(),
            )
            .map_err(map_error)
    }

    fn write_page(&mut self, digest: PageDigest, bytes: &[u8]) -> Result<PageWrite, MapError> {
        let key = ObjectKey::from_digest(ObjectDomain::MapPage, digest.bytes());
        self.objects
            .stage(key, bytes, self.work.get_mut())
            .map(|outcome| match outcome {
                StageOutcome::Inserted => PageWrite::Inserted,
                StageOutcome::Reused => PageWrite::Reused,
            })
            .map_err(map_error)
    }
}

fn map_error(error: StoreError) -> MapError {
    MapError {
        class: match error.class {
            StoreErrorClass::Input => MapErrorClass::Input,
            StoreErrorClass::Resource => MapErrorClass::Resource,
            StoreErrorClass::Corrupt => MapErrorClass::Corrupt,
            StoreErrorClass::Io => MapErrorClass::Store,
        },
        code: error.code,
        message: error.message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::storage::memory::MemoryPackedStore;
    use crate::platform::storage::object::StoreReadLimits;

    fn stored_page() -> (MemoryPackedStore, PageDigest, Vec<u8>) {
        let bytes = b"bounded page payload".to_vec();
        let digest = PageDigest::of(&bytes);
        let key = ObjectKey::from_digest(ObjectDomain::MapPage, digest.bytes());
        let mut store = MemoryPackedStore::default();
        store
            .stage(key, &bytes, &mut StoreWork::default())
            .expect("stage page fixture");
        (store, digest, bytes)
    }

    #[test]
    fn admitted_reader_rejects_zero_store_dimensions_before_payload_copy() {
        let (store, digest, bytes) = stored_page();
        for (limits, code, expected_catalog_lookups) in [
            (
                StoreReadLimits {
                    maximum_catalog_lookups: 0,
                    maximum_objects: 1,
                    maximum_bytes: bytes.len() as u64,
                },
                "object_read_catalog_lookups_exhausted",
                0,
            ),
            (
                StoreReadLimits {
                    maximum_catalog_lookups: 1,
                    maximum_objects: 0,
                    maximum_bytes: bytes.len() as u64,
                },
                "object_read_objects_exhausted",
                1,
            ),
            (
                StoreReadLimits {
                    maximum_catalog_lookups: 1,
                    maximum_objects: 1,
                    maximum_bytes: 0,
                },
                "object_read_bytes_exhausted",
                1,
            ),
        ] {
            let reader = ObjectPageReader::new_admitted(&store, StoreReadAdmission::new(limits));
            let error = reader
                .read_page(digest, bytes.len())
                .expect_err("zero accepted-store dimension must reject");
            assert_eq!(error.code, code);
            assert_eq!(reader.work().catalog_lookups, expected_catalog_lookups);
            assert_eq!(reader.work().objects_read, 0);
            assert_eq!(reader.work().bytes_read, 0);
        }
    }

    #[test]
    fn admitted_reader_accepts_exact_store_dimensions() {
        let (store, digest, bytes) = stored_page();
        let reader = ObjectPageReader::new_admitted(
            &store,
            StoreReadAdmission::new(StoreReadLimits {
                maximum_catalog_lookups: 1,
                maximum_objects: 1,
                maximum_bytes: bytes.len() as u64,
            }),
        );
        assert_eq!(
            reader
                .read_page(digest, bytes.len())
                .expect("exact accepted-store admission"),
            Some(bytes.clone())
        );
        assert_eq!(reader.work().catalog_lookups, 1);
        assert_eq!(reader.work().objects_read, 1);
        assert_eq!(reader.work().bytes_read, bytes.len() as u64);
    }
}
