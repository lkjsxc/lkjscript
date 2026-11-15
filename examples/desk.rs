//! Desk commands and the isolated HTTP candidate have separate authority entries.
mod http_protocol;
mod http_policy;
mod http_server;
mod desk {
    pub mod store;
    pub mod cli;
    pub mod web;
}

pub fn main(mut args: Arguments, mut state: Directory, mut state_env: Environment,
    mut timezone: Environment, mut output: Stdout, mut errors: Stderr, mut input: Stdin,
    mut clock: Clock, mut entropy: Entropy, mut exit: ExitStatus) {
    let result = desk::cli::run(&mut args, &mut state, &mut state_env, &mut timezone,
        &mut output, &mut input, &mut clock, &mut entropy);
    match result {
        Ok(status) => exit.set_status(status),
        Err(problem) => {
            errors.write(format!("desk: {problem}\n").as_bytes());
            exit.set_status(1u8);
        }
    }
}

pub fn serve(output: Stdout, state: Directory, clock: Clock, id_clock: Clock,
    entropy: Entropy, listener: Listener, ready: Readiness, stop: Stop) {
    let policy = http_policy::Policy {
        idle: 60_000_000_000usize, request: 2_000_000_000usize,
        write: 2_000_000_000usize, shutdown: 5_000_000_000usize,
    };
    let handler = desk::web::Desk { state, clock: id_clock, entropy };
    http_server::serve(output, listener, clock, ready, stop, policy, handler);
}
