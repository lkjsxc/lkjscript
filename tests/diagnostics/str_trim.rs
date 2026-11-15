// expect: reject E_UNSUPPORTED
// pending: Explain that str.trim() is unavailable and suggest std::str::trim_ascii(value) when stripping only ASCII bytes 9..13 and 32 is intended; unlike Unicode trim, it preserves non-breaking and other Unicode spaces.
pub fn probe() -> usize { " 3600\n".trim().len() }
