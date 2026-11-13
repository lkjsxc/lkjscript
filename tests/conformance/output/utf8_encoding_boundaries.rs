// Exhaust all one/two-byte inputs, then three/four-byte boundary continuations.
// Each bit is the validity of one input, in the fixed order below.
fn observe(
    bytes: &[u8],
    bits: &mut Vec<u8>,
    packed: &mut u8,
    position: &mut u8,
    count: &mut usize,
) {
    if std::str::from_utf8(bytes).is_ok() {
        *packed = *packed | (1u8 << *position);
    }
    *count += 1usize;
    if *position == 7u8 {
        bits.push(*packed);
        *packed = 0u8;
        *position = 0u8;
    } else {
        *position += 1u8;
    }
}
pub fn main(mut world: World) {
    let mut bits: Vec<u8> = Vec::new();
    let mut packed = 0u8;
    let mut position = 0u8;
    let mut count = 0usize;
    let empty: [u8; 0] = [];
    observe(&empty, &mut bits, &mut packed, &mut position, &mut count);
    let mut first = 0u16;
    while first < 256u16 {
        observe(
            &[first as u8],
            &mut bits,
            &mut packed,
            &mut position,
            &mut count,
        );
        let mut second = 0u16;
        while second < 256u16 {
            observe(
                &[first as u8, second as u8],
                &mut bits,
                &mut packed,
                &mut position,
                &mut count,
            );
            second += 1u16;
        }
        first += 1u16;
    }
    let boundary = [127u8, 128u8, 191u8, 192u8];
    first = 224u16;
    while first <= 239u16 {
        let mut second = 128u16;
        while second <= 191u16 {
            let mut index = 0usize;
            while index < boundary.len() {
                observe(
                    &[first as u8, second as u8, boundary[index]],
                    &mut bits,
                    &mut packed,
                    &mut position,
                    &mut count,
                );
                index += 1usize;
            }
            second += 1u16;
        }
        first += 1u16;
    }
    first = 240u16;
    while first <= 244u16 {
        let mut second = 128u16;
        while second <= 191u16 {
            let mut third_index = 0usize;
            while third_index < 2usize {
                let third = if third_index == 0usize { 128u8 } else { 191u8 };
                let mut index = 0usize;
                while index < boundary.len() {
                    observe(
                        &[first as u8, second as u8, third, boundary[index]],
                        &mut bits,
                        &mut packed,
                        &mut position,
                        &mut count,
                    );
                    index += 1usize;
                }
                third_index += 1usize;
            }
            second += 1u16;
        }
        first += 1u16;
    }
    observe(
        &[0xf0u8, 0x9fu8, 0x98u8],
        &mut bits,
        &mut packed,
        &mut position,
        &mut count,
    );
    observe(
        &[0xf4u8, 0x8fu8, 0xbfu8],
        &mut bits,
        &mut packed,
        &mut position,
        &mut count,
    );
    observe(
        &[0xe0u8, 0xa0u8],
        &mut bits,
        &mut packed,
        &mut position,
        &mut count,
    );
    observe(
        &[0x61u8, 0xc2u8, 0x80u8],
        &mut bits,
        &mut packed,
        &mut position,
        &mut count,
    );
    observe(
        &[0xc2u8, 0x80u8, 0x61u8],
        &mut bits,
        &mut packed,
        &mut position,
        &mut count,
    );
    observe(
        &[0xe2u8, 0x82u8, 0xacu8, 0x61u8],
        &mut bits,
        &mut packed,
        &mut position,
        &mut count,
    );
    observe(
        &[0x61u8, 0xe2u8, 0x82u8, 0xacu8],
        &mut bits,
        &mut packed,
        &mut position,
        &mut count,
    );
    observe(
        &[0xc2u8, 0x80u8, 0xc2u8, 0x80u8],
        &mut bits,
        &mut packed,
        &mut position,
        &mut count,
    );
    observe(
        &[0x61u8, 0x62u8, 0x63u8, 0x64u8],
        &mut bits,
        &mut packed,
        &mut position,
        &mut count,
    );
    assert_eq!(count, 72458usize);
    if position > 0u8 {
        bits.push(packed);
    }
    let mut output = world.stdout();
    output.write(bits.as_slice());
}
