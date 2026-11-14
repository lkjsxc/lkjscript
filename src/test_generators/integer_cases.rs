// Exhaustive arithmetic boundary table: three types, five operations, thirteen pairs.
// Expected values are independent constants, not computed by the tested operation.
fn operands(index: usize) -> (i64, i64) {
    match index {
        0 => (0, 0),
        1 => (1, 0),
        2 => (0, 1),
        3 => (-1, 1),
        4 => (9223372036854775807, 1),
        5 => (-9223372036854775808, -1),
        6 => (-1, -1),
        7 => (-9223372036854775808, 2),
        8 => (4611686018427387904, 4),
        9 => (2147483647, 2),
        10 => (-2147483648, -1),
        11 => (255, 255),
        _ => (-1, 2),
    }
}
fn expected(row: usize) -> &'static str {
    match row{
        0=>"0 1 1 0 0 -1 -2 2 4 R0001 R0001 510 1",
        1=>"0 1 -1 -2 -2 1 0 -2 -4 2147483645 -2147483647 0 -3",
        2=>"0 0 0 -1 -1 0 1 0 0 R0001 R0001 65025 -2",
        3=>"R0002 R0002 0 -1 -1 0 1 0 0 1073741823 R0001 1 0",
        4=>"R0002 R0002 0 0 0 0 0 0 0 1 R0001 0 -1",
        5=>"0 1 1 0 R0001 R0001 -2 -9223372036854775806 4611686018427387908 2147483649 -2147483649 510 1",
        6=>"0 1 -1 -2 9223372036854775806 -9223372036854775807 0 R0001 4611686018427387900 2147483645 -2147483647 0 -3",
        7=>"0 0 0 -1 9223372036854775807 R0001 1 R0001 R0001 4294967294 2147483648 65025 -2",
        8=>"R0002 R0002 0 -1 9223372036854775807 R0001 1 -4611686018427387904 1152921504606846976 1073741823 2147483648 1 0",
        9=>"R0002 R0002 0 0 0 R0001 0 0 0 1 0 0 -1",
        10=>"0 1 1 R0001 9223372036854775808 R0001 R0001 9223372036854775810 4611686018427387908 2147483649 R0001 510 R0001",
        11=>"0 1 R0001 18446744073709551614 9223372036854775806 R0001 0 9223372036854775806 4611686018427387900 2147483645 R0001 0 18446744073709551613",
        12=>"0 0 0 18446744073709551615 9223372036854775807 R0001 R0001 R0001 R0001 4294967294 R0001 65025 R0001",
        13=>"R0002 R0002 0 18446744073709551615 9223372036854775807 0 1 4611686018427387904 1152921504606846976 1073741823 0 1 9223372036854775807",
        _=>"R0002 R0002 0 0 0 9223372036854775808 0 0 0 1 18446744071562067968 0 1",
    }
}
pub fn cases() -> Vec<(String, String)> {
    let mut cases: Vec<(String, String)> = Vec::new();
    for ty in 2..5usize {
        for operation in 0..5usize {
            let results = super::whitespace(expected((ty - 2) * 5 + operation));
            let operator = match operation {
                0 => "+",
                1 => "-",
                2 => "*",
                3 => "/",
                _ => "%",
            };
            for index in 0..13usize {
                let (a, b) = operands(index);
                let a = if ty == 2 { a as i32 as i64 } else { a };
                let b = if ty == 2 { b as i32 as i64 } else { b };
                let result = &results[index];
                let expectation = if result.starts_with("R") {
                    format!("trap {result}\n# stdout:")
                } else {
                    format!("value {result}")
                };
                let name = format!("integer_{ty}_{operation}_{index}.lkjir");
                let text=format!("lkjscript-ir 1\n# expect: {expectation}\ntype \"()\" unit\ntype \"bool\" bool\ntype \"i32\" int 32 signed\ntype \"i64\" int 64 signed\ntype \"usize\" int 64 unsigned\ntype \"u8\" int 8 unsigned\ntype \"f64\" f64\nfn \"probe\" 0 {ty}\nlocal \"a\" {ty} let\nlocal \"b\" {ty} let\nlocal \"result\" {ty} let\n@1 int 0 {a}\n@2 int 1 {b}\n@3 binary 2 \"{operator}\" 0 1\n@4 return 2\nendfn\n");
                cases.push((name, text));
            }
        }
    }
    cases
}
