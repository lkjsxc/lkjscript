//! Refresh the frozen origin oracle using explicit read-only GET/HEAD grants.
//! It refuses an existing fixture; see tests/fixtures/site/README.md.
mod http_client;
mod http_site_links;
mod http_site;
mod http_site_probes;
mod http_site_oracle;
use std::json::{self, Value};
use http_client::{io, require};
use http_site_oracle as oracle;

fn capture(output: &mut Stdout, origin: &mut Connector, root: &mut Directory) -> Result<(), String> {
    match root.open(oracle::PATH) {
        Ok(file) => { drop(file); return Err("origin oracle already exists; an intentional refresh must remove it first".to_string()); }
        Err(2i64) => {},
        Err(errno) => return Err(format!("inspect origin oracle: errno {errno}")),
    }
    let requests = oracle::requests(root)?;
    let mut records: Vec<Value> = Vec::new();
    let mut body_bytes = 0usize;
    for request in &requests {
        let extra = request.headers()?;
        let page = match http_site::fetch(origin, &request.method, &request.path, &extra) {
            Ok(page) => page,
            Err(problem) => return Err(format!("record {} {}: {problem}", request.method, request.path)),
        };
        require(page.headers.len() == 6usize, &format!("{}: expected all six stable origin headers", request.path))?;
        body_bytes = body_bytes.checked_add(page.body.len()).ok_or("oracle byte count overflow".to_string())?;
        records.push(oracle::recorded(request, &page));
    }
    let fixture = Value::Object(vec![
        ("version".to_string(), Value::from_usize(1usize)),
        ("origin".to_string(), Value::String("http://127.0.0.1:8798".to_string())),
        ("records".to_string(), Value::Array(records)),
    ]);
    let text = match json::stringify(&fixture) { Ok(text) => text, Err(problem) => return Err(format!("encode origin oracle: {problem}")) };
    let mut file = io(root.create_new(oracle::PATH), "create origin oracle")?;
    let saved = io(file.write_all(text.as_bytes()), "write origin oracle").and_then(|_| io(file.sync(), "sync origin oracle"));
    drop(file);
    if let Err(problem) = saved {
        if let Err(errno) = root.remove(oracle::PATH) { return Err(format!("{problem}; remove incomplete oracle: errno {errno}")); }
        return Err(problem);
    }
    output.write(format!("ok site oracle recorded: 51 search queries, 13 shared protocol probes, {body_bytes} complete response body bytes hashed; {} fixture bytes; {}\n",
        text.len(), oracle::PATH).as_bytes());
    Ok(())
}
pub fn main(mut output: Stdout, mut origin: Connector, mut root: Directory) {
    if let Err(problem) = capture(&mut output, &mut origin, &mut root) {
        output.write(format!("site oracle record: {problem}\n").as_bytes());
        panic!("{problem}");
    }
}
