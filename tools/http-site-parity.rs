//! Compare a fresh crawl of the live origin with a granted candidate endpoint.
//! Only GET and HEAD are sent. The manifest is a coverage check, never the
//! source of crawl links; optional paths.txt adds query/search observations.
mod http_client;
mod http_site_links;
mod http_site;
mod http_site_probes;
use http_client::require;
use http_site::{Crawl, Page};

fn observe(target: &mut Connector, endpoint: &str, method: &str, path: &str, extra: &str) -> Result<Page, String> {
    match http_site::fetch(target, method, path, extra) {
        Ok(page) => Ok(page),
        Err(problem) => Err(format!("{endpoint} {method} {path}: {problem}")),
    }
}
fn equal(origin: &Page, candidate: &Page, label: &str) -> Result<(), String> {
    match http_site::compare(origin, candidate) {
        Ok(()) => Ok(()),
        Err(problem) => Err(format!("{label}: {problem}")),
    }
}
fn probe(output: &mut Stdout, origin: &mut Connector, candidate: &mut Connector,
    label: &str, method: &str, path: &str, extra: &str) -> Result<(), String> {
    let expected = observe(origin, "origin", method, path, extra)?;
    let actual = observe(candidate, "candidate", method, path, extra)?;
    equal(&expected, &actual, label)?;
    output.write(format!("ok probe {label}: {method} {path}, status {}, {} body bytes\n",
        expected.status, expected.body.len()).as_bytes());
    Ok(())
}
fn parity(output: &mut Stdout, origin: &mut Connector, candidate: &mut Connector, root: &mut Directory) -> Result<(), String> {
    let archived = http_site::manifest_paths(root)?;
    let mut crawl = Crawl::new(true);
    for path in http_site::extra_paths(root)? { crawl.add(path); }
    let mut walked = 0usize;
    let mut body_bytes = 0usize;
    while let Some(path) = crawl.next() {
        let expected = observe(origin, "origin", "GET", &path, "")?;
        // Discover from this live response before reading the candidate. A
        // candidate cannot hide a missing route by omitting its incoming link.
        crawl.discover(&expected)?;
        let actual = observe(candidate, "candidate", "GET", &path, "")?;
        equal(&expected, &actual, &format!("GET {path}"))?;
        walked = walked.checked_add(1usize).ok_or("walk count overflow".to_string())?;
        body_bytes = body_bytes.checked_add(expected.body.len()).ok_or("body byte count overflow".to_string())?;
        output.write(format!("ok path {path}: status {}, {} body bytes, {} stable headers\n",
            expected.status, expected.body.len(), expected.headers.len()).as_bytes());
    }
    for path in &archived {
        require(crawl.contains(path), &format!("archived path is absent from the independent live crawl: {path}"))?;
    }
    let probes = http_site_probes::protocol();
    for request in &probes {
        let extra = request.headers()?;
        probe(output, origin, candidate, &request.name, &request.method, &request.path, &extra)?;
    }
    output.write(format!("ok site parity: {walked} independently crawled/requested paths, {} archived paths covered, 13 protocol probes, {body_bytes} walked body bytes; exact status, Content-Length, stable header multiset and body equality; Date and Connection excluded\n",
        archived.len()).as_bytes());
    Ok(())
}
pub fn main(mut output: Stdout, mut origin: Connector, mut candidate: Connector, mut root: Directory) {
    if let Err(problem) = parity(&mut output, &mut origin, &mut candidate, &mut root) {
        output.write(format!("site parity: {problem}\n").as_bytes());
        panic!("{problem}");
    }
}
