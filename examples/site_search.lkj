//! Search the exact public Markdown snapshot named by the archived origin.
//! Metadata and page framing come from the downloaded homepage/search page.
//! Results prioritize title matches, then preserve the published document order.
use crate::site_archive::Archive;
use std::text;

struct Document {
    title: String,
    summary: String,
    source: String,
    card: String,
}
pub struct Search {
    before_value: String,
    after_value: String,
    empty: String,
    after_results: String,
    documents: Vec<Document>,
}
fn marker(value: &str, needle: &str, at: usize) -> Result<usize, String> {
    match value[at..].find(needle) {
        Some(offset) => Ok(at + offset),
        None => Err(format!("site search archive is missing {needle}")),
    }
}
fn query(raw: &str) -> Result<String, String> {
    let mut found = false;
    let mut value = String::new();
    for field in raw.split('&') {
        if field.is_empty() { continue; }
        let split = field.find('=').unwrap_or(field.len());
        let key = text::form_decode(&field[..split]);
        if key != "q" {
            return Err(format!("Failed to deserialize query string: {key}: unknown field `{key}`, expected `q`"));
        }
        if found {
            return Err("Failed to deserialize query string: duplicate field `q`".to_string());
        }
        found = true;
        if split < field.len() { value = text::form_decode(&field[split + 1usize..]); }
    }
    if value.len() > 128usize {
        return Err("Search must be at most 128 UTF-8 bytes without control characters.".to_string());
    }
    for character in value.as_str().chars() {
        if character.is_control() {
            return Err("Search must be at most 128 UTF-8 bytes without control characters.".to_string());
        }
    }
    Ok(text::trim(value.as_str()).to_string())
}
impl Search {
    pub fn open(archive: &mut Archive) -> Result<Search, String> {
        let template = archive.text("/search")?;
        let insertion = marker(&template, "value=\"\"", 0usize)? + "value=\"".len();
        let results = marker(&template, "</form>", insertion)? + "</form>".len();
        let after = marker(&template, "</section>", results)?;
        let before_value = template[..insertion].to_string();
        let after_value = template[insertion..results].to_string();
        let empty = template[results..after].to_string();
        let after_results = template[after..].to_string();
        let home = archive.text("/")?;
        let mut documents: Vec<Document> = Vec::new();
        let mut at = 0usize;
        let prefix = "<a class=\"card\" href=\"";
        while let Some(relative) = home[at..].find(prefix) {
            let path_start = at + relative + prefix.len();
            let path_end = marker(&home, "\"", path_start)?;
            let path = home[path_start..path_end].to_string();
            if !path.starts_with("/docs/") || archive.index(&path).is_none() {
                return Err(format!("site search card has no published document: {path}"));
            }
            let title_start = marker(&home, "<h3>", path_end)? + "<h3>".len();
            let title_end = marker(&home, "</h3>", title_start)?;
            let raw_title = &home[title_start..title_end];
            let arrow = raw_title.find("<span").unwrap_or(raw_title.len());
            let title = text::trim(&raw_title[..arrow]).to_string();
            let summary_start = marker(&home, "<p>", title_end)? + "<p>".len();
            let summary_end = marker(&home, "</p>", summary_start)?;
            let summary = home[summary_start..summary_end].to_string();
            at = marker(&home, "</a>", summary_end)? + "</a>".len();
            let name = format!("search-{}.md", documents.len());
            let source = archive.raw(&name)?;
            let card = format!("<a class=\"card\" href=\"{path}\"><h3>{title}</h3><p>{summary}</p></a>");
            documents.push(Document {
                title: text::lowercase_chars(&title),
                summary: text::lowercase_chars(&summary),
                source: text::lowercase_chars(&source),
                card,
            });
        }
        if documents.len() != 8usize {
            return Err("site search requires the eight published source documents".to_string());
        }
        Ok(Search { before_value, after_value, empty, after_results, documents })
    }
    /// User input is total: rejected queries become a 400 body, never a server error.
    pub fn render(&self, raw: &str) -> (u16, String) {
        let display = match query(raw) {
            Ok(value) => value,
            Err(problem) => return (400u16, problem),
        };
        let mut middle = String::new();
        if display.is_empty() {
            middle.push_str(&self.empty);
        } else {
            let needle = text::lowercase_chars(&display);
            let mut cards = String::new();
            let mut count = 0usize;
            // Scalar-wise lowercase is equivalent here to the origin's full
            // lowercase matching: these pinned sources contain no cased
            // non-ASCII letters (only ASCII, 猫 and punctuation/box drawing).
            for priority in 0usize..2usize {
                for document in &self.documents {
                    let title = document.title.contains(needle.as_str());
                    let matches = if priority == 0usize { title } else {
                        !title && (document.summary.contains(needle.as_str()) || document.source.contains(needle.as_str()))
                    };
                    if matches { cards.push_str(&document.card); count += 1usize; }
                }
            }
            if count == 0usize {
                middle.push_str("<p class=\"empty\">No matching documentation. Try a topic such as HTTP, ownership or deployment.</p>");
            } else {
                middle.push_str(&format!("<p>{count} matching documents</p>"));
                middle.push_str(&cards);
            }
        }
        let value = text::escape_html(&display);
        (200u16, format!("{}{value}{}{middle}{}", self.before_value, self.after_value, self.after_results))
    }
}
