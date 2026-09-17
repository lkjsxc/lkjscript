#![forbid(unsafe_code)]

fn main() -> std::process::ExitCode {
    lkjscript_dev::release_controller_entry(std::env::args_os())
}
