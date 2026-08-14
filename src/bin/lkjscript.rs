use lkjscript::daemon;
use lkjscript::{
    Client, IdempotencyKey, LocalHandle, NodeId, NodeTarget, OperationDraft, Request, RequestId,
    Response, Revision, RuntimeValue, SemanticType, Transaction, TransactionOp, ValueDraft,
    WorkspaceId,
};
use std::path::PathBuf;
use std::process::ExitCode;
use std::str::FromStr;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut arguments = std::env::args().skip(1);
    if arguments.next().as_deref() != Some("--state") {
        return Err(usage("expected --state"));
    }
    let state = PathBuf::from(
        arguments
            .next()
            .ok_or_else(|| usage("missing state directory"))?,
    );
    let command = arguments.next().ok_or_else(|| usage("missing command"))?;
    let client = Client::new(daemon::endpoint_path(&state));
    let request = match command.as_str() {
        "workspace-create" => Request::CreateWorkspace,
        "bootstrap-42" => {
            let workspace = parse::<WorkspaceId>(arguments.next(), "workspace")?;
            no_more(arguments)?;
            Request::ApplyTransaction(bootstrap_42(workspace))
        }
        "summary" => {
            let workspace = parse::<WorkspaceId>(arguments.next(), "workspace")?;
            let revision = Revision::new(parse::<u64>(arguments.next(), "revision")?);
            no_more(arguments)?;
            Request::WorkspaceSummary {
                workspace,
                revision,
            }
        }
        "node" => {
            let workspace = parse::<WorkspaceId>(arguments.next(), "workspace")?;
            let revision = Revision::new(parse::<u64>(arguments.next(), "revision")?);
            let serial = parse::<u64>(arguments.next(), "node serial")?;
            let node = NodeId::new(workspace, serial).map_err(|error| error.to_string())?;
            let expand = match arguments.next().as_deref() {
                None => false,
                Some("--expand") => true,
                Some(_) => return Err(usage("node accepts only optional --expand")),
            };
            no_more(arguments)?;
            Request::Node {
                workspace,
                revision,
                node,
                expand,
            }
        }
        "blockers" => {
            let workspace = parse::<WorkspaceId>(arguments.next(), "workspace")?;
            let revision = Revision::new(parse::<u64>(arguments.next(), "revision")?);
            no_more(arguments)?;
            Request::Blockers {
                workspace,
                revision,
            }
        }
        "run" => {
            let workspace = parse::<WorkspaceId>(arguments.next(), "workspace")?;
            let revision = Revision::new(parse::<u64>(arguments.next(), "revision")?);
            let serial = parse::<u64>(arguments.next(), "entry node serial")?;
            let entry = NodeId::new(workspace, serial).map_err(|error| error.to_string())?;
            no_more(arguments)?;
            Request::Run {
                workspace,
                revision,
                entry,
            }
        }
        "rename-and-set-i64" => {
            let workspace = parse::<WorkspaceId>(arguments.next(), "workspace")?;
            let base = Revision::new(parse::<u64>(arguments.next(), "base revision")?);
            let function = node(workspace, arguments.next(), "function serial")?;
            let constant = node(workspace, arguments.next(), "constant serial")?;
            let name = arguments
                .next()
                .ok_or_else(|| usage("missing replacement name"))?;
            let value = parse::<i64>(arguments.next(), "i64 value")?;
            no_more(arguments)?;
            Request::ApplyTransaction(Transaction {
                workspace,
                base_revision: base,
                idempotency_key: None,
                dry_run: false,
                operations: vec![
                    TransactionOp::RenameNode {
                        node: NodeTarget::Existing(function),
                        name,
                    },
                    TransactionOp::ReplaceOperation {
                        operation: NodeTarget::Existing(constant),
                        replacement: OperationDraft::ConstI64(value),
                    },
                ],
            })
        }
        "shutdown" => {
            no_more(arguments)?;
            Request::Shutdown
        }
        _ => return Err(usage("unknown command")),
    };
    let response = client
        .request(RequestId::new(1), &request)
        .map_err(|error| error.to_string())?;
    print_response(response)
}

fn bootstrap_42(workspace: WorkspaceId) -> Transaction {
    let package = LocalHandle::new(1);
    let module = LocalHandle::new(2);
    let function = LocalHandle::new(3);
    let region = LocalHandle::new(4);
    let block = LocalHandle::new(5);
    let forty = LocalHandle::new(6);
    let two = LocalHandle::new(7);
    let add = LocalHandle::new(8);
    let return_operation = LocalHandle::new(9);
    let local = NodeTarget::Local;
    let result = |operation| ValueDraft::OperationResult {
        operation: local(operation),
        output: 0,
    };
    Transaction {
        workspace,
        base_revision: Revision::INITIAL,
        idempotency_key: Some(IdempotencyKey::from_bytes([0x42; 16])),
        dry_run: false,
        operations: vec![
            TransactionOp::CreatePackage {
                handle: package,
                name: "app".to_owned(),
            },
            TransactionOp::CreateModule {
                handle: module,
                package: local(package),
                name: "root".to_owned(),
            },
            TransactionOp::CreateFunction {
                handle: function,
                module: local(module),
                name: "main".to_owned(),
                result: SemanticType::I64,
            },
            TransactionOp::CreateRegion {
                handle: region,
                function: local(function),
            },
            TransactionOp::CreateBlock {
                handle: block,
                region: local(region),
            },
            TransactionOp::CreateOperation {
                handle: forty,
                block: local(block),
                before: None,
                operation: OperationDraft::ConstI64(40),
            },
            TransactionOp::CreateOperation {
                handle: two,
                block: local(block),
                before: None,
                operation: OperationDraft::ConstI64(2),
            },
            TransactionOp::CreateOperation {
                handle: add,
                block: local(block),
                before: None,
                operation: OperationDraft::AddI64 {
                    lhs: result(forty),
                    rhs: result(two),
                },
            },
            TransactionOp::CreateOperation {
                handle: return_operation,
                block: local(block),
                before: None,
                operation: OperationDraft::Return { value: result(add) },
            },
            TransactionOp::SetFunctionBody {
                function: local(function),
                region: local(region),
            },
            TransactionOp::SetEntryFunction {
                package: local(package),
                function: local(function),
            },
        ],
    }
}

fn print_response(response: Response) -> Result<(), String> {
    match response {
        Response::WorkspaceCreated(summary) | Response::WorkspaceSummary(summary) => {
            println!(
                "workspace={} revision={} hash={} root={} nodes={} complete={} blockers={}",
                summary.workspace,
                summary.revision,
                summary.hash,
                summary.root,
                summary.node_count,
                summary.complete,
                summary.blocker_count
            );
            for entry in summary.entries {
                println!("entry={entry}");
            }
        }
        Response::TransactionApplied(result) => {
            println!(
                "workspace={} revision={} hash={} published={} changes={}",
                result.workspace,
                result.revision,
                result.hash,
                result.published,
                result.diff.changes.len()
            );
            for (handle, node) in result.allocations {
                println!("handle={} node={}", handle.get(), node);
            }
        }
        Response::Node(view) => {
            println!(
                "node={} kind={:?} revision={} complete={} children={} references={} diagnostics={} name={}",
                view.summary.node,
                view.summary.kind,
                view.summary.revision,
                view.summary.complete,
                view.summary.child_count,
                view.summary.reference_count,
                view.summary.diagnostic_count,
                view.summary.display_name.as_deref().unwrap_or("-")
            );
            if let Some(signature) = view.summary.signature {
                println!(
                    "signature_parameters={} result={:?}",
                    signature.parameters.len(),
                    signature.result
                );
            }
            if let Some(record) = view.record {
                println!("record={record:?}");
            }
        }
        Response::Blockers { blockers, .. } => {
            println!("blockers={}", blockers.len());
            for blocker in blockers {
                println!(
                    "owner={} target={} category={:?} expected={:?}",
                    blocker.owner,
                    blocker
                        .target
                        .map(|target| target.to_string())
                        .unwrap_or_else(|| "-".to_owned()),
                    blocker.category,
                    blocker.expected_type
                );
            }
        }
        Response::Run(result) => {
            match result.value {
                RuntimeValue::Unit => println!("unit"),
                RuntimeValue::Bool(value) => println!("bool={value}"),
                RuntimeValue::I64(value) => println!("i64={value}"),
            }
            println!(
                "compile_ns={} execute_ns={}",
                result.compile_nanoseconds, result.execute_nanoseconds
            );
        }
        Response::Acknowledged => println!("acknowledged"),
        Response::Error(error) => {
            return Err(format!(
                "code={:?} operation={:?} target={:?} expected_type={:?} actual_type={:?} message={}",
                error.code,
                error.operation_index,
                error.target,
                error.expected_type,
                error.actual_type,
                error.message
            ));
        }
    }
    Ok(())
}

fn parse<T: FromStr>(value: Option<String>, name: &str) -> Result<T, String>
where
    T::Err: std::fmt::Display,
{
    value
        .ok_or_else(|| usage(&format!("missing {name}")))?
        .parse::<T>()
        .map_err(|error| format!("invalid {name}: {error}"))
}

fn node(workspace: WorkspaceId, value: Option<String>, name: &str) -> Result<NodeId, String> {
    let serial = parse::<u64>(value, name)?;
    NodeId::new(workspace, serial).map_err(|error| error.to_string())
}

fn no_more(mut arguments: impl Iterator<Item = String>) -> Result<(), String> {
    if arguments.next().is_some() {
        Err(usage("too many arguments"))
    } else {
        Ok(())
    }
}

fn usage(reason: &str) -> String {
    format!(
        "{reason}\nusage: lkjscript --state DIRECTORY COMMAND [ARGS]\n\
         commands: workspace-create | bootstrap-42 WORKSPACE | summary WORKSPACE REVISION | \
         node WORKSPACE REVISION SERIAL [--expand] | blockers WORKSPACE REVISION | \
         run WORKSPACE REVISION ENTRY_SERIAL | \
         rename-and-set-i64 WORKSPACE BASE FUNCTION_SERIAL CONSTANT_SERIAL NAME VALUE | shutdown"
    )
}
