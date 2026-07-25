pub fn print() {
    println!("lkjscript - typed line-oriented language runtime");
    println!();
    println!("Usage:");
    println!(
        "  lkjscript run [--engine vm|auto|baseline-jit|optimizing-jit] [--auto-jit-threshold N]"
    );
    println!("                 [--disable-auto-jit] <file.lkjscript> [--] [script-args...]");
    println!("                 default: auto at 64 function entries; explicit vm is deterministic");
    println!("  lkjscript disasm <file.lkjscript>");
    println!("  lkjscript --help");
    println!("  lkjscript --version");
    println!();
    println!("Environment:");
    println!("  LKJSCRIPT_ROOT             installed root containing src/std and src/lib");
    println!(
        "  LKJSCRIPT_JIT_DIAGNOSTICS  emit SSA, bytes, relocations, metadata, counts to stderr"
    );
    println!("  LKJSCRIPT_JIT_DUMP_DIR     write generated .bin files for external objdump");
    println!("  LKJSCRIPT_METRICS          emit one low-overhead JSON metrics line to stderr");
    println!("  LKJSCRIPT_METRICS_FILE     write that metrics line to an explicit file instead");
}
