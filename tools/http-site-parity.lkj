//! Compare a fresh crawl of the live origin with a granted candidate endpoint.
//! Only GET and HEAD are sent. The manifest is a coverage check, never the
//! source of crawl links; required paths.txt supplies the 51 search probes.
mod http_client;
mod http_site_links;
mod http_site;
mod http_site_probes;
mod http_site_queries;
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
// A successful live/hardened handoff must include the same complete search
// coverage as the frozen oracle. Validate before either Connector is used.
fn query_coverage(paths: &[String]) -> Result<(), String> {
    http_site_queries::validate(paths)
}
fn parity(output: &mut Stdout, origin: &mut Connector, candidate: &mut Connector, root: &mut Directory) -> Result<(), String> {
    let archived = http_site::manifest_paths(root)?;
    let queries = http_site::extra_paths(root)?;
    query_coverage(&queries)?;
    let mut crawl = Crawl::new(true);
    for path in queries { crawl.add(path); }
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

fn published_query_paths() -> Vec<String> {
    let mut paths: Vec<String> = Vec::new();
    for line in include_str!("../examples/site-content/paths.txt").lines() {
        let line = std::str::trim_ascii(line);
        if line.is_empty() || line.starts_with("#") { continue; }
        paths.push(http_site_links::resolve("/", line).unwrap());
    }
    paths
}
#[test]
fn live_query_coverage_accepts_published_fixture() {
    assert!(query_coverage(&published_query_paths()).is_ok());
}
#[test]
fn live_query_coverage_rejects_missing_or_short_fixture() {
    let missing: Vec<String> = Vec::new();
    assert!(query_coverage(&missing).is_err());
    let mut short = published_query_paths();
    short.pop();
    assert!(query_coverage(&short).is_err());
}
#[test]
fn live_query_coverage_rejects_duplicates_and_non_search_targets() {
    let mut paths = published_query_paths();
    let last = paths.len() - 1usize;
    let duplicate = paths[0usize].clone();
    paths[last] = duplicate;
    assert!(query_coverage(&paths).is_err());
    paths[last] = "/docs/start?parity=1".to_string();
    assert!(query_coverage(&paths).is_err());
}
