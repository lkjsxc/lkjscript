pub struct RunOptions {
    pub file: String,
    pub script_args: Vec<String>,
}

pub fn parse_run(args: &[String]) -> Result<RunOptions, String> {
    let mut index = 1_usize;
    if let Some(unknown) = args
        .get(index)
        .filter(|argument| argument.starts_with("--"))
    {
        return Err(format!("unknown run option: {unknown}"));
    }
    let file = args
        .get(index)
        .ok_or_else(|| "run needs a .lkjscript path".to_string())?
        .clone();
    index += 1;
    if args.get(index).map(String::as_str) == Some("--") {
        index += 1;
    }
    let script_args = args.get(index..).unwrap_or_default().to_vec();
    Ok(RunOptions { file, script_args })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::parse_run;

    #[test]
    fn run_accepts_only_a_file_and_script_arguments() {
        let parsed = parse_run(&[
            "run".into(),
            "main.lkjscript".into(),
            "--".into(),
            "argument".into(),
        ])
        .expect("parse ordinary run");
        assert_eq!(parsed.file, "main.lkjscript");
        assert_eq!(parsed.script_args, ["argument"]);

        for removed in ["--engine", "--auto-jit-threshold", "--disable-auto-jit"] {
            assert!(
                parse_run(&["run".into(), removed.into(), "main.lkjscript".into()]).is_err(),
                "removed option {removed} must reject"
            );
        }
        assert!(parse_run(&[
            "run".into(),
            "--resource-profile".into(),
            "sandbox".into(),
            "main.lkjscript".into(),
        ])
        .is_err());
    }
}
