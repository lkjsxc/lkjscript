use std::error::Error;

use super::*;

#[test]
fn request_and_response_frames_are_exact_and_closed() -> Result<(), Box<dyn Error>> {
    let request = ControlRequest::current(9, [7; 32], ControlOperation::Shutdown)?;
    let frame = encode_request_frame(&request)?;
    assert_eq!(decode_request_frame(&frame), Ok(request));
    let response = ControlResponse {
        request_id: 9,
        result: Ok(ControlSuccess::ShutdownAccepted),
    };
    let frame = encode_response_frame(&response)?;
    assert_eq!(decode_response_frame(&frame), Ok(response));
    Ok(())
}

#[test]
fn application_control_frames_preserve_typed_payloads() -> Result<(), Box<dyn Error>> {
    let operations = [
        ControlOperation::ApplicationInstall(ApplicationInstallRequest {
            name: "counter".into(),
            package: [3; 32],
            package_root: "/tmp/counter".into(),
            entry: "main.lkjscript".into(),
            capabilities: vec![lkjscript_core::CapabilityKind::Stdio],
            max_concurrent_invocations: 2,
            max_total_invocations: 9,
        }),
        ControlOperation::ApplicationInvoke {
            application: 7,
            arguments: vec!["one".into(), "two".into()],
        },
    ];
    for operation in operations {
        let request = ControlRequest::current(11, [4; 32], operation)?;
        let frame = encode_request_frame(&request)?;
        assert_eq!(decode_request_frame(&frame), Ok(request));
    }
    let application = ControlledApplication {
        application: 7,
        name: "counter".into(),
        desired_running: true,
        state: ControlledApplicationState::Running,
        incarnation: Some(2),
        process: Some(100),
    };
    for result in [
        ControlSuccess::Applications(vec![application]),
        ControlSuccess::ApplicationInvoked {
            application: 7,
            outcome: lkjscript_core::ExecutionOutcome::Exited(0),
            output: b"frame".to_vec(),
        },
    ] {
        let response = ControlResponse {
            request_id: 11,
            result: Ok(result),
        };
        let frame = encode_response_frame(&response)?;
        assert_eq!(decode_response_frame(&frame), Ok(response));
    }
    Ok(())
}

#[test]
fn malformed_stale_and_wrong_digest_requests_fail_closed() -> Result<(), Box<dyn Error>> {
    let request = ControlRequest::current(1, [0; 32], ControlOperation::Status)?;
    let frame = encode_request_frame(&request)?;
    for length in 0..frame.len() {
        assert!(decode_request_frame(&frame[..length]).is_err());
    }
    let mut stale = request.clone();
    stale.identity.platform_revision -= 1;
    assert!(matches!(
        validate_request(&stale),
        Err(ControlFailure::StaleRevision { .. })
    ));
    let mut wrong = request;
    wrong.identity.contract_digest = lkjscript_contracts::ContractDigest::from_bytes([1; 32]);
    assert_eq!(
        validate_request(&wrong),
        Err(ControlFailure::ContractMismatch)
    );
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn unix_control_authenticates_peer_and_replays_one_mutation() -> Result<(), Box<dyn Error>> {
    use std::os::unix::net::UnixStream;
    use std::sync::{Arc, Mutex};

    let (left, right) = UnixStream::pair()?;
    let principal = lkjscript_host::local_peer_principal(&left)?;
    drop((left, right));
    let path = std::env::temp_dir().join(format!("lkjscript-control-{}.sock", std::process::id()));
    let mut server = UnixControlServer::bind(&path, principal.user)?;
    let count = Arc::new(Mutex::new(0_u32));
    let server_count = Arc::clone(&count);
    let thread = std::thread::spawn(move || -> Result<(), ControlError> {
        for _ in 0..2 {
            server.serve_one(|_| {
                let mut count = server_count.lock().map_err(|_| ControlFailure::Internal)?;
                *count += 1;
                Ok(ControlSuccess::ShutdownAccepted)
            })?;
        }
        Ok(())
    });
    let client = UnixControlClient::new(path);
    let request = ControlRequest::current(44, [9; 32], ControlOperation::Shutdown)?;
    assert_eq!(
        client.call(&request)?.result,
        Ok(ControlSuccess::ShutdownAccepted)
    );
    assert_eq!(
        client.call(&request)?.result,
        Ok(ControlSuccess::ShutdownAccepted)
    );
    let joined = thread
        .join()
        .map_err(|_| "control server thread panicked")?;
    joined?;
    assert_eq!(*count.lock().map_err(|_| "count lock poisoned")?, 1);
    Ok(())
}
