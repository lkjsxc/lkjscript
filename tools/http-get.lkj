//! Send one raw HTTP request from stdin to the exact granted endpoint.
//! Writes the framed response to stdout, then closes its owned connection.
mod http_client;
use http_client::{Connection, io, require};
fn exchange(output: &mut Stdout, input: &mut Stdin, target: &mut Connector) -> Result<(), String> {
    let request = io(input.read_to_end(), "read request")?;
    require(!request.is_empty(), "empty request")?;
    let head = request.len() >= 5usize && &request[..5] == b"HEAD ";
    let mut connection = Connection::new(io(target.connect(), "connect")?);
    connection.send(&request)?;
    let response = connection.header(head)?;
    output.write(&response.header);
    if !head && response.status >= 200u16 && response.status != 204u16 && response.status != 304u16 {
        let body = connection.body(response.length)?;
        output.write(&body);
    }
    if response.close { connection.eof()?; }
    Ok(())
}
pub fn main(mut output: Stdout, mut input: Stdin, mut target: Connector) {
    if let Err(problem) = exchange(&mut output, &mut input, &mut target) {
        panic!("{problem}");
    }
}
