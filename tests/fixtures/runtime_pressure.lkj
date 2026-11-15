fn grow(n: i64, parent: &i64, a: i64, b: i64, c: i64, d: i64, e: i64, f: i64) -> i64 {
    let page = [n; 1024];
    if n == 0 { return *parent + a + b + c + d + e + f; }
    let result = grow(n - 1, &page[17], a, b, c, d, e, f);
    assert!(page[17] == n);
    result
}
fn main(mut world: World) {
    let mut values = Vec::new();
    let mut index = 0usize;
    while index < 2097152 {
        values.push(7i64);
        index += 1;
    }
    let marker = 41i64;
    let result = grow(96, &marker, 1, 2, 3, 4, 5, 6);
    assert_eq!(result + values[0] + values[2097151], 36i64);
    world.stdout().write(b"ok pressure\n");
}
