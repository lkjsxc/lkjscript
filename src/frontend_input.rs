//! Native command/input policy over the production module loader.
//! The caller supplies bytes; this adapter records a stable snapshot of reads.
use crate::source::{SourceBundle, SourceInput, SourceReport};
use std::collections::BTreeMap;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Output {
    K,
    Check,
    Pack,
}

pub struct SourceOptions {
    pub root: String,
    pub directory: String,
    pub library: Option<String>,
    pub output: Output,
}

pub enum Request {
    Help,
    Bundle { path: String, check: bool },
    Source(SourceOptions),
}

pub fn usage() -> String {
    "usage: frontend [--check] BUNDLE\n       frontend --source ROOT [--source-dir DIR]\n           (--library-dir DIR | --no-library) [--check | --pack]\nROOT is a logical path beneath source-dir (default .).\nLibrary files use $std/... logical paths. --pack writes an LKJSB1 snapshot.\n".to_string()
}

fn value(arguments: &[String], at: &mut usize, option: &str) -> Result<String, String> {
    *at += 1;
    if *at == arguments.len() {
        Err(format!("{option} needs a value"))
    } else {
        Ok(arguments[*at].clone())
    }
}

pub fn parse(arguments: Vec<String>) -> Result<Request, String> {
    if arguments.len() == 1 && arguments[0].as_str() == "--help" {
        return Ok(Request::Help);
    }
    let mut root: Option<String> = None;
    let mut bundle: Option<String> = None;
    let mut directory = ".".to_string();
    let mut directory_set = false;
    let mut library: Option<String> = None;
    let mut library_set = false;
    let mut output = Output::K;
    let mut output_set = false;
    let mut at = 0usize;
    while at < arguments.len() {
        let option = arguments[at].clone();
        match option.as_str() {
            "--source" => {
                if root.is_some() {
                    return Err("--source was supplied more than once".to_string());
                }
                root = Some(value(&arguments, &mut at, &option)?);
            }
            "--source-dir" => {
                if directory_set {
                    return Err("--source-dir was supplied more than once".to_string());
                }
                directory = value(&arguments, &mut at, &option)?;
                directory_set = true;
            }
            "--library-dir" => {
                if library_set {
                    return Err("choose exactly one library directory or --no-library".to_string());
                }
                library = Some(value(&arguments, &mut at, &option)?);
                library_set = true;
            }
            "--no-library" => {
                if library_set {
                    return Err("choose exactly one library directory or --no-library".to_string());
                }
                library_set = true;
            }
            "--check" | "--pack" => {
                if output_set {
                    return Err("choose at most one of --check and --pack".to_string());
                }
                output = if option.as_str() == "--check" {
                    Output::Check
                } else {
                    Output::Pack
                };
                output_set = true;
            }
            _ => {
                if option.as_str().starts_with("--") {
                    return Err(format!("unknown option {option}"));
                }
                if bundle.is_some() {
                    return Err("expected one source bundle path".to_string());
                }
                bundle = Some(option.clone());
            }
        }
        at += 1;
    }
    if let Some(root) = root {
        if root.is_empty() {
            return Err("--source needs a nonempty logical root".to_string());
        }
        if bundle.is_some() {
            return Err("a bundle path cannot be combined with --source".to_string());
        }
        if !library_set {
            return Err("source mode needs --library-dir DIR or --no-library".to_string());
        }
        Ok(Request::Source(SourceOptions {
            root,
            directory,
            library,
            output,
        }))
    } else {
        if directory_set || library_set || output == Output::Pack {
            return Err("source-directory, library and pack options need --source".to_string());
        }
        match bundle {
            Some(path) => Ok(Request::Bundle {
                path,
                check: output == Output::Check,
            }),
            None => Err(usage()),
        }
    }
}

pub struct Loaded {
    pub report: SourceReport,
    pub bundle: SourceBundle,
}

pub struct Cache {
    files: Vec<SourceInput>,
    paths: BTreeMap<String, Option<usize>>,
}

impl Cache {
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            paths: BTreeMap::new(),
        }
    }

    pub fn into_bundle(self, options: &SourceOptions) -> SourceBundle {
        SourceBundle {
            root: options.root.clone(),
            library: if options.library.is_some() {
                Some("$std/lib.rs".to_string())
            } else {
                None
            },
            files: self.files,
        }
    }

    /// Snapshot a successful read or absence once, including ambiguity probes.
    pub fn read<F: FnMut(&str) -> Result<Option<String>, String>>(
        &mut self,
        path: &str,
        read: &mut F,
    ) -> Result<Option<String>, String> {
        let cached = self.paths.get(&path.to_string()).cloned();
        if let Some(index) = cached {
            return Ok(match index {
                Some(index) => Some(self.files[index].text.clone()),
                None => None,
            });
        }
        let text = read(path)?;
        match text {
            Some(text) => {
                let index = self.files.len();
                self.files.push(SourceInput {
                    path: path.to_string(),
                    text: text.clone(),
                });
                self.paths.insert(path.to_string(), Some(index));
                Ok(Some(text))
            }
            None => {
                self.paths.insert(path.to_string(), None);
                Ok(None)
            }
        }
    }
}
