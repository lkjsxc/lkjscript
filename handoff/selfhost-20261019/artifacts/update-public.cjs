const fs=require('fs');
const p='tests/public_cli/native_elf.rs';let s=fs.readFileSync(p,'utf8');
let start=s.indexOf('        format!(\n            "request base={base}');let end=s.indexOf('\n    )\n    .unwrap();',start);
const names=['input','elf','flow','machine','asm','types','custody','profile','values','heap','objects','hash','stream','runtime','aggregate'];
s=s.slice(0,start)+'        format!("request base={base}\\n{}", concat!(\n'+names.map(n=>`            include_str!("../../tools/selfhost/${n}.lkjc"), "\\n",`).join('\n')+'\n        )),'+s.slice(end);
start=s.indexOf('    // Independently encode profile 2:');end=s.indexOf('    let mut digest = blake3::Hasher::new();',start);
s=s.slice(0,start)+`    // Independently encode profile 3: a main function calls checked add(1, 41).
    // No exporter/code generator is used to construct this input.
    let mut payload = vec![0u8; 2390];
    payload[..8].copy_from_slice(b"LKJCI001");
    fn put(bytes: &mut [u8], at: usize, value: u32) {
        bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    put(&mut payload, 8, 3);
    for (at, value) in [(16, 2390u64), (24, 2), (32, 5890)] {
        payload[at..at + 8].copy_from_slice(&value.to_le_bytes());
    }
    for (i, value) in [112, 1, 128, 7, 2176, 2, 2256, 4, 2384, 6, 0, 2, 0, 0]
        .into_iter().enumerate() {
        put(&mut payload, 72 + i * 4, value);
    }
    for ty in 1..=7 {
        put(&mut payload, 128 + (ty - 1) * 16, ty as u32);
    }
    put(&mut payload, 228, 1); // List<I64>'s element type.
    for (i, value) in [0, 1, 0, 0, 0, 4, 0, 2384, 0, 0,
        0, 1, 2, 2, 4, 0, 1, 2388, 0, 0].into_iter().enumerate() {
        put(&mut payload, 2176 + i * 4, value);
    }
    for (i, (tag, a, b, immediate, state, depth)) in [
        (1, 0, 0, 1i64, 2384, 0),
        (1, 0, 0, 41, 2384, 1),
        (9, 1, 2, 0, 2385, 2),
        (11, 0, 0, 0, 2387, 1),
    ].into_iter().enumerate() {
        let at = 2256 + i * 32;
        put(&mut payload, at, tag);
        put(&mut payload, at + 4, a);
        put(&mut payload, at + 8, b);
        payload[at + 16..at + 24].copy_from_slice(&immediate.to_le_bytes());
        put(&mut payload, at + 24, state);
        put(&mut payload, at + 28, depth);
    }
    payload[2384..].fill(1);
`+s.slice(end);
fs.writeFileSync(p,s);
