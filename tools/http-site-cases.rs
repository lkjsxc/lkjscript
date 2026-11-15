//! Verify a candidate against archived bytes and frozen live-origin observations.
//! This executable has no origin grant and never reads the live service.
mod http_client;
mod http_site_links;
mod http_site;
mod http_site_probes;
mod http_site_oracle;
use std::json::Value;
use http_client::require;
use http_site_oracle as oracle;

fn archive_case(target: &mut Connector, root: &mut Directory, value: &Value) -> Result<usize, String> {
    let expected = oracle::archived(root, value)?;
    let actual = http_site::get(target, &expected.path)?;
    match http_site::compare(&expected, &actual) {
        Ok(()) => Ok(actual.body.len()),
        Err(problem) => Err(format!("archived GET {}: {problem}", expected.path)),
    }
}
fn check(output: &mut Stdout, target: &mut Connector, root: &mut Directory) -> Result<(), String> {
    let manifest = oracle::read(root, oracle::MANIFEST)?;
    oracle::version(&manifest)?;
    let pages = manifest.get("pages").and_then(|field| field.as_array()).ok_or("site manifest pages must be an array".to_string())?;
    require(pages.len() == 12usize, "site archive must contain the twelve published paths")?;
    let mut body_bytes = 0usize;
    for page in pages {
        let bytes = archive_case(target, root, page)?;
        body_bytes = body_bytes.checked_add(bytes).ok_or("site case byte count overflow".to_string())?;
    }
    let missing = manifest.get("not_found").ok_or("site manifest needs its missing response".to_string())?;
    let missing_bytes = archive_case(target, root, missing)?;
    body_bytes = body_bytes.checked_add(missing_bytes).ok_or("site case byte count overflow".to_string())?;
    let requests = oracle::requests(root)?;
    let fixture = oracle::read(root, oracle::PATH)?;
    oracle::version(&fixture)?;
    require(oracle::string(&fixture, "origin")? == "http://127.0.0.1:8798", "site oracle names another origin")?;
    let records = fixture.get("records").and_then(|field| field.as_array()).ok_or("site oracle records must be an array".to_string())?;
    require(records.len() == requests.len(), "site oracle count differs from the recorded request list")?;
    for index in 0usize..requests.len() {
        let request = &requests[index];
        let extra = request.headers()?;
        let actual = match http_site::fetch(target, &request.method, &request.path, &extra) {
            Ok(page) => page,
            Err(problem) => return Err(format!("oracle {} {}: {problem}", request.method, request.path)),
        };
        if let Err(problem) = oracle::verify_record(&records[index], request, &actual) {
            return Err(format!("oracle {} {} ({}): {problem}", request.method, request.path, request.name));
        }
        body_bytes = body_bytes.checked_add(actual.body.len()).ok_or("site case byte count overflow".to_string())?;
    }
    output.write(format!("ok site offline cases: 12 published paths plus archived 404 checked byte-for-byte, 51 search queries and 13 protocol probes checked against frozen origin status/Content-Length/stable headers/full-body SHA256, {body_bytes} response body bytes; no live-origin connection\n").as_bytes());
    Ok(())
}
pub fn main(mut output: Stdout, mut target: Connector, mut root: Directory) {
    if let Err(problem) = check(&mut output, &mut target, &mut root) {
        output.write(format!("site offline cases: {problem}\n").as_bytes());
        panic!("{problem}");
    }
}
