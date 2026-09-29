use crate::catalog::{DOCUMENTS, source_url};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd, html};
use std::collections::BTreeSet;

pub(crate) fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn link(path: &str, destination: &str) -> String {
    if destination.starts_with('#')
        || destination.starts_with("https://")
        || destination.starts_with("http://")
    {
        return destination.to_owned();
    }
    if destination.starts_with('/')
        || destination.contains(':')
        || destination.contains('\\')
        || destination.chars().any(char::is_control)
    {
        return "#".to_owned();
    }
    let (relative, fragment) = destination.split_once('#').unwrap_or((destination, ""));
    let mut parts = path.split('/').collect::<Vec<_>>();
    parts.pop();
    for part in relative.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return "#".to_owned();
                }
            }
            value => parts.push(value),
        }
    }
    let normalized = parts.join("/");
    let mut url = match DOCUMENTS
        .iter()
        .find(|document| document.path == normalized)
    {
        Some(document) => format!("/docs/{}", document.slug),
        None => source_url(&normalized),
    };
    if !fragment.is_empty() {
        url.push('#');
        url.push_str(fragment);
    }
    url
}

pub(crate) fn render(path: &str, markdown: &str) -> String {
    let mut events = Vec::new();
    let mut heading: Option<(pulldown_cmark::HeadingLevel, Vec<Event<'_>>, String)> = None;
    let mut identifiers = BTreeSet::new();
    for event in Parser::new_ext(
        markdown,
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH,
    ) {
        let event = match event {
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                title,
                id,
            }) => Event::Start(Tag::Link {
                link_type,
                dest_url: link(path, &dest_url).into(),
                title,
                id,
            }),
            // Documentation cannot add executable HTML or browser network fetches.
            Event::Html(text) | Event::InlineHtml(text) => Event::Text(text),
            Event::Start(Tag::Image { .. }) | Event::End(TagEnd::Image) => continue,
            value => value,
        };
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                heading = Some((level, Vec::new(), String::new()))
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((level, content, text)) = heading.take() {
                    let mut id = String::new();
                    for character in text.chars().flat_map(char::to_lowercase) {
                        if character.is_alphanumeric() || matches!(character, '-' | '_') {
                            id.push(character);
                        } else if character.is_whitespace() {
                            id.push('-');
                        }
                    }
                    if id.is_empty() {
                        id.push_str("section");
                    }
                    let base = id.clone();
                    let mut suffix = 0;
                    while !identifiers.insert(id.clone()) {
                        suffix += 1;
                        id = format!("{base}-{suffix}");
                    }
                    events.push(Event::Html(
                        format!("<{level} id=\"{}\">", escape(&id)).into(),
                    ));
                    events.extend(content);
                    events.push(Event::Html(format!("</{level}>\n").into()));
                }
            }
            value => {
                if let Some((_, content, text)) = &mut heading {
                    if let Event::Text(value) | Event::Code(value) = &value {
                        text.push_str(value);
                    }
                    content.push(value);
                } else {
                    events.push(value);
                }
            }
        }
    }
    let mut output = String::new();
    html::push_html(&mut output, events.into_iter());
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn relative_links_resolve_without_filesystem_access() {
        assert_eq!(
            link("docs/guides/native-web.md", "native-command.md#hello"),
            "/docs/start#hello"
        );
        assert_eq!(
            link("docs/guides/native-web.md", "../../docs/status.md"),
            "/docs/status"
        );
        assert_eq!(link("docs/a.md", "../../secret"), "#");
        for value in [
            "javascript:alert(1)",
            "data:text/html,x",
            "//evil.test",
            "file:///etc/passwd",
            "../\\secret",
        ] {
            assert_eq!(link("docs/a.md", value), "#");
        }
    }
    #[test]
    fn generated_suffixes_cannot_collide_with_literal_heading_names() {
        let result = render("docs/a.md", "# A\n\n# A\n\n# A-1\n\n# A\n");
        for id in ["a", "a-1", "a-1-1", "a-2"] {
            assert_eq!(result.matches(&format!("id=\"{id}\"")).count(), 1);
        }
    }

    #[test]
    fn content_is_escaped_and_heading_anchors_are_stable() {
        let result = render(
            "docs/a.md",
            "# Hello `world`\n\n# Hello world\n\n<script>alert(1)</script>\n\n![tracking](https://evil.test/pixel)\n\n[bad](javascript:evil)\n",
        );
        assert!(result.contains("id=\"hello-world\""));
        assert!(result.contains("id=\"hello-world-1\""));
        assert!(result.contains("&lt;script&gt;"));
        assert!(!result.contains("<script>"));
        assert!(!result.contains("<img"));
        assert!(!result.contains("href=\"javascript:"));
    }
}
