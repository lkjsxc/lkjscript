//! Builtin and declared trait identities survive symbolic signature checking.
#[path = "../bridge/library.rs"]
mod bootstrap_library;

use lkjscript::{
    compiler,
    source::{SourceBundle, SourceInput},
};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn compare(text: &str, expected: std::result::Result<(), &str>) {
    let mut files = bootstrap_library::standard_library();
    files.push(SourceInput {
        path: "trait_identity.rs".to_string(),
        text: text.to_string(),
    });
    let result = compiler::check_bundle(&SourceBundle {
        root: "trait_identity.rs".to_string(),
        library: Some("$std/lib.rs".to_string()),
        files,
    });
    match expected {
        Ok(()) => {
            result.unwrap_or_else(|problem| panic!("valid identity rejected: {problem:?}"));
        }
        Err(reason) => {
            let problem = match result {
                Ok(_) => panic!("unrelated trait bound was accepted"),
                Err(problem) => problem,
            };
            assert_eq!(problem.code, "E_TRAIT", "{problem:?}");
            assert!(problem.message.contains(reason), "{problem:?}");
            assert_eq!(problem.path, "trait_identity.rs");
        }
    }
    let directory = std::env::temp_dir().join(format!(
        "lkjscript-trait-identity-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let source = directory.join("identity.rs");
    std::fs::write(&source, text).unwrap();
    let output = Command::new("rustc")
        .args([
            "--edition=2024",
            "--crate-type=lib",
            "--crate-name=identity_case",
            "--emit=metadata",
        ])
        .arg(&source)
        .arg("-o")
        .arg(directory.join("identity.rmeta"))
        .output()
        .unwrap();
    std::fs::remove_dir_all(&directory).unwrap();
    assert_eq!(
        output.status.success(),
        expected.is_ok(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn builtin_clone_cannot_strengthen_a_same_named_declared_bound() {
    compare(
        "trait Clone{} trait Contract{fn take<T:crate::Clone>(&self,value:T)->T;}
         mod nested{pub struct Counter;impl crate::Contract for Counter{fn take<T:Clone>(&self,value:T)->T{value}}}
         pub fn probe()->i64{0}",
        Err("requires a stronger generic bound"),
    );
}

#[test]
fn builtin_copy_does_not_imply_a_same_named_user_clone() {
    compare(
        "trait Clone{} trait Contract{fn take<T:Copy>(&self,value:T)->T;}
         mod nested{pub struct Counter;impl crate::Contract for Counter{fn take<T:crate::Clone>(&self,value:T)->T{value}}}
         pub fn probe()->i64{0}",
        Err("requires a stronger generic bound"),
    );
}

#[test]
fn declared_supertraits_and_aliases_retain_their_identity() {
    compare(
        "trait Clone{} trait Copy:Clone{} trait Contract{fn take<T:crate::Copy>(&self,value:T)->T;}
         mod nested{use crate::Clone as RootClone;pub struct Counter;impl crate::Contract for Counter{fn take<T:RootClone>(&self,value:T)->T{value}}}
         pub fn probe()->i64{0}",
        Ok(()),
    );
}

#[test]
fn trait_type_argument_requirements_retain_declared_identity() {
    compare(
        "trait Clone{} trait Contract<T:crate::Clone>{}
         mod nested{pub struct Holder<T>{value:T} impl<T:Clone> crate::Contract<T> for Holder<T>{}}
         pub fn probe()->i64{0}",
        Err("does not establish a trait type-argument bound"),
    );
    compare(
        "trait Clone{} trait Contract<T:crate::Clone>{}
         mod nested{pub struct Holder<T>{value:T} impl<T:crate::Clone> crate::Contract<T> for Holder<T>{}}
         pub fn probe()->i64{0}",
        Ok(()),
    );
}
