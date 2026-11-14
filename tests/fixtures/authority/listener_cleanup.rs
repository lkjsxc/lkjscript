// expect: check
// authority-expect: --grant incoming=ADDRESS; listener_address; listener_close
pub fn main(mut incoming: Listener) {
    let _ = incoming.port().unwrap();
    drop(incoming);
}
