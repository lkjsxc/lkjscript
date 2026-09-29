//! A read-only documentation origin. There is no compiler, filesystem-serving,
//! authentication database, user content, subprocess or outbound HTTP client.
mod assets;
mod catalog;
mod markdown;
mod view;

use axum::{
    Router,
    extract::{Path, Query, Request, State},
    http::{HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use serde::Deserialize;
use std::sync::Arc;

pub fn revision() -> &'static str {
    catalog::revision()
}

struct Page {
    slug: &'static str,
    title: &'static str,
    summary: &'static str,
    searchable: String,
    html: String,
}
struct Site {
    home: String,
    pages: Vec<Page>,
}

pub fn app() -> Router {
    let pages = catalog::DOCUMENTS
        .iter()
        .map(|document| Page {
            slug: document.slug,
            title: document.title,
            summary: document.summary,
            searchable: format!(
                "{}\n{}\n{}",
                document.title, document.summary, document.markdown
            )
            .to_lowercase(),
            html: view::document(
                document,
                &markdown::render(document.path, document.markdown),
            ),
        })
        .collect();
    Router::new()
        .route("/", get(home))
        .route("/docs/{slug}", get(document))
        .route("/search", get(search))
        .route("/healthz", get(health))
        .route("/assets/site.css", get(stylesheet))
        .route(
            "/robots.txt",
            get(|| async { "User-agent: *\nAllow: /\nDisallow: /search\n" }),
        )
        .fallback(not_found)
        .layer(middleware::from_fn(boundary))
        .with_state(Arc::new(Site {
            home: view::home(),
            pages,
        }))
}

async fn home(State(site): State<Arc<Site>>) -> Html<String> {
    Html(site.home.clone())
}
async fn document(State(site): State<Arc<Site>>, Path(slug): Path<String>) -> Response {
    match site.pages.iter().find(|page| page.slug == slug) {
        Some(page) => Html(page.html.clone()).into_response(),
        None => not_found().await,
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Search {
    #[serde(default)]
    q: String,
}
async fn search(State(site): State<Arc<Site>>, Query(input): Query<Search>) -> Response {
    if input.q.len() > 128 || input.q.chars().any(char::is_control) {
        return (
            StatusCode::BAD_REQUEST,
            "Search must be at most 128 UTF-8 bytes without control characters.",
        )
            .into_response();
    }
    let query = input.q.trim();
    let needle = query.to_lowercase();
    let mut results = site
        .pages
        .iter()
        .filter(|page| !needle.is_empty() && page.searchable.contains(&needle))
        .collect::<Vec<_>>();
    results.sort_by_key(|page| !page.title.to_lowercase().contains(&needle));
    let mut body = format!(
        "<section class=\"results\"><p class=\"eyebrow\">DOCUMENTATION SEARCH</p><h1>Find your next step.</h1>{}",
        view::search_form(query)
    );
    if needle.is_empty() {
        body.push_str("<p class=\"empty\">Search the eight published guides and project notes. This does not search private workspace files.</p>");
    } else if results.is_empty() {
        body.push_str("<p class=\"empty\">No matching documentation. Try a topic such as HTTP, ownership or deployment.</p>");
    } else {
        body.push_str(&format!("<p>{} matching documents</p>", results.len()));
        for page in results {
            body.push_str(&format!(
                "<a class=\"card\" href=\"/docs/{}\"><h3>{}</h3><p>{}</p></a>",
                page.slug,
                markdown::escape(page.title),
                markdown::escape(page.summary)
            ));
        }
    }
    body.push_str("</section>");
    Html(view::page("Search", &body)).into_response()
}
async fn health() -> impl IntoResponse {
    (
        [("content-type", "text/plain; charset=utf-8")],
        format!(
            "ok\nsite={}\nsource={}\n",
            env!("CARGO_PKG_VERSION"),
            revision()
        ),
    )
}
async fn stylesheet() -> impl IntoResponse {
    ([("content-type", "text/css; charset=utf-8")], assets::CSS)
}
async fn not_found() -> Response {
    (StatusCode::NOT_FOUND, Html(view::page("Not found", "<section class=\"results\"><p class=\"eyebrow\">404 / NOT FOUND</p><h1>Nothing published here.</h1><p>This server exposes only its embedded documentation.</p><a href=\"/\">Return to the project →</a></section>"))).into_response()
}
async fn boundary(request: Request, next: Next) -> Response {
    let head = request.method() == axum::http::Method::HEAD;
    let mut response = if request.uri().to_string().len() > 4096 {
        (StatusCode::URI_TOO_LONG, "Request URI is too long.").into_response()
    } else {
        next.run(request).await
    };
    if head {
        *response.body_mut() = axum::body::Body::empty();
    }
    let headers = response.headers_mut();
    headers.insert("content-security-policy", HeaderValue::from_static("default-src 'none'; style-src 'self'; img-src 'none'; script-src 'none'; connect-src 'none'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'; object-src 'none'"));
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    headers.insert(
        "permissions-policy",
        HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
    );
    headers.insert("cache-control", HeaderValue::from_static("no-cache"));
    response
}

#[cfg(test)]
mod tests;
