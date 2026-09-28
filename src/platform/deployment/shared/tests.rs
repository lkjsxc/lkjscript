//! Exact identity, private lifecycle and failed-admission witnesses.
use super::*;
use crate::platform::cli::{execute_build, execute_new};
use std::path::Path;

fn fixture() -> (tempfile::TempDir, DeploymentDescriptor) {
    let root = tempfile::tempdir().unwrap();
    let project = root.path().join("project");
    execute_new(&[
        project.display().to_string(),
        "--template".into(),
        "http".into(),
        "--name".into(),
        "shared-fixture".into(),
    ])
    .unwrap();
    execute_build(vec![
        "--project".into(),
        project.display().to_string(),
        "build".into(),
        "--output".into(),
        root.path().join("application.lkja").display().to_string(),
    ])
    .unwrap();
    let mut descriptor =
        decode_deployment(&fs::read(project.join("service.deployment.json")).unwrap()).unwrap();
    descriptor.artifact = "application.lkja".into();
    fs::remove_dir_all(project).unwrap();
    (root, descriptor)
}

fn write(root: &Path, name: &str, descriptor: &DeploymentDescriptor) -> PathBuf {
    let path = root.join(name);
    fs::write(&path, encode_deployment(descriptor).unwrap()).unwrap();
    path
}

fn request() -> HttpRequest {
    HttpRequest {
        method: "GET".into(),
        path: "/".into(),
        query: String::new(),
        headers: Vec::new(),
        body: Vec::new(),
    }
}

#[tokio::test]
async fn exact_shared_code_has_private_kernels_and_unloads_after_last_owner() {
    let (root, mut descriptor) = fixture();
    descriptor
        .configuration
        .insert("private".into(), ConfigurationValue::Text("alpha".into()));
    let first = write(root.path(), "alpha.json", &descriptor);
    descriptor
        .configuration
        .insert("private".into(), ConfigurationValue::Text("beta".into()));
    let second = write(root.path(), "beta.json", &descriptor);
    // Repeated start/stop must not grow a process-global code cache.
    for _ in 0..3 {
        let set = PreparedServiceSet::load(
            &[first.clone(), second.clone()],
            Handle::current(),
            &ExecutionControl::uncancelled(),
        )
        .unwrap();
        assert_eq!(set.observe().instances, 2);
        assert_eq!(set.observe().programs.len(), 1);
        assert_eq!(set.observe().programs[0].instances, 2);
        assert!(set.observe().programs[0].artifact_object_bytes > 0);
        assert_eq!(set.observe().private_configuration_fields, 2);
        let deployments = set.into_deployments();
        assert!(Arc::ptr_eq(
            &deployments[0].program,
            &deployments[1].program
        ));
        assert_ne!(
            deployments[0].descriptor.configuration,
            deployments[1].descriptor.configuration
        );
        let weak = Arc::downgrade(&deployments[0].program);
        let first = deployments[0].http_application().unwrap();
        let second = deployments[1].http_application().unwrap();
        drop(deployments);
        assert_eq!(first.dispatch(request()).await.unwrap().0.status, 200);
        assert_eq!(second.dispatch(request()).await.unwrap().0.status, 200);
        let stopped = first.shutdown().await;
        assert_eq!(stopped.remaining_tasks, 0);
        assert!(stopped.cleanup_failures.is_empty());
        assert!(!first.observe_resident().accepting);
        drop(first);
        assert!(weak.upgrade().is_some());
        assert!(second.observe_resident().accepting);
        assert_eq!(second.dispatch(request()).await.unwrap().0.status, 200);
        assert_eq!(second.observe_resident().admitted, 2);
        let stopped = second.shutdown().await;
        assert_eq!(stopped.remaining_tasks, 0);
        assert!(stopped.cleanup_failures.is_empty());
        drop(second);
        assert!(weak.upgrade().is_none(), "no cache retains unloaded code");
    }
}

#[tokio::test]
async fn distinct_exact_programs_coexist_without_identity_aliasing() {
    let (first_root, descriptor) = fixture();
    let (second_root, second_descriptor) = fixture();
    let paths = [
        write(first_root.path(), "first.json", &descriptor),
        write(second_root.path(), "second.json", &second_descriptor),
    ];
    let set = PreparedServiceSet::load(&paths, Handle::current(), &ExecutionControl::uncancelled())
        .unwrap();
    assert_eq!(set.observe().programs.len(), 2);
    assert!(!Arc::ptr_eq(
        &set.deployments[0].program,
        &set.deployments[1].program
    ));
    let mut cleanup = deployment_error("test_cleanup", "not invoked");
    set.close_uninvoked(&mut cleanup);
    assert!(cleanup.notes.is_empty());
}

#[tokio::test]
async fn cache_hits_still_read_strict_artifacts_and_validate_each_descriptor() {
    let (root, descriptor) = fixture();
    let first = write(root.path(), "first.json", &descriptor);
    let control = ExecutionControl::uncancelled();
    let mut cache = BTreeMap::new();
    let admitted = AdmittedDeployment::load_shared(&first, &control, &mut cache).unwrap();
    let again = AdmittedDeployment::load_shared(&first, &control, &mut cache).unwrap();
    assert!(Arc::ptr_eq(&admitted.program, &again.program));
    let mut invalid = descriptor.clone();
    invalid.target = "missing".into();
    let invalid = write(root.path(), "invalid.json", &invalid);
    assert!(AdmittedDeployment::load_shared(&invalid, &control, &mut cache).is_err());
    let mut corrupted = fs::read(root.path().join("application.lkja")).unwrap();
    *corrupted.last_mut().unwrap() ^= 1;
    fs::write(root.path().join("application.lkja"), corrupted).unwrap();
    assert!(AdmittedDeployment::load_shared(&first, &control, &mut cache).is_err());
    assert_eq!(cache.len(), 1);
}

#[tokio::test]
async fn group_bounds_and_cancellation_reject_before_live_preparation() {
    let (root, mut descriptor) = fixture();
    descriptor
        .runtime
        .as_mut()
        .unwrap()
        .maximum_concurrent_tasks = MAXIMUM_CONCURRENT_TASKS;
    let first = write(root.path(), "first.json", &descriptor);
    for paths in [
        Vec::new(),
        vec![first.clone(); MAXIMUM_SHARED_DEPLOYMENTS + 1],
        vec![first.clone(), first.clone()],
    ] {
        let error =
            PreparedServiceSet::load(&paths, Handle::current(), &ExecutionControl::uncancelled())
                .unwrap_err();
        assert_eq!(error.code, "shared_serve_limit");
    }
    let control = ExecutionControl::uncancelled();
    control.cancel();
    assert_eq!(
        PreparedServiceSet::load(&[first], Handle::current(), &control)
            .unwrap_err()
            .class,
        DiagnosticClass::Cancelled
    );
}
