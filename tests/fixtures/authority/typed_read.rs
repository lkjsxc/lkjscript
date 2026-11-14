// expect: check
// authority-expect: --grant send; --grant tree=PATH; --grant timer
// CLI grants permit only this directory and the declared output/clock operations.
pub fn main(mut send: Stdout, mut tree: Directory, mut timer: Clock) {
    let before = timer.nanoseconds().unwrap();
    let mut file = tree.open("authority-input").unwrap();
    let bytes = file.read_to_end().unwrap();
    assert_eq!(&bytes[..], b"granted\0\xff\n");
    let after = timer.nanoseconds().unwrap();
    assert!(after >= before);
    send.write(&bytes);
}
