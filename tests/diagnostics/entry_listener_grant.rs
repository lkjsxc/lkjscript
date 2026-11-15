// expect: check
// pending: Include the granted numeric endpoint in the located startup failure; Listener kind and errno now identify the refusal instead of blaming output.
pub fn main(mut incoming: Listener) {
    let _ = incoming.port().unwrap();
}
