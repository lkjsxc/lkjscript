//! Optional application deadlines beneath the already-granted document root.
//! Three decimal lines: idle seconds, whole-request seconds, whole-write seconds.
#[derive(Clone, Copy)]
pub struct Policy { pub idle: usize, pub request: usize, pub write: usize }

fn duration(value: Option<&str>, name: &str) -> Result<usize, String> {
    let text = value.ok_or(format!("missing {name} seconds"))?;
    let seconds = match text.parse::<usize>() {
        Ok(seconds) => seconds,
        Err(_) => return Err(format!("invalid {name} seconds")),
    };
    if seconds == 0usize { return Err(format!("{name} seconds must be positive")); }
    seconds.checked_mul(1_000_000_000usize)
        .ok_or(format!("{name} seconds overflow nanoseconds"))
}
fn parse(text: &str) -> Result<Policy, String> {
    let mut lines = text.split('\n');
    let idle = duration(lines.next(), "idle")?;
    let request = duration(lines.next(), "request")?;
    let write = duration(lines.next(), "write")?;
    for extra in lines {
        if !extra.is_empty() { return Err("unexpected line after write seconds".to_string()); }
    }
    Ok(Policy { idle, request, write })
}
pub fn read(root: &mut Directory) -> Result<Policy, String> {
    let mut file = match root.open(".http-policy") {
        Ok(file) => file,
        Err(2i64) => return Ok(Policy {
            idle: 60_000_000_000usize, request: 2_000_000_000usize, write: 2_000_000_000usize
        }),
        Err(errno) => return Err(format!("open .http-policy: errno {errno}")),
    };
    let bytes = match file.read_to_end() {
        Ok(bytes) => bytes,
        Err(errno) => return Err(format!("read .http-policy: errno {errno}")),
    };
    let text = match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(_) => return Err(".http-policy is not UTF-8".to_string()),
    };
    match parse(text.as_str()) {
        Ok(policy) => Ok(policy),
        Err(problem) => Err(format!(".http-policy: {problem}")),
    }
}
#[test]
fn explicit_deadlines() {
    let policy = parse("60\n30\n30\n").unwrap();
    assert_eq!(policy.idle, 60_000_000_000usize);
    assert_eq!(policy.request, 30_000_000_000usize);
    assert_eq!(policy.write, 30_000_000_000usize);
    let fractional_boundary = parse("18446744073\n2\n2").unwrap();
    assert_eq!(fractional_boundary.idle, 18_446_744_073_000_000_000usize);
}
#[test]
fn malformed_deadlines() {
    assert!(parse("").is_err());
    assert!(parse("60\n2").is_err());
    assert!(parse("60\n0\n2").is_err());
    assert!(parse("60\n-1\n2").is_err());
    assert!(parse("60\n2\ntext").is_err());
    assert!(parse("60\n2\n2\nextra").is_err());
    assert!(parse("18446744074\n2\n2").is_err());
    assert!(parse("18446744073709551616\n2\n2").is_err());
}
