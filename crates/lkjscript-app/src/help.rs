pub fn print() {
    println!("lkjscript - typed line-oriented language runtime");
    println!();
    println!("Usage:");
    println!(
        "  lkjscript run [--engine vm|auto|baseline-jit|optimizing-jit] [--auto-jit-threshold N]"
    );
    println!("                 [--disable-auto-jit] [--resource-profile NAME]");
    println!("                 <file.lkjscript> [--] [script-args...]");
    println!("                 default: auto at 64 function entries; explicit vm is deterministic");
    println!("  lkjscript describe [--json]");
    println!("  lkjscript package <lock|check> [package-path]");
    println!("  lkjscript memory inventory [--json]");
    println!("  lkjscript memory explain <identity>");
    println!("  lkjscript disasm [--resource-profile NAME] <file.lkjscript>");
    println!("  lkjscript semantic describe");
    println!("  lkjscript semantic [-] < request.json");
    println!("  lkjscript semantic serve --stdio");
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
