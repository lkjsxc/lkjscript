//! Persistent-map adapter over the generic immutable object store.

use super::object::{
    ImmutableObjectStore, ObjectDomain, ObjectKey, StageOutcome, StoreError, StoreErrorClass,
    StoreWork,
};
use crate::platform::persistent_map::{MapError, MapErrorClass, PageDigest, PageStore, PageWrite};
use std::cell::RefCell;

pub struct ObjectPageStore<S> {
    objects: S,
    work: RefCell<StoreWork>,
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
    fn read_page(&self, digest: PageDigest) -> Result<Option<Vec<u8>>, MapError> {
        let key = ObjectKey::from_digest(ObjectDomain::MapPage, digest.bytes());
        self.objects
            .read(
                key,
                crate::platform::persistent_map::MAXIMUM_PAGE_BYTES,
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
