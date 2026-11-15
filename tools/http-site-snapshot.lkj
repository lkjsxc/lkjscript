//! Capture the documentation origin using only GET and explicit grants.
//! Existing search source/provenance files are preserved; snapshot files are
//! create-exclusive and manifest publication follows complete body writes.
mod http_client;
mod http_site_links;
mod http_site;
use std::json::{self, Value};
use http_client::{io, require};
use http_site::{Crawl, Page};

fn save(root: &mut Directory, created: &mut Vec<String>, name: &str, bytes: &[u8]) -> Result<(), String> {
    let mut file = io(root.create_new(name), &format!("create snapshot {name}"))?;
    created.push(name.to_string());
    io(file.write_all(bytes), &format!("write snapshot {name}"))?;
    io(file.sync(), &format!("sync snapshot {name}"))
}
fn record(root: &mut Directory, created: &mut Vec<String>, page: &Page, index: usize) -> Result<Value, String> {
    let filename = format!("{index}.body");
    save(root, created, &filename, &page.body)?;
    Ok(http_site::metadata(page, &filename))
}
fn capture(output: &mut Stdout, origin: &mut Connector, root: &mut Directory, created: &mut Vec<String>) -> Result<(), String> {
    match root.open("manifest.json") {
        Ok(file) => { drop(file); return Err("snapshot manifest.json already exists".to_string()); }
        Err(2i64) => {},
        Err(errno) => return Err(format!("inspect snapshot destination: errno {errno}")),
    }
    let mut crawl = Crawl::new(false);
    let mut pages: Vec<Value> = Vec::new();
    while let Some(path) = crawl.next() {
        let page = http_site::get(origin, &path)?;
        crawl.discover(&page)?;
        let entry = record(root, created, &page, pages.len())?;
        output.write(format!("snapshot {}: status {}, {} body bytes, {} stable headers\n",
            page.path, page.status, page.body.len(), page.headers.len()).as_bytes());
        pages.push(entry);
    }
    let missing = http_site::get(origin, http_site::MISSING)?;
    require(missing.status == 404u16, "snapshot missing-path probe did not return 404")?;
    let not_found = record(root, created, &missing, pages.len())?;
    let count = pages.len();
    let manifest = Value::Object(vec![
        ("version".to_string(), Value::from_usize(1usize)),
        ("pages".to_string(), Value::Array(pages)),
        ("not_found".to_string(), not_found),
    ]);
    let text = match json::stringify(&manifest) { Ok(text) => text, Err(error) => return Err(format!("snapshot manifest {error}")) };
    save(root, created, ".manifest-next", text.as_bytes())?;
    io(root.sync(), "sync complete snapshot files")?;
    io(root.rename(".manifest-next", "manifest.json"), "publish snapshot manifest")?;
    let last = created.len() - 1usize;
    created[last] = "manifest.json".to_string();
    io(root.sync(), "sync snapshot manifest directory")?;
    output.write(format!("ok site snapshot: {count} canonical published paths plus missing response; byte-exact bodies; manifest.json\n").as_bytes());
    Ok(())
}
pub fn main(mut output: Stdout, mut origin: Connector, mut root: Directory) {
    let mut created: Vec<String> = Vec::new();
    if let Err(problem) = capture(&mut output, &mut origin, &mut root, &mut created) {
        output.write(format!("site snapshot: {problem}\n").as_bytes());
        for name in &created {
            if let Err(errno) = root.remove(name) {
                output.write(format!("snapshot cleanup failed for {name}: errno {errno}\n").as_bytes());
            }
        }
        panic!("{problem}");
    }
}
