//! Unsigned and signed LEB128 encoders used by the production WebAssembly emitter.
//! Inputs span their full u64/i64 representations; output grows as bytes are appended.

pub fn uleb(out: &mut Vec<u8>, mut value: u64) {
    loop {
        let byte = (value & 127) as u8;
        value >>= 7;
        out.push(if value == 0 { byte } else { byte | 128 });
        if value == 0 {
            break;
        }
    }
}
pub fn sleb(out: &mut Vec<u8>, mut value: i64) {
    loop {
        let byte = (value & 127) as u8;
        value >>= 7;
        let done = (value == 0 && byte & 64 == 0) || (value == -1 && byte & 64 != 0);
        out.push(if done { byte } else { byte | 128 });
        if done {
            break;
        }
    }
}
