use crate::{
    catalog::{DOCUMENTS, revision, source_url},
    markdown::escape,
};

pub(crate) fn page(title: &str, body: &str) -> String {
    format!(
        r##"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta name="description" content="lkjscript: a meaning-oriented language and application platform. Explore the language, its implementation and its current limits."><title>{} · lkjscript</title><link rel="stylesheet" href="/assets/site.css"></head><body><a class="skip" href="#content">Skip to content</a><header><a class="brand" href="/">lkj<span>script</span><span class="badge">EXPERIMENTAL</span></a><nav aria-label="Primary"><a href="/docs/start">Learn</a><a href="/docs/direction">Design</a><a href="/docs/status">Status</a><a href="https://github.com/lkjsxc/lkjscript">Source ↗</a></nav></header><main id="content">{body}</main><footer><span>Built in Rust. No browser JavaScript.</span><span>Documentation snapshot · <code>{}</code></span><a href="/docs/security">Trust boundaries</a></footer></body></html>"##,
        escape(title),
        escape(revision())
    )
}

pub(crate) fn search_form(query: &str) -> String {
    format!(
        r##"<form class="search" action="/search" method="get"><label class="sr-only" for="search-input">Search documentation</label><input id="search-input" name="q" type="search" maxlength="128" value="{}" placeholder="Search the documentation" autocomplete="off"><button type="submit">Search</button></form>"##,
        escape(query)
    )
}

pub(crate) fn home() -> String {
    let cards = DOCUMENTS.iter().map(|document| format!(r##"<a class="card" href="/docs/{}"><h3>{} <span aria-hidden="true">↗</span></h3><p>{}</p></a>"##, document.slug, escape(document.title), escape(document.summary))).collect::<String>();
    page(
        "A language built around meaning",
        &format!(
            r##"<section class="hero"><div><p class="eyebrow">LANGUAGE / RUNTIME / TOOLS</p><h1>One executable.<br><em>Typed meaning.</em></h1><p class="lead">A language and application platform for agents. Build, inspect and change programs through explicit, checkable meaning.</p><div class="actions"><a class="button" href="/docs/start">Create your first program <span aria-hidden="true">→</span></a><a href="/docs/direction">Explore the design ↗</a></div><p class="note">An evolving project, not a stability or hostile-code sandbox promise.</p></div><aside class="terminal" aria-label="Example development workflow"><div class="terminal-title">THE DEVELOPMENT LOOP</div><pre><code><span>01 / CREATE</span>
lkjscript new hello --template command

<span>02 / CHECK</span>
lkjscript --project hello check

<span>03 / RUN</span>
lkjscript --project hello run main</code></pre><div class="terminal-foot">Meaning is the authority.<br>Text is a proposal.</div></aside></section><section class="principles" aria-label="Core principles"><div><span class="number">01</span><h2>One semantic authority</h2><p>The accepted typed graph owns program meaning. Names and projections remain useful without becoming competing sources of truth.</p></div><div><span class="number">02</span><h2>Explicit contracts</h2><p>Types, ownership, effects and exact dependencies make changes inspectable and execution authority visible.</p></div><div><span class="number">03</span><h2>A Rust implementation</h2><p>The current direction selects Rust for the implementation. Native examples and remaining interpreted tooling are distinguished in the status notes.</p></div></section><section class="library"><div class="section-heading"><div><p class="eyebrow">EXPLORE THE PROJECT</p><h2>From first program<br>to the underlying ideas.</h2></div>{}</div><div class="cards">{cards}</div></section>"##,
            search_form("")
        ),
    )
}

pub(crate) fn document(document: &crate::catalog::Document, content: &str) -> String {
    let navigation = DOCUMENTS
        .iter()
        .map(|item| {
            format!(
                r##"<a href="/docs/{}"{}>{}</a>"##,
                item.slug,
                if item.slug == document.slug {
                    " aria-current=\"page\""
                } else {
                    ""
                },
                escape(item.title)
            )
        })
        .collect::<String>();
    page(
        document.title,
        &format!(
            r##"<div class="docs"><aside class="sidebar"><p class="eyebrow">DOCUMENTATION</p>{navigation}<a class="source-link" href="{}">View this source ↗</a></aside><div class="document"><div class="document-meta"><span>REPOSITORY SNAPSHOT</span><a href="/search">Search ↗</a></div><article>{content}</article></div></div>"##,
            escape(&source_url(document.path))
        ),
    )
}
