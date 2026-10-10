use super::*;

#[test]
fn all_declared_cargo_executables_require_machine_records_before_test_arguments() {
    let root = tempfile::tempdir().expect("registry fixture");
    let registry = base_registry(root.path(), root.path(), Path::new("/bin/true"))
        .expect("maintained registry");
    let mut producers = Vec::new();
    for gate in &registry.gates {
        if gate.command.first().map(String::as_str) == Some("cargo")
            && !gate.required_outputs.is_empty()
        {
            producers.push(gate.name.as_str());
            assert!(
                !gate.cacheable,
                "Cargo must observe its current output selection"
            );
            let arguments: Vec<_> = gate.command.iter().take_while(|arg| *arg != "--").collect();
            assert_eq!(
                arguments
                    .iter()
                    .filter(|arg| arg.as_str() == "--message-format=json")
                    .count(),
                1
            );
            assert_eq!(
                gate.required_outputs,
                [root.path().join("target/release/lkjscript")]
            );
        }
    }
    assert_eq!(producers, ["release_build", "release_command_lifecycle"]);
    let requested = profile("full").expect("full profile");
    let identity = registry
        .profile_digest("full", &requested)
        .expect("profile identity");
    let mut weakened = registry.clone();
    for gate in &mut weakened.gates {
        gate.command
            .retain(|argument| argument != "--message-format=json");
    }
    assert_ne!(
        identity,
        weakened
            .profile_digest("full", &requested)
            .expect("weakened identity")
    );
}
