fn timed_read(
    mut output: ChildStdout,
    timeout: Duration,
    child: &mut Child,
) -> Result<(ChildStdout, ProcessResponse), String> {
    let worker = std::thread::spawn(move || {
        let response = read_response(&mut output);
        (output, response)
    });
    let deadline = Instant::now() + timeout;
    while !worker.is_finished() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1));
    }
    if !worker.is_finished() {
        terminate(child);
    }
    let (output, response) = worker
        .join()
        .map_err(|_| "process protocol reader panicked".to_string())?;
    let response = response.map_err(|error| format!("read process response: {error}"))?;
    Ok((output, response))
}

fn wait_or_terminate(child: &mut Child, timeout: Duration) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    loop {
        if child
            .try_wait()
            .map_err(|error| format!("wait for process cell: {error}"))?
            .is_some()
        {
            return Ok(());
        }
        if Instant::now() >= deadline {
            terminate(child);
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn terminate(child: &mut Child) {
    if child.try_wait().ok().flatten().is_none() {
        let _ = child.kill();
    }
    let _ = child.wait();
}
