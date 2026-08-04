use super::*;

#[test]
fn unique_forwarding_epilogue_with_place_ends_is_tail_position() {
    let code = [
        Op::StoreUniqueLocal as u8,
        4,
        Op::ByteVectorPlaceEnd as u8,
        5,
        Op::StoreLocal as u8,
        6,
        Op::Pop as u8,
        Op::ByteVectorPlaceEnd as u8,
        4,
        Op::StoreLocal as u8,
        6,
        Op::Pop as u8,
        Op::TakeUniqueLocal as u8,
        4,
        Op::Return as u8,
    ];
    assert!(forwarding_epilogue(&code, 0));
}
