use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

use lkjscript_host::local_peer_principal;

use super::{
    decode_request_frame, decode_response_frame, encode_request_frame, encode_response_frame,
    validate_request, ControlError, ControlFailure, ControlOperation, ControlRequest,
    ControlResponse, MAX_CONTROL_FRAME_BYTES, MAX_REPLAY_ENTRIES,
};

pub struct UnixControlServer {
    listener: UnixListener,
    path: PathBuf,
    allowed_user: u32,
    replay: BTreeMap<[u8; 32], (ControlOperation, ControlResponse)>,
}

impl UnixControlServer {
    pub fn bind(path: impl Into<PathBuf>, allowed_user: u32) -> Result<Self, ControlError> {
        let path = path.into();
        if path.exists() {
            fs::remove_file(&path).map_err(io_error)?;
        }
        let listener = UnixListener::bind(&path).map_err(io_error)?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).map_err(io_error)?;
        Ok(Self {
            listener,
            path,
            allowed_user,
            replay: BTreeMap::new(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn serve_one(
        &mut self,
        mut handler: impl FnMut(
            &ControlRequest,
            lkjscript_host::LocalPrincipal,
        ) -> Result<super::ControlSuccess, ControlFailure>,
    ) -> Result<ControlOperation, ControlError> {
        let (mut stream, _) = self.listener.accept().map_err(io_error)?;
        configure(&stream)?;
        let principal = local_peer_principal(&stream)?;
        if principal.user != self.allowed_user {
            return Err(ControlError::Unauthorized(principal));
        }
        let request = decode_request_frame(&read_frame(&mut stream)?)?;
        let response = if let Err(failure) = validate_request(&request) {
            ControlResponse {
                request_id: request.request_id,
                result: Err(failure),
            }
        } else if request.operation.modifies() {
            match self.replay.get(&request.idempotency_id) {
                Some((operation, response)) if *operation == request.operation => response.clone(),
                Some(_) => ControlResponse {
                    request_id: request.request_id,
                    result: Err(ControlFailure::ReplayConflict),
                },
                None => {
                    let response = ControlResponse {
                        request_id: request.request_id,
                        result: handler(&request, principal),
                    };
                    if self.replay.len() == MAX_REPLAY_ENTRIES {
                        return Err(ControlError::ReplayConflict);
                    }
                    self.replay.insert(
                        request.idempotency_id,
                        (request.operation.clone(), response.clone()),
                    );
                    response
                }
            }
        } else {
            ControlResponse {
                request_id: request.request_id,
                result: handler(&request, principal),
            }
        };
        write_frame(&mut stream, &encode_response_frame(&response)?)?;
        Ok(request.operation.clone())
    }
}

impl Drop for UnixControlServer {
    fn drop(&mut self) {
        let _ignored = fs::remove_file(&self.path);
    }
}

pub struct UnixControlClient {
    path: PathBuf,
}

impl UnixControlClient {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn call(&self, request: &ControlRequest) -> Result<ControlResponse, ControlError> {
        let mut stream = UnixStream::connect(&self.path).map_err(io_error)?;
        configure(&stream)?;
        write_frame(&mut stream, &encode_request_frame(request)?)?;
        let response = decode_response_frame(&read_frame(&mut stream)?)?;
        if response.request_id != request.request_id {
            return Err(ControlError::Malformed("response request identity"));
        }
        Ok(response)
    }
}

fn configure(stream: &UnixStream) -> Result<(), ControlError> {
    let timeout = Some(Duration::from_secs(5));
    stream.set_read_timeout(timeout).map_err(io_error)?;
    stream.set_write_timeout(timeout).map_err(io_error)
}

fn read_frame(stream: &mut UnixStream) -> Result<Vec<u8>, ControlError> {
    let mut prefix = [0_u8; 4];
    stream.read_exact(&mut prefix).map_err(io_error)?;
    let length = u32::from_le_bytes(prefix) as usize;
    if length > MAX_CONTROL_FRAME_BYTES {
        return Err(ControlError::Oversized);
    }
    let mut frame = Vec::with_capacity(length + 4);
    frame.extend_from_slice(&prefix);
    frame.resize(length + 4, 0);
    stream.read_exact(&mut frame[4..]).map_err(io_error)?;
    Ok(frame)
}

fn write_frame(stream: &mut UnixStream, frame: &[u8]) -> Result<(), ControlError> {
    stream.write_all(frame).map_err(io_error)?;
    stream.flush().map_err(io_error)
}

fn io_error(error: std::io::Error) -> ControlError {
    ControlError::Io(error.to_string())
}
