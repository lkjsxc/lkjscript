// expect: reject E_UNSUPPORTED
// rewrite: str.trim() is unavailable; std::str::trim_ascii(value) strips only bytes 9..13 and 32 and preserves Unicode spaces. An automatic edit requires an ASCII literal receiver.
pub fn probe() -> usize { " 3600\n".trim().len() }
