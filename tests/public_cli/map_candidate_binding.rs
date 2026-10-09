//! Exercise release selection in isolated child processes, without changing shared env.
use super::native_map_entries::support as components;
use super::*;

#[test]
fn release_candidate_is_the_explicit_map_copy_source() {
    const PROBE: &str = "LKJSCRIPT_MAP_BINDING_PROBE";
    if env::var_os(PROBE).is_some() {
        let candidate = binary();
        let expected = components::digest(&candidate);
        let copied = components::Native::from_candidate(&candidate);
        assert_eq!(expected, components::digest(&copied.root.join("lkjscript")));
        assert_eq!(
            copied.cli(&copied.root, &["binding-probe"], true),
            "selected-map-candidate\n"
        );
        assert_eq!(expected, components::digest(&candidate));
        assert_eq!(expected, components::digest(&copied.root.join("lkjscript")));
        return;
    }
    let temporary = tempfile::tempdir().unwrap();
    let selected = temporary.path().join("selected");
    let substitute = temporary.path().join("substitute");
    for (file, text) in [
        (&selected, "selected-map-candidate"),
        (&substitute, "wrong-candidate"),
    ] {
        std::fs::write(file, format!("#!/bin/sh\nprintf '%s\\n' '{text}'\n")).unwrap();
        std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let missing = temporary.path().join("absent");
    let relative = PathBuf::from("selected");
    let link = temporary.path().join("link");
    std::os::unix::fs::symlink(&selected, &link).unwrap();
    for (candidate, expected) in [
        (&selected, true),
        (&substitute, false),
        (&missing, false),
        (&relative, false),
        (&link, false),
    ] {
        let output = support::output(
            Command::new(env::current_exe().unwrap())
                .args([
                    "--exact",
                    "map_candidate_binding::release_candidate_is_the_explicit_map_copy_source",
                    "--nocapture",
                ])
                .current_dir(temporary.path())
                .env_clear()
                .env("PATH", "")
                .env("HOME", temporary.path())
                .env(PROBE, "1")
                .env(RELEASE_CANDIDATE_ENVIRONMENT, candidate)
                .env("LKJSCRIPT_COMPONENT_CANDIDATE", &substitute)
                .env("CARGO_BIN_EXE_lkjscript", &substitute),
        )
        .unwrap();
        assert_eq!(
            output.status.success(),
            expected,
            "{}: {}{}",
            candidate.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
