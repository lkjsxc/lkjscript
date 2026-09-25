//! One useful ordinary application: explicit forms, conditional saves and detached restart.
use super::*;
use serde_json::json;

#[path = "native_editor_author.rs"]
mod author;
use super::native_http as http;

// Public, disposable local fixture data, never a deployment default or real credential.
const AUTH: &str = "Basic Zml4dHVyZTpuYXRpdmUtZWRpdG9yLW9ubHk=";
const AUTH_ENVIRONMENT: &[(&str, &str)] = &[("LKJSCRIPT_EDITOR_AUTHORIZATION", AUTH)];

fn headers() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Host", "localhost"),
        ("Authorization", AUTH),
        ("Origin", "http://localhost"),
        ("Content-Type", "application/x-www-form-urlencoded"),
    ]
}

fn encoded(base: &str, text: &str) -> String {
    let text = text
        .as_bytes()
        .iter()
        .map(|byte| format!("%{byte:02X}"))
        .collect::<String>();
    format!("base={base}&text={text}&intent=save")
}

fn revision(body: &str) -> &str {
    body.split_once("name=\"base\" value=\"")
        .unwrap()
        .1
        .split('"')
        .next()
        .unwrap()
}

fn draft(body: &str) -> &str {
    body.split_once("<textarea ")
        .unwrap()
        .1
        .split_once(">\n")
        .unwrap()
        .1
        .split_once("</textarea>")
        .unwrap()
        .0
}

#[test]
fn native_editor_public_composition_rejects_untrusted_inputs_and_preserves_conditional_saves() {
    let (public, descriptor) = author::author();
    assert!(!public.project.exists());
    let head = || std::fs::read(public.root.path().join("data/HEAD")).unwrap();
    let first = http::Server::start(&public, "first", &descriptor, AUTH_ENVIRONMENT);
    let initial = head();
    let get = http::send(first.address, "GET", "/", &headers(), "");
    assert_eq!(get.status, 200);
    assert_eq!(revision(&get.body), "0");
    assert_eq!(draft(&get.body), "");
    assert!(get.headers.contains("cache-control: no-store"));
    assert!(get.headers.contains("script-src 'none'"));
    assert!(get.headers.contains("form-action 'self'"));
    // Fetch's Origin algorithm makes non-CORS POST Origin null under no-referrer.
    // Keep real same-origin browser forms compatible without admitting null origins.
    assert!(get.headers.contains("referrer-policy: same-origin"));
    assert!(get.body.contains("<form method=\"post\""));
    assert_eq!(head(), initial);

    // An operational put failure is neither a saved note nor an invalid form.
    let mut constrained = descriptor.clone();
    constrained["grants"][3]["adapter"]["limits"]["maximum_value_bytes"] = json!(1);
    let constrained = http::Server::start(&public, "constrained", &constrained, AUTH_ENVIRONMENT);
    assert_eq!(
        http::send(
            constrained.address,
            "POST",
            "/",
            &headers(),
            &encoded("0", "cannot fit the store's value limit")
        )
        .status,
        500
    );
    assert_eq!(head(), initial, "failed put must not publish data");
    constrained.stop();

    for (name, value, expected) in [
        ("Authorization", None, 401),
        ("Authorization", Some("wrong"), 401),
        ("Host", Some("other.test"), 400),
        ("Origin", None, 403),
        ("Origin", Some("null"), 403),
        ("Origin", Some("http://other.test"), 403),
        ("Content-Type", None, 415),
        ("Content-Type", Some("application/json"), 415),
    ] {
        let mut custom = headers();
        custom.retain(|(key, _)| *key != name);
        if let Some(value) = value {
            custom.push((name, value));
        }
        let reply = http::send(
            first.address,
            "POST",
            "/",
            &custom,
            &encoded("0", "not saved"),
        );
        assert_eq!(reply.status, expected, "rejected {name}");
        if expected == 401 {
            assert!(reply.headers.contains("www-authenticate: basic"));
        }
        assert_eq!(head(), initial);
    }
    for (name, expected) in [
        ("Authorization", 401),
        ("Host", 400),
        ("Origin", 403),
        ("Content-Type", 415),
    ] {
        let mut custom = headers();
        custom.push(*custom.iter().find(|(key, _)| *key == name).unwrap());
        assert_eq!(
            http::send(
                first.address,
                "POST",
                "/",
                &custom,
                &encoded("0", "duplicate")
            )
            .status,
            expected
        );
        assert_eq!(head(), initial);
    }
    for body in [
        "base=0&text=a&text=b&intent=save",
        "base=0&base=1&intent=save",
        "base=0&text=a&intent=save&unknown=x",
        "base=0&text=a",
        "base=0&text=a&intent=other",
        "base=0&text=%&intent=save",
        "base=0&text=%ff&intent=save",
        "base=0&text=%00&intent=save",
        "base=0&text=%7f&intent=save",
        "base=00&text=a&intent=save",
        "base=-1&text=a&intent=save",
    ] {
        assert_eq!(
            http::send(first.address, "POST", "/", &headers(), body).status,
            400,
            "invalid form"
        );
        assert_eq!(head(), initial);
    }
    for (method, route, body, status) in [
        (
            "GET",
            "/?base=0&text=ignored&intent=save",
            String::new(),
            200,
        ),
        ("PUT", "/", encoded("0", "wrong method"), 404),
        ("POST", "/unknown", encoded("0", "wrong route"), 404),
        ("POST", "/", encoded("1", "stale"), 409),
        (
            "POST",
            "/",
            encoded("9999999999999999999", "not an integer conversion"),
            409,
        ),
        ("POST", "/", "x".repeat(16385), 413),
    ] {
        assert_eq!(
            http::send(first.address, method, route, &headers(), &body).status,
            status
        );
        assert_eq!(head(), initial);
    }

    let text = "\n日本語 🌱\r\nsecond\rthird\ttab &+";
    let saved = http::send(first.address, "POST", "/", &headers(), &encoded("0", text));
    assert_eq!(saved.status, 303);
    assert!(saved.headers.contains("location: /"));
    assert_eq!(saved.body, "");
    let get = http::send(first.address, "GET", "/", &headers(), "");
    assert_eq!(revision(&get.body), "1");
    assert_eq!(draft(&get.body), "\n日本語 🌱\nsecond\nthird\ttab &amp;+");

    let second = http::Server::start(&public, "second", &descriptor, AUTH_ENVIRONMENT);
    let barrier = std::sync::Barrier::new(8);
    let races = std::thread::scope(|scope| {
        let handles = (0..8)
            .map(|index| {
                let address = if index % 2 == 0 {
                    first.address
                } else {
                    second.address
                };
                let barrier = &barrier;
                scope.spawn(move || {
                    barrier.wait();
                    http::send(
                        address,
                        "POST",
                        "/",
                        &headers(),
                        &encoded("1", &format!("writer {index}")),
                    )
                })
            })
            .collect::<Vec<_>>();
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(races.iter().filter(|reply| reply.status == 303).count(), 1);
    assert_eq!(races.iter().filter(|reply| reply.status == 409).count(), 7);
    for (index, reply) in races
        .iter()
        .enumerate()
        .filter(|(_, reply)| reply.status == 409)
    {
        assert_eq!(draft(&reply.body), format!("writer {index}"));
        assert_eq!(revision(&reply.body), "2");
        assert!(reply.body.contains("Current saved text"));
        assert!(reply.body.contains("Save reviewed draft"));
    }
    assert_eq!(
        revision(&http::send(first.address, "GET", "/", &headers(), "").body),
        "2"
    );
    assert_eq!(
        http::send(
            first.address,
            "POST",
            "/",
            &headers(),
            &encoded("2", &"x".repeat(4096))
        )
        .status,
        303
    );
    let accepted = head();
    assert_eq!(
        http::send(
            first.address,
            "POST",
            "/",
            &headers(),
            &encoded("3", &"x".repeat(4097))
        )
        .status,
        400
    );
    assert_eq!(head(), accepted);
    first.stop();
    second.stop();
    let restarted = http::Server::start(&public, "restart", &descriptor, AUTH_ENVIRONMENT);
    let get = http::send(restarted.address, "GET", "/", &headers(), "");
    assert_eq!(get.status, 200);
    assert_eq!(revision(&get.body), "3");
    assert_eq!(draft(&get.body), "x".repeat(4096));
    assert_eq!(head(), accepted);

    // Alter only disposable operational data through a separate public native command.
    author::install_data_fixture(&public);
    for mode in [0, 1, 3, 4] {
        author::write_fixture(&public, &descriptor, mode);
        let corrupt = head();
        assert_eq!(
            http::send(restarted.address, "GET", "/", &headers(), "").status,
            503
        );
        assert_eq!(
            http::send(
                restarted.address,
                "POST",
                "/",
                &headers(),
                &encoded("0", "not a repair")
            )
            .status,
            503
        );
        assert_eq!(
            head(),
            corrupt,
            "invalid storage must not silently become empty"
        );
    }
    author::write_fixture(&public, &descriptor, 2);
    let exhausted = head();
    let get = http::send(restarted.address, "GET", "/", &headers(), "");
    assert_eq!(get.status, 200);
    assert_eq!(revision(&get.body), "9223372036854775807");
    assert_eq!(
        http::send(
            restarted.address,
            "POST",
            "/",
            &headers(),
            &encoded("9223372036854775807", "must not wrap")
        )
        .status,
        503
    );
    assert_eq!(head(), exhausted);
    restarted.stop();

    let deployment = public.input("missing-secret.json", &descriptor.to_string());
    let failure = support::output(
        Command::new(&public.executable)
            .args(["serve", "--deployment", path(&deployment)])
            .current_dir(public.root.path())
            .env_clear()
            .env("PATH", ""),
    )
    .unwrap();
    assert!(!failure.status.success());
    assert!(!String::from_utf8_lossy(&failure.stdout).contains("\"event\":\"ready\""));
    assert_eq!(head(), exhausted);
}
