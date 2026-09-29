#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::*;
use axum::{
    body::{Body, to_bytes},
    http::{Method, Request},
};
use tower::ServiceExt;

async fn request(router: Router, method: Method, uri: &str) -> Response {
    router
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("host", "untrusted.invalid")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}
async fn text(response: Response) -> String {
    String::from_utf8(
        to_bytes(response.into_body(), 2 * 1024 * 1024)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap()
}
fn headers(response: &Response) {
    assert!(
        response.headers()["content-security-policy"]
            .to_str()
            .unwrap()
            .contains("script-src 'none'")
    );
    assert_eq!(response.headers()["x-content-type-options"], "nosniff");
    assert_eq!(response.headers()["referrer-policy"], "no-referrer");
    assert_eq!(response.headers()["cache-control"], "no-cache");
}

#[tokio::test]
async fn all_published_documents_are_real_embedded_content() {
    let router = app();
    let home = request(router.clone(), Method::GET, "/").await;
    headers(&home);
    let home = text(home).await;
    assert!(home.contains("One executable."));
    assert!(!home.contains("untrusted.invalid"));
    assert!(!home.contains("<script"));
    let mut slugs = std::collections::BTreeSet::new();
    for document in catalog::DOCUMENTS {
        assert!(slugs.insert(document.slug));
        assert!(home.contains(&format!("href=\"/docs/{}\"", document.slug)));
        assert!(!document.markdown.is_empty());
        let response = request(
            router.clone(),
            Method::GET,
            &format!("/docs/{}", document.slug),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        headers(&response);
        assert!(
            response.headers()["content-type"]
                .to_str()
                .unwrap()
                .starts_with("text/html")
        );
        let body = text(response).await;
        assert!(body.contains("<article>"));
        assert!(body.contains("aria-current=\"page\""));
        assert!(body.contains(&format!("<title>{}", document.title)));
        assert!(!body.contains("<script"));
        assert!(!body.contains("<img"));
    }
}

#[tokio::test]
async fn unknown_paths_never_fall_through_to_workspace_files() {
    let router = app();
    for path in [
        "/.env",
        "/Cargo.toml",
        "/src/main.rs",
        "/docs/unknown",
        "/docs/%2e%2e%2fsecret",
        "/api/run",
        "/.git/config",
    ] {
        let response = request(router.clone(), Method::GET, path).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
        headers(&response);
        assert!(text(response).await.contains("Nothing published here."));
    }
}

#[tokio::test]
async fn search_finds_content_and_escapes_user_input() {
    let router = app();
    let response = request(router.clone(), Method::GET, "/search?q=HTTP").await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = text(response).await;
    assert!(body.contains("matching documents"));
    assert!(body.contains("/docs/http"));
    let response = request(router.clone(), Method::GET, "/search?q=%22%3E%3Cscript%3E").await;
    assert_eq!(response.status(), StatusCode::OK);
    headers(&response);
    let body = text(response).await;
    assert!(!body.contains("<script>"));
    assert!(body.contains("&quot;&gt;&lt;script&gt;"));
    assert!(
        text(request(router.clone(), Method::GET, "/search").await)
            .await
            .contains("Search the eight")
    );
    assert!(
        text(request(router, Method::GET, "/search?q=impossible-zqzyxv").await)
            .await
            .contains("No matching documentation")
    );
}

#[tokio::test]
async fn search_admission_counts_utf8_bytes_and_rejects_ambiguous_fields() {
    let router = app();
    for query in [
        format!("q={}", "a".repeat(129)),
        "q=one&q=two".to_owned(),
        "q=one&file=private".to_owned(),
        "q=%00".to_owned(),
        format!("q={}", "%E6%97%A5".repeat(43)),
    ] {
        let response = request(router.clone(), Method::GET, &format!("/search?{query}")).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{query}");
        headers(&response);
    }
    for query in ["a".repeat(128), "%E6%97%A5".repeat(42)] {
        assert_eq!(
            request(router.clone(), Method::GET, &format!("/search?q={query}"))
                .await
                .status(),
            StatusCode::OK
        );
    }
    let response = request(
        router,
        Method::GET,
        &format!("/search?q={}", "a".repeat(4096)),
    )
    .await;
    assert_eq!(response.status(), StatusCode::URI_TOO_LONG);
    headers(&response);
}

#[tokio::test]
async fn methods_headers_and_head_bodies_are_consistent() {
    let router = app();
    for path in [
        "/",
        "/docs/start",
        "/search?q=HTTP",
        "/healthz",
        "/assets/site.css",
        "/missing",
    ] {
        let response = request(router.clone(), Method::HEAD, path).await;
        headers(&response);
        assert_eq!(
            response.status(),
            if path == "/missing" {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::OK
            }
        );
        assert!(text(response).await.is_empty());
    }
    for method in [Method::POST, Method::PUT, Method::DELETE, Method::PATCH] {
        let response = request(router.clone(), method, "/").await;
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
        headers(&response);
    }
    let response = request(router.clone(), Method::GET, "/assets/site.css").await;
    assert_eq!(
        response.headers()["content-type"],
        "text/css; charset=utf-8"
    );
    assert!(text(response).await.contains("@media(max-width:700px)"));
    let health = text(request(router, Method::GET, "/healthz").await).await;
    assert_eq!(
        health,
        format!(
            "ok\nsite={}\nsource={}\n",
            env!("CARGO_PKG_VERSION"),
            revision()
        )
    );
}
