use super::*;

#[test]
fn instance_id_and_strict_json_are_canonical_and_closed() {
    let instance = InstanceId::from_bytes([0x12; 16]);
    assert_eq!(instance.to_string(), "12121212121212121212121212121212");
    assert_eq!(instance.to_string().parse::<InstanceId>(), Ok(instance));
    for malformed in [
        "1212",
        "1212121212121212121212121212121A",
        "1212121212121212121212121212121g",
    ] {
        assert!(malformed.parse::<InstanceId>().is_err());
    }

    let canonical =
        br#"{"version":2,"instance":"12121212121212121212121212121212","base_revision":7}"#;
    let request = strict_json::<InstanceDeleteRequest>(canonical, "delete").expect("canonical");
    validate_version(request.version).expect("active contract version");
    assert_eq!(request.instance, instance);
    assert_eq!(request.base_revision, 7);
    assert_eq!(
        validate_version(1).expect_err("old contract version").code,
        ErrorCode::ProtocolVersion
    );
    for malformed in [
        br#"{"version":2,"version":2,"instance":"12121212121212121212121212121212","base_revision":7}"#.as_slice(),
        br#"{"version":2,"instance":"12121212121212121212121212121212","base_revision":7,"extra":0}"#.as_slice(),
        br#"{"version":2,"instance":"12121212121212121212121212121212","base_revision":7}x"#.as_slice(),
    ] {
        assert!(strict_json::<InstanceDeleteRequest>(malformed, "delete").is_err());
    }
}

#[test]
fn instance_policy_event_key_and_path_bounds_accept_exact_and_reject_one_over() {
    let exact = InstancePolicy::default();
    validate_policy(exact).expect("global maxima are valid policy values");
    for invalid in [
        InstancePolicy {
            maximum_state_bytes: exact.maximum_state_bytes + 1,
            ..exact
        },
        InstancePolicy {
            maximum_event_bytes: exact.maximum_event_bytes + 1,
            ..exact
        },
        InstancePolicy {
            maximum_history_bytes: exact.maximum_history_bytes + 1,
            ..exact
        },
        InstancePolicy {
            maximum_transitions: exact.maximum_transitions + 1,
            ..exact
        },
        InstancePolicy {
            maximum_replay_work: exact.maximum_replay_work + 1,
            ..exact
        },
        InstancePolicy {
            maximum_state_bytes: 0,
            ..exact
        },
    ] {
        assert_eq!(
            validate_policy(invalid).expect_err("invalid policy").code,
            ErrorCode::PolicyExceeded
        );
    }

    let exact_key = "a".repeat(MAXIMUM_EVENT_KEY_BYTES);
    validate_event_key(InstanceMode::Commit, Some(&exact_key)).expect("maximum event key");
    assert!(
        validate_event_key(
            InstanceMode::Commit,
            Some(&"a".repeat(MAXIMUM_EVENT_KEY_BYTES + 1))
        )
        .is_err()
    );
    assert!(validate_event_key(InstanceMode::Commit, Some("bad/key")).is_err());
    assert!(validate_event_key(InstanceMode::ValidateOnly, Some("unused")).is_err());

    let exact_path = format!("/{}", "a".repeat(MAXIMUM_INSTANCE_PATH_BYTES - 1));
    validate_absolute_path(Path::new(&exact_path), "path").expect("maximum path grammar");
    let over_path = format!("/{}", "a".repeat(MAXIMUM_INSTANCE_PATH_BYTES));
    assert!(validate_absolute_path(Path::new(&over_path), "path").is_err());
}

#[test]
fn instance_envelope_rejects_every_truncation_mutation_old_version_and_trailing_byte() {
    let value = HostAttemptRecord {
        instance: InstanceId::from_bytes([0x34; 16]),
        command: CommandId::from_bytes([0x56; 32]),
        interface: HostInterface::ImmutableBlob.identity(),
        grant: HostGrantDigest::from_bytes([0x78; 32]),
        adapter: HostAdapterKind::Production,
    };
    let (bytes, digest) =
        encode_envelope(ATTEMPT_MAGIC, ATTEMPT_DOMAIN, &value, 1024).expect("encode attempt");
    let (decoded, decoded_digest): (HostAttemptRecord, InstanceRecordDigest) =
        decode_envelope(ATTEMPT_MAGIC, ATTEMPT_DOMAIN, &bytes, 1024).expect("decode attempt");
    assert_eq!(decoded, value);
    assert_eq!(decoded_digest, digest);
    assert_eq!(
        encode_envelope(ATTEMPT_MAGIC, ATTEMPT_DOMAIN, &decoded, 1024)
            .expect("re-encode")
            .0,
        bytes
    );

    for end in 0..bytes.len() {
        assert!(
            decode_envelope::<HostAttemptRecord>(
                ATTEMPT_MAGIC,
                ATTEMPT_DOMAIN,
                &bytes[..end],
                1024
            )
            .is_err(),
            "truncation {end}"
        );
    }
    for index in 0..bytes.len() {
        let mut mutated = bytes.clone();
        mutated[index] ^= 1;
        assert!(
            decode_envelope::<HostAttemptRecord>(ATTEMPT_MAGIC, ATTEMPT_DOMAIN, &mutated, 1024)
                .is_err(),
            "mutation {index}"
        );
    }
    let mut old_version = bytes.clone();
    old_version[8..10].copy_from_slice(&1_u16.to_le_bytes());
    assert!(
        decode_envelope::<HostAttemptRecord>(ATTEMPT_MAGIC, ATTEMPT_DOMAIN, &old_version, 1024)
            .is_err()
    );
    let mut trailing = bytes;
    trailing.push(0);
    assert!(
        decode_envelope::<HostAttemptRecord>(ATTEMPT_MAGIC, ATTEMPT_DOMAIN, &trailing, 1024)
            .is_err()
    );
}

#[test]
fn host_interfaces_and_operation_outcomes_are_closed_and_disjoint() {
    assert_ne!(
        HostInterface::ApplicationActivation.identity(),
        HostInterface::ImmutableBlob.identity()
    );
    assert!(application::host_outcome_is_compatible(
        HostOperation::ActivateApplication,
        HostOutcomeClass::OutcomeUnknown
    ));
    assert!(!application::host_outcome_is_compatible(
        HostOperation::ActivateApplication,
        HostOutcomeClass::AlreadyPresent
    ));
    assert!(application::host_outcome_is_compatible(
        HostOperation::PutBlob,
        HostOutcomeClass::AlreadyPresent
    ));
    assert!(!application::host_outcome_is_compatible(
        HostOperation::InspectBlob,
        HostOutcomeClass::Succeeded
    ));
}

#[test]
fn immutable_blob_adapter_is_content_addressed_bounded_and_idempotent() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let namespace = temporary.path().join("objects");
    create_private_directory(&namespace).expect("blob namespace");
    let grant = HostGrant {
        version: INSTANCE_CONTRACT_VERSION,
        name: "objects".into(),
        instance: InstanceId::from_bytes([0x33; 16]),
        slot: "blob".into(),
        interface: HostInterface::ImmutableBlob,
        adapter: HostAdapterKind::Production,
        descriptor: HostGrantDescriptor::ImmutableBlob {
            namespace: namespace.to_string_lossy().into_owned(),
            maximum_objects: 2,
            maximum_bytes: 16,
        },
    };
    validate_grant(&grant, grant.instance).expect("valid blob grant");
    let content = ByteString::from_slice(b"exact").expect("content");
    let input = HostAdapterInput::None;
    let (first, digest) = put_blob_adapter(&grant, &input, &content).expect("first put");
    assert_eq!(first, HostOutcomeClass::Succeeded);
    let (second, repeated) = put_blob_adapter(&grant, &input, &content).expect("repeat put");
    assert_eq!(second, HostOutcomeClass::AlreadyPresent);
    assert_eq!(repeated, digest);
    let (present, evidence) = inspect_blob_adapter(&grant, &input, &digest).expect("inspect");
    assert_eq!(present, HostOutcomeClass::ReconciliationPresent);
    assert_eq!(evidence, digest);
}

#[test]
fn immutable_publication_is_repeatable_but_conflicting_bytes_reject() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let directory = temporary.path();
    let path = directory.join("object");
    publish_immutable(directory, &path, b"exact", ".test-").expect("publish exact");
    publish_immutable(directory, &path, b"exact", ".test-").expect("repeat exact");
    assert_eq!(fs::read(&path).expect("published bytes"), b"exact");
    assert_eq!(
        publish_immutable(directory, &path, b"other", ".test-")
            .expect_err("conflict")
            .code,
        ErrorCode::ArtifactCorrupt
    );
}

#[test]
fn canonical_paths_reject_relative_dot_repeated_and_symlinked_parents() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    assert!(validate_absolute_path(Path::new("relative"), "path").is_err());
    assert!(validate_absolute_path(Path::new("/tmp/../escape"), "path").is_err());
    assert!(validate_absolute_path(Path::new("/tmp//repeat"), "path").is_err());
    let real = temporary.path().join("real");
    fs::create_dir(&real).expect("real directory");
    let linked = temporary.path().join("linked");
    std::os::unix::fs::symlink(&real, &linked).expect("symlink");
    assert!(validate_parent_chain(&linked.join("child"), "path").is_err());
    assert!(
        validate_source_path(&linked.join("application.lkja"), temporary.path()).is_err(),
        "a lexically contained source cannot traverse a symlinked parent"
    );

    let instance = temporary.path().join("instance");
    create_private_directory(&instance).expect("instance authority directory");
    for child in ["records", "outcomes", "attempts"] {
        create_private_directory(&instance.join(child)).expect("instance authority directory");
    }
    assert!(validate_instance_directory_layout(&instance).is_ok());
    fs::remove_dir(instance.join("outcomes")).expect("remove outcomes directory");
    std::os::unix::fs::symlink(&real, instance.join("outcomes"))
        .expect("substituted outcomes symlink");
    assert!(validate_instance_directory_layout(&instance).is_err());
}

#[test]
fn activation_faults_distinguish_previsibility_failure_from_unknown_visibility() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let slot = temporary.path().join("active.lkja");
    let bytes = b"exact application bytes";

    for fault in [
        ActivationFault::BeforeWrite,
        ActivationFault::AfterWrite,
        ActivationFault::AfterFileSync,
    ] {
        let error = activate_slot_with_fault(&slot, bytes, fault).expect_err("injected failure");
        assert_eq!(error.code, ErrorCode::Io);
        assert!(
            !slot.exists(),
            "previsibility fault {fault:?} exposed a slot"
        );
    }

    for fault in [
        ActivationFault::AfterVisibility,
        ActivationFault::AfterDirectorySync,
    ] {
        let error = activate_slot_with_fault(&slot, bytes, fault).expect_err("unknown outcome");
        assert_eq!(error.code, ErrorCode::ArtifactPublicationOutcomeUnknown);
        assert_eq!(fs::read(&slot).expect("visible slot"), bytes);
        fs::remove_file(&slot).expect("reset slot");
    }

    activate_slot_with_fault(&slot, bytes, ActivationFault::None).expect("activation");
    assert_eq!(fs::read(slot).expect("active bytes"), bytes);
}

#[test]
fn immutable_object_faults_have_exact_visibility_and_repeatable_recovery() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let bytes = b"canonical immutable object";
    let cases = [
        (ImmutablePublicationFault::BeforeWrite, false),
        (ImmutablePublicationFault::AfterWrite, false),
        (ImmutablePublicationFault::AfterFileSync, false),
        (ImmutablePublicationFault::AfterLink, true),
        (ImmutablePublicationFault::AfterCleanup, true),
        (ImmutablePublicationFault::AfterDirectorySync, true),
    ];

    for (index, (fault, visible)) in cases.into_iter().enumerate() {
        let directory = temporary.path().join(index.to_string());
        fs::create_dir(&directory).expect("case directory");
        let path = directory.join("object");
        let error = publish_immutable_with_fault(&directory, &path, bytes, ".fault-", fault)
            .expect_err("injected publication fault");
        if visible {
            assert_eq!(error.code, ErrorCode::ArtifactPublicationOutcomeUnknown);
            assert_eq!(fs::read(&path).expect("visible immutable object"), bytes);
            publish_immutable(&directory, &path, bytes, ".retry-")
                .expect("exact retry observes the same immutable object");
        } else {
            assert_eq!(error.code, ErrorCode::Io);
            assert!(!path.exists(), "previsibility fault {fault:?} published");
            publish_immutable(&directory, &path, bytes, ".retry-")
                .expect("previsibility retry publishes");
        }
        assert_eq!(fs::read(path).expect("recovered object"), bytes);
    }
}

#[test]
fn head_faults_preserve_old_authority_or_report_visible_unknown() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let old = b"old exact head";
    let new = b"new exact head";
    let cases = [
        (HeadPublicationFault::BeforeWrite, false),
        (HeadPublicationFault::AfterWrite, false),
        (HeadPublicationFault::AfterFileSync, false),
        (HeadPublicationFault::AfterVisibility, true),
        (HeadPublicationFault::AfterDirectorySync, true),
    ];

    for (index, (fault, visible)) in cases.into_iter().enumerate() {
        let directory = temporary.path().join(index.to_string());
        fs::create_dir(&directory).expect("case directory");
        write_new_file(&directory.join(HEAD_FILE), old).expect("old head");
        let error =
            publish_head_bytes_with_fault(&directory, new, fault).expect_err("injected head fault");
        if visible {
            assert_eq!(error.code, ErrorCode::ArtifactPublicationOutcomeUnknown);
            assert_eq!(fs::read(directory.join(HEAD_FILE)).expect("new head"), new);
        } else {
            assert_eq!(error.code, ErrorCode::Io);
            assert_eq!(fs::read(directory.join(HEAD_FILE)).expect("old head"), old);
        }
    }

    let directory = temporary.path().join("success");
    fs::create_dir(&directory).expect("success directory");
    write_new_file(&directory.join(HEAD_FILE), old).expect("old head");
    publish_head_bytes_with_fault(&directory, new, HeadPublicationFault::None)
        .expect("publish head");
    assert_eq!(fs::read(directory.join(HEAD_FILE)).expect("new head"), new);
}

#[test]
fn instance_creation_directory_faults_have_one_visibility_boundary() {
    let temporary = tempfile::tempdir().expect("temporary root");
    let root = temporary.path();
    let cases = [
        (InstanceDirectoryFault::BeforeVisibility, false),
        (InstanceDirectoryFault::AfterVisibility, true),
        (InstanceDirectoryFault::AfterDirectorySync, true),
    ];

    for (index, (fault, visible)) in cases.into_iter().enumerate() {
        let staging = root.join(format!("staging-{index}"));
        let destination = root.join(format!("instance-{index}"));
        fs::create_dir(&staging).expect("staging directory");
        write_new_file(&staging.join("authority"), b"exact").expect("staged authority");
        sync_directory(&staging).expect("staging sync");
        let error = publish_instance_directory_with_fault(root, &staging, &destination, fault)
            .expect_err("injected directory fault");
        if visible {
            assert_eq!(error.code, ErrorCode::ArtifactPublicationOutcomeUnknown);
            assert_eq!(
                fs::read(destination.join("authority")).expect("visible authority"),
                b"exact"
            );
        } else {
            assert_eq!(error.code, ErrorCode::Io);
            assert!(!destination.exists());
            assert!(staging.exists());
        }
    }

    let staging = root.join("staging-success");
    let destination = root.join("instance-success");
    fs::create_dir(&staging).expect("staging directory");
    publish_instance_directory_with_fault(
        root,
        &staging,
        &destination,
        InstanceDirectoryFault::None,
    )
    .expect("directory publication");
    assert!(destination.is_dir());
}
