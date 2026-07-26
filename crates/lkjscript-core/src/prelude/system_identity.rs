const fn stable(a: u64, b: u64, c: u64, d: u64) -> [u8; 32] {
    let parts = [
        a.to_be_bytes(),
        b.to_be_bytes(),
        c.to_be_bytes(),
        d.to_be_bytes(),
    ];
    let mut bytes = [0; 32];
    let mut part = 0;
    while part < 4 {
        let mut index = 0;
        while index < 8 {
            bytes[part * 8 + index] = parts[part][index];
            index += 1;
        }
        part += 1;
    }
    bytes
}

pub const SYSTEM_ERROR_ID: [u8; 32] = stable(
    0xd39d9b28d02519bc,
    0xdcfa4a9dfd82b177,
    0x46815a562107f0e3,
    0xcfd8f300d7338e4c,
);
pub const SYSTEM_ERROR_LAYOUT: [u8; 32] = stable(
    0x99b92b22aa82b0d6,
    0x58d208d8fa804ce7,
    0x78f42808fa63a89a,
    0xa962905960a67155,
);
pub const SYSTEM_ERROR_VARIANTS: [[u8; 32]; 8] = [
    stable(
        0x205b7262a85abec1,
        0xfd82421d7889ddce,
        0x9ab509e5fb162705,
        0x97b4393335261774,
    ),
    stable(
        0xc7a6aefae4d90025,
        0x5fff2b9a33b079cf,
        0x0b74673a0fbb5683,
        0x9ccbc8a238b28501,
    ),
    stable(
        0x5ad4ba843a8a23ae,
        0x11a982ea89c57445,
        0x2328e1e53b702329,
        0x3104b249ce9c6a1f,
    ),
    stable(
        0xc64d6edffba128e0,
        0x37698c10af26938f,
        0x7f14c3c081ead17e,
        0xf363b6ce6a82249,
    ),
    stable(
        0xe9bc246043d1966f,
        0xf71cbf496183cc4c,
        0xdeb95e1af0d17db8,
        0xe2021d4ab210c754,
    ),
    stable(
        0x5dce941002ddb299,
        0x531b81e7da42ec55,
        0x68a3f3b0d2b6d7d2,
        0x796a930478eedaf7,
    ),
    stable(
        0xd528e008ff143f2a,
        0x7bc6c945ef909699,
        0x997218f94031db24,
        0xfc613ee744daea81,
    ),
    stable(
        0xdb7d248fc7c6871a,
        0x7783de761a1c816d,
        0x9633dfd0fa05d2a9,
        0xec0e38aa6a37e2c8,
    ),
];
pub const SYSTEM_CODE_ID: [u8; 32] = stable(
    0x0af265b8297829ce,
    0xd8bba25229906d9c,
    0x3152e2c0da8fb2b8,
    0xa7455276cdf04c70,
);
pub const SYSTEM_DETAIL_ID: [u8; 32] = stable(
    0x6cb2359e4767ef4c,
    0x264fdc09c88e6065,
    0x5a5a1dca804b51f9,
    0x953b422a067e8acd,
);
pub const SYSTEM_UTF8_ID: [u8; 32] = stable(
    0x89c2dd711dd06d31,
    0x7f6d949c683fea36,
    0xcda4a6b83479b68e,
    0x00c05251fad6468f,
);
