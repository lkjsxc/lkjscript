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

pub const OPTION_VALUE_ID: [u8; 32] = stable(
    0x4d47c495daebb1ee,
    0xb513ef9b3d616ec1,
    0x7a03c478d3c7ba28,
    0x87c0911f846ef4fe,
);
pub const OPTION_LAYOUT: [u8; 32] = stable(
    0x5965300224cf5710,
    0x7d473982080110d8,
    0x1c6f0f5eb39b6fb0,
    0xd48e4102e4ddc5d1,
);
pub const RESULT_OK_VALUE_ID: [u8; 32] = stable(
    0xf5def33d3834d2e6,
    0x88f88e5d10ff53a8,
    0xb11a5e10ad7c8e5d,
    0x0589557e4eb2c75c,
);
pub const RESULT_ERR_ERROR_ID: [u8; 32] = stable(
    0x2ff3ecf17942d3d0,
    0xa8089d8bf7558bfa,
    0x3ae07305e3b682f1,
    0x1e818c51f0f61075,
);
pub const RESULT_LAYOUT: [u8; 32] = stable(
    0xad73350ff048d2f4,
    0x87d7e61e526ab73c,
    0x5020f64837b8fddf,
    0xd10ebb350984c9f0,
);
pub const UTF8_ERROR_LAYOUT: [u8; 32] = stable(
    0x26e6a342f998fb19,
    0x2ecaf66e53c837ff,
    0xcc66243608018330,
    0xf526c14018cca102,
);
pub const UTF8_ERROR_VARIANTS: [[u8; 32]; 6] = [
    stable(
        0x334ab5997eee386d,
        0x4f2b0d75d0e01d2c,
        0xe7cf0801468ff37a,
        0xf0ec64e1996733d3,
    ),
    stable(
        0x290e4a15e51b3e7f,
        0x0ef25262cbdf4ac8,
        0x9b0c80095826ac19,
        0x8c36757225136c6d,
    ),
    stable(
        0xd2b57f672323c85f,
        0x7f9b29ed3b838fe3,
        0xbad073a0c610dec4,
        0xf936cd88bf33dd3d,
    ),
    stable(
        0xf350d71715a12ecd,
        0x7821a40bb6169e16,
        0x5608acfd536c089a,
        0xd8f5546f98549726,
    ),
    stable(
        0x57efa29c3830f9f2,
        0xbfe83e7bf1d28287,
        0xc1d97f1c0af965c8,
        0x064f9a9eb666e3ac,
    ),
    stable(
        0xba0b1db16522747c,
        0x4869822d9f07e3b0,
        0x26c583183cc696c9,
        0x92e217d0dc5a0c78,
    ),
];
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
        0x0f363b6ce6a82249,
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
