//! The same request descriptions drive live parity and the frozen CI oracle.
//! Header descriptors avoid embedding the 65,537-byte test value in JSON.
pub struct Probe {
    pub name: String,
    pub method: String,
    pub path: String,
    pub extra_headers: String,
}
impl Probe {
    pub fn new(name: &str, method: &str, path: &str, extra_headers: &str) -> Probe {
        Probe { name: name.to_string(), method: method.to_string(),
            path: path.to_string(), extra_headers: extra_headers.to_string() }
    }
    pub fn headers(&self) -> Result<String, String> {
        match self.extra_headers.as_str() {
            "none" => Ok(String::new()),
            "large_header_65537" => Ok(format!("X-Large: {}\r\n", "x".repeat(65_537usize))),
            "range_0_15" => Ok("Range: bytes=0-15\r\n".to_string()),
            "conditional_future" => Ok("If-None-Match: *\r\nIf-Modified-Since: Fri, 01 Jan 2100 00:00:00 GMT\r\n".to_string()),
            _ => Err(format!("unknown site test header descriptor: {}", self.extra_headers)),
        }
    }
}
pub fn protocol() -> Vec<Probe> {
    // Keep traversal targets raw; URL normalization would change these probes.
    // HEAD's Content-Length is compared, including the origin's observed zero.
    vec![
        Probe::new("missing", "GET", crate::http_site::MISSING, "none"),
        Probe::new("literal traversal", "GET", "/../docs/start", "none"),
        Probe::new("encoded traversal", "GET", "/%2e%2e/docs/start", "none"),
        Probe::new("65537-byte header value", "GET", "/", "large_header_65537"),
        Probe::new("HEAD framing", "HEAD", "/", "none"),
        Probe::new("range", "GET", "/", "range_0_15"),
        Probe::new("conditional", "GET", "/", "conditional_future"),
        Probe::new("encoded path", "GET", "/docs/%73tart", "none"),
        Probe::new("trailing slash", "GET", "/docs/start/", "none"),
        Probe::new("non-search query", "GET", "/docs/start?parity=1", "none"),
        Probe::new("encoded search route", "GET", "/%73earch", "none"),
        Probe::new("encoded asset route", "GET", "/assets/%73ite.css", "none"),
        Probe::new("encoded route separator", "GET", "/docs%2fstart", "none"),
    ]
}
