pub fn probe() -> i64 {
    let _text = std::fs::read_to_string("/etc/hostname");
    0
}
