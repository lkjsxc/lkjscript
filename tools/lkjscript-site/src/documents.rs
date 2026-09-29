// One Rust catalog owns publication metadata and changed-check input selection.
// Each consumer supplies `documents!`; the checker does not link the HTTP server.
documents! {
    (
        "start",
        "docs/guides/native-command.md",
        "Your first program",
        "Create, inspect, change and run a typed command application."
    ),
    (
        "web",
        "docs/guides/native-web.md",
        "Build for the web",
        "An editable web application, from one installed executable."
    ),
    (
        "http",
        "docs/guides/native-http.md",
        "HTTP applications",
        "Compose routes, requests and standalone service deployments."
    ),
    (
        "forms",
        "docs/guides/native-forms.md",
        "Typed form handling",
        "Decode browser input through ordinary language libraries."
    ),
    (
        "direction",
        "docs/direction.md",
        "Language direction",
        "The graph, types, ownership, effects and long-term design."
    ),
    (
        "architecture",
        "docs/architecture.md",
        "Architecture",
        "Where meaning, validation, storage and execution belong."
    ),
    (
        "status",
        "docs/status.md",
        "Development status",
        "Implemented capabilities, publication boundaries and remaining work."
    ),
    (
        "security",
        "docs/security.md",
        "Trust and security",
        "Understand grants, trusted-code execution and security nonclaims."
    ),
}
