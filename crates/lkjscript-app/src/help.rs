pub fn print() {
    println!("lkjscript - typed line-oriented language runtime");
    println!();
    println!("Usage:");
    println!("  lkjscript run <file.lkjscript> [--] [script-args...]");
    println!("                 one baseline-native attempt, then VM fallback before entry");
    println!("  lkjscript describe [--json]");
    println!("  lkjscript package <lock|check> [package-path]");
    println!("  lkjscript memory inventory [--json]");
    println!("  lkjscript memory explain <identity>");
    println!("  lkjscript disasm <file.lkjscript>");
    println!("  lkjscript --help");
    println!("  lkjscript --version");
    println!();
    println!("Environment:");
    println!(
        "  LKJSCRIPT_JIT_DIAGNOSTICS  emit SSA, bytes, relocations, metadata, counts to stderr"
    );
    println!("  LKJSCRIPT_JIT_DUMP_DIR     write generated .bin files for external objdump");
    println!("  LKJSCRIPT_METRICS          emit one low-overhead JSON metrics line to stderr");
    println!("  LKJSCRIPT_METRICS_FILE     write that metrics line to an explicit file instead");
}
