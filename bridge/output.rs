//! The rustc-only conformance host. Source programs receive capabilities but
//! cannot construct them; only the wrapper calls enter with the language entry.
use std::io::Write;

pub struct World {
    _seal: (),
}
pub struct Stdout {
    _seal: (),
}

impl World {
    pub fn stdout(&mut self) -> Stdout {
        Stdout { _seal: () }
    }
}
impl Stdout {
    pub fn write(&mut self, bytes: &[u8]) {
        if let Err(error) = std::io::stdout().write_all(bytes) {
            eprintln!("LKJ_RUST_TRAP R0007 {error}");
            std::process::exit(101);
        }
    }
}

pub fn enter(entry: fn(World)) {
    // The language traps without unwinding. The oracle hook exits before Rust
    // unwinds so output/destructor observations obey that declared departure.
    std::panic::set_hook(Box::new(|info| {
        let text = info.to_string();
        let kind = if text.contains("overflow") {
            "R0001"
        } else if text.contains("divide by zero") || text.contains("divisor of zero") {
            "R0002"
        } else if text.contains("index out of bounds") {
            "R0003"
        } else if text.contains("assertion") {
            "R0005"
        } else {
            "R0004"
        };
        eprintln!("LKJ_RUST_TRAP {kind}");
        std::process::exit(101);
    }));
    entry(World { _seal: () });
}
