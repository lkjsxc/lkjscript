//! Reduced-limit recovery scans must admit each footer before index materialization.

use super::super::object::{ObjectDomain, ObjectKey, StoreErrorClass};
use super::super::pack::{HEADER_BYTES, PackBuilder, PackMetadata, SealedPack, TRAILER_BYTES};
use super::scan_pack_metadata_with_limit;
use std::cell::Cell;
use std::io::{self, Cursor, Read, Seek, SeekFrom};
use std::rc::Rc;

fn two_entry_pack(ordinal: u8) -> SealedPack {
    let mut builder = PackBuilder::default();
    for entry in 0..2_u8 {
        let bytes = [ordinal, entry];
        let key = ObjectKey::for_bytes(ObjectDomain::Blob, &bytes);
        builder
            .insert(key, &bytes)
            .expect("stage small fixture object");
    }
    let pack = builder.seal().expect("seal valid two-entry pack");
    assert_eq!(pack.metadata.entries.len(), 2);
    pack
}

struct IndexReadTripwire {
    cursor: Cursor<Vec<u8>>,
    index_start: u64,
    index_end: u64,
    index_reads: Rc<Cell<usize>>,
    forbid_index: bool,
}

impl IndexReadTripwire {
    fn new(pack: &SealedPack, index_reads: Rc<Cell<usize>>, forbid_index: bool) -> Self {
        let trailer_start = pack.bytes.len() - TRAILER_BYTES;
        let index_end = u64::from_be_bytes(
            pack.bytes[trailer_start..trailer_start + 8]
                .try_into()
                .expect("fixture footer offset"),
        );
        Self {
            cursor: Cursor::new(pack.bytes.clone()),
            index_start: HEADER_BYTES as u64 + pack.metadata.payload_bytes,
            index_end,
            index_reads,
            forbid_index,
        }
    }
}

impl Read for IndexReadTripwire {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let position = self.cursor.position();
        if !buffer.is_empty()
            && position < self.index_end
            && position.saturating_add(buffer.len() as u64) > self.index_start
        {
            self.index_reads.set(self.index_reads.get() + 1);
            assert!(!self.forbid_index, "over-limit footer index was read");
        }
        self.cursor.read(buffer)
    }
}

impl Seek for IndexReadTripwire {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.cursor.seek(position)
    }
}

#[test]
fn aggregate_four_rejects_third_two_entry_footer_before_index_read() {
    let packs = [two_entry_pack(0), two_entry_pack(1), two_entry_pack(2)];
    let names = packs.iter().map(|pack| pack.id.file_name()).collect();
    let index_reads = [
        Rc::new(Cell::new(0)),
        Rc::new(Cell::new(0)),
        Rc::new(Cell::new(0)),
    ];
    let mut opened = 0;
    let error = match scan_pack_metadata_with_limit(names, 4, |name| {
        let ordinal = opened;
        opened += 1;
        let pack = &packs[ordinal];
        assert_eq!(name, pack.id.file_name());
        Ok((
            IndexReadTripwire::new(pack, Rc::clone(&index_reads[ordinal]), ordinal == 2),
            pack.bytes.len() as u64,
        ))
    }) {
        Ok(_) => panic!("six physical entries must exceed the four-entry aggregate bound"),
        Err(error) => error,
    };
    assert_eq!(error.class, StoreErrorClass::Resource);
    assert_eq!(error.code, "catalog_entry_count");
    assert_eq!(opened, 3, "third footer must reach admission");
    assert_eq!(index_reads[0].get(), 1);
    assert_eq!(index_reads[1].get(), 1);
    assert_eq!(index_reads[2].get(), 0);
}

#[test]
fn aggregate_four_accepts_exactly_two_two_entry_footers() {
    let packs = [two_entry_pack(0), two_entry_pack(1)];
    let names = packs.iter().map(|pack| pack.id.file_name()).collect();
    let index_reads = [Rc::new(Cell::new(0)), Rc::new(Cell::new(0))];
    let mut opened = 0;
    let scan = scan_pack_metadata_with_limit(names, 4, |name| {
        let ordinal = opened;
        opened += 1;
        let pack = &packs[ordinal];
        assert_eq!(name, pack.id.file_name());
        Ok((
            IndexReadTripwire::new(pack, Rc::clone(&index_reads[ordinal]), false),
            pack.bytes.len() as u64,
        ))
    })
    .expect("exact aggregate boundary must admit both complete footers");
    assert_eq!(opened, 2);
    assert_eq!(scan.metadata.len(), 2);
    assert_eq!(
        scan.metadata
            .values()
            .map(|metadata| metadata.entries.len())
            .sum::<usize>(),
        4
    );
    for (ordinal, pack) in packs.iter().enumerate() {
        assert_eq!(scan.metadata.get(&pack.id), Some(&pack.metadata));
        assert_eq!(index_reads[ordinal].get(), 1);
    }
}

#[test]
fn ordinary_footer_reader_has_no_finite_entry_allowance() {
    let pack = two_entry_pack(0);
    let mut reader = Cursor::new(&pack.bytes);
    let read = PackMetadata::read_footer(&mut reader, pack.bytes.len() as u64)
        .expect("ordinary footer reader must admit a valid pack");
    assert_eq!(read.metadata, pack.metadata);
}

#[test]
fn aggregate_admission_preserves_index_authentication() {
    let mut pack = two_entry_pack(0);
    let index_start = HEADER_BYTES + pack.metadata.payload_bytes as usize;
    pack.bytes[index_start + 1] ^= 1;
    let mut reader = Cursor::new(&pack.bytes);
    let error = PackMetadata::read_footer_with_entry_allowance(
        &mut reader,
        pack.bytes.len() as u64,
        Some(4),
    )
    .expect_err("an admitted count must not bypass index authentication");
    assert_eq!(error.class, StoreErrorClass::Corrupt);
    assert_eq!(error.code, "pack_index_checksum");
}

#[test]
fn per_pack_count_validation_precedes_aggregate_admission() {
    let mut pack = two_entry_pack(0);
    let trailer_start = pack.bytes.len() - TRAILER_BYTES;
    let footer = u64::from_be_bytes(
        pack.bytes[trailer_start..trailer_start + 8]
            .try_into()
            .expect("fixture footer offset"),
    ) as usize;
    let excessive = pack.metadata.entries.len() as u64 + 1;
    pack.bytes[footer + 12..footer + 20].copy_from_slice(&excessive.to_be_bytes());
    let index_reads = Rc::new(Cell::new(0));
    let mut reader = IndexReadTripwire::new(&pack, Rc::clone(&index_reads), true);
    let error = PackMetadata::read_footer_with_entry_allowance(
        &mut reader,
        pack.bytes.len() as u64,
        Some(0),
    )
    .expect_err("per-pack bounds must remain independent of the aggregate allowance");
    assert_eq!(error.class, StoreErrorClass::Resource);
    assert_eq!(error.code, "pack_index_size");
    assert_eq!(index_reads.get(), 0);
}

#[test]
fn optional_footer_entry_policy_is_exact_before_index_allocation() {
    let pack = two_entry_pack(0);
    for maximum in [None, Some(0), Some(1), Some(2), Some(usize::MAX)] {
        let rejected = maximum.is_some_and(|maximum| maximum < 2);
        let index_reads = Rc::new(Cell::new(0));
        let mut reader = IndexReadTripwire::new(&pack, Rc::clone(&index_reads), rejected);
        let result = PackMetadata::read_footer_with_entry_allowance(
            &mut reader,
            pack.bytes.len() as u64,
            maximum,
        );
        if rejected {
            assert_eq!(result.unwrap_err().code, "catalog_entry_count");
            assert_eq!(index_reads.get(), 0);
        } else {
            assert_eq!(result.unwrap().metadata, pack.metadata);
            assert_eq!(index_reads.get(), 1);
        }
    }
}

#[test]
fn malformed_footer_framing_rejects_before_index_allocation() {
    for (offset, code) in [
        (0, "pack_magic"),
        (8, "pack_contract"),
        (10, "pack_contract"),
    ] {
        let mut pack = two_entry_pack(0);
        pack.bytes[offset] ^= 0x80;
        let index_reads = Rc::new(Cell::new(0));
        let mut reader = IndexReadTripwire::new(&pack, Rc::clone(&index_reads), true);
        let error = PackMetadata::read_footer(&mut reader, pack.bytes.len() as u64).unwrap_err();
        assert_eq!(error.code, code);
        assert_eq!(index_reads.get(), 0);
    }
    for (relative_offset, code) in [
        (8, "pack_index_contract"),
        (10, "pack_index_contract"),
        (20, "pack_payload_length"),
    ] {
        let mut pack = two_entry_pack(0);
        let footer = pack.bytes.len() - TRAILER_BYTES - super::super::pack::FOOTER_BYTES;
        pack.bytes[footer + relative_offset] ^= 0x80;
        let index_reads = Rc::new(Cell::new(0));
        let mut reader = IndexReadTripwire::new(&pack, Rc::clone(&index_reads), true);
        let error = PackMetadata::read_footer(&mut reader, pack.bytes.len() as u64).unwrap_err();
        assert_eq!(error.code, code);
        assert_eq!(index_reads.get(), 0);
    }
}
