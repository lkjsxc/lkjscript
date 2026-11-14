//! Authority is declared by entry parameters; this query never manufactures it.
//! Effects are a conservative traversal of checked-IR control flow, direct calls,
//! custom destructors and owning structural cleanup. No source/library scan.
use crate::ir::{CheckedProgram, Function, IntrinsicOp, Op, Place, Program, Projection, TypeKind};

#[derive(Clone)]
pub struct Requirement {
    pub parameter: usize,
    pub ty: usize,
    pub name: String,
    pub capability: String,
    pub needs_value: bool,
}
#[derive(Clone)]
pub struct Manifest {
    pub full_world: bool,
    pub requirements: Vec<Requirement>,
}
#[derive(Clone)]
pub struct Operation {
    pub name: String,
    pub line: usize,
    pub column: usize,
    pub cleanup: bool,
}
#[derive(Clone)]
pub struct ReachableFunction {
    pub function: usize,
    pub through_drop: bool,
    pub operations: Vec<Operation>,
}
pub struct Report {
    pub entry: usize,
    pub manifest: Manifest,
    pub functions: Vec<ReachableFunction>,
}

fn capability(kind: &TypeKind) -> Option<(&'static str, bool)> {
    match kind {
        TypeKind::Stdout => Some(("Stdout", false)),
        TypeKind::Stdin => Some(("Stdin", false)),
        TypeKind::Stderr => Some(("Stderr", false)),
        TypeKind::Clock => Some(("Clock", false)),
        TypeKind::Process => Some(("Process", false)),
        TypeKind::Directory => Some(("Directory", true)),
        TypeKind::Listener => Some(("Listener", true)),
        _ => None,
    }
}
pub fn manifest(checked: &CheckedProgram, entry: usize) -> Result<Manifest, String> {
    let program = checked.program();
    let function = program
        .functions
        .get(entry)
        .ok_or("E_ENTRY unknown entry function")?;
    if function.params == 1 && matches!(program.types[function.locals[0].ty].kind, TypeKind::World)
    {
        return Ok(Manifest {
            full_world: true,
            requirements: Vec::new(),
        });
    }
    let mut requirements: Vec<Requirement> = Vec::new();
    for parameter in 0..function.params {
        let local = &function.locals[parameter];
        let ty = &program.types[local.ty];
        if matches!(ty.kind, TypeKind::World) {
            return Err("E_ENTRY World must be the entry's only parameter".to_string());
        }
        if matches!(ty.kind, TypeKind::File) {
            return Err(format!("E_ENTRY File parameter {} cannot be granted; accept Directory and open the file inside the entry", local.name));
        }
        let (label, needs_value) = match capability(&ty.kind) {
            Some(value) => value,
            None => {
                return Err(format!(
                "E_ENTRY parameter {} has ungrantable type {}; use an owned capability parameter",
                local.name, ty.name
            ))
            }
        };
        let mut valid_name = !local.name.is_empty();
        for byte in local.name.as_bytes() {
            if *byte <= 32 || *byte == b'=' || *byte == 127 {
                valid_name = false;
            }
        }
        if !valid_name {
            return Err("E_ENTRY grant parameter names must be nonempty and contain no whitespace or equals sign".to_string());
        }
        for previous in &requirements {
            if previous.name == local.name {
                return Err(format!(
                    "E_ENTRY duplicate grant parameter name {}",
                    local.name
                ));
            }
        }
        requirements.push(Requirement {
            parameter,
            ty: local.ty,
            name: local.name.clone(),
            capability: label.to_string(),
            needs_value,
        });
    }
    Ok(Manifest {
        full_world: false,
        requirements,
    })
}
fn effect(op: IntrinsicOp) -> bool {
    // All remaining closed-vocabulary intrinsics consume explicit authority.
    // Keep additions conservative: an unclassified new operation is displayed.
    !matches!(
        op,
        IntrinsicOp::CharFromU32
            | IntrinsicOp::F64FromBits
            | IntrinsicOp::F64ToBits
            | IntrinsicOp::ArrayRepeat
            | IntrinsicOp::StrLen
            | IntrinsicOp::StrAsBytes
            | IntrinsicOp::StrFromUtf8
            | IntrinsicOp::Assert
            | IntrinsicOp::VecNew
            | IntrinsicOp::VecLen
            | IntrinsicOp::VecPush
            | IntrinsicOp::VecPop
            | IntrinsicOp::Replace
            | IntrinsicOp::BoxNew
            | IntrinsicOp::Cast
            | IntrinsicOp::Clone
    )
}
fn operation(
    operations: &mut Vec<Operation>,
    name: &str,
    line: usize,
    column: usize,
    cleanup: bool,
) {
    for previous in operations.iter() {
        if previous.name == name && previous.cleanup == cleanup {
            return;
        }
    }
    operations.push(Operation {
        name: name.to_string(),
        line,
        column,
        cleanup,
    });
}
fn place_type(program: &Program, function: &Function, place: &Place) -> usize {
    let mut ty = function.locals[place.local].ty;
    for projection in &place.projections {
        ty = match (projection, &program.types[ty].kind) {
            (Projection::Deref, TypeKind::Ref { inner, .. } | TypeKind::Box { inner }) => *inner,
            (Projection::Field(index), TypeKind::Struct { fields, .. }) => fields[*index].ty,
            (Projection::VariantField(variant, index), TypeKind::Enum { variants, .. }) => {
                variants[*variant].fields[*index].ty
            }
            (
                Projection::Index(_),
                TypeKind::Array { inner, .. }
                | TypeKind::Vec { inner }
                | TypeKind::Slice { inner, .. },
            ) => *inner,
            _ => unreachable!(),
        };
    }
    ty
}
fn enqueue(
    function: usize,
    through_drop: bool,
    seen: &mut [bool],
    pending: &mut Vec<(usize, bool)>,
) {
    if !seen[function] {
        seen[function] = true;
        pending.push((function, through_drop));
    }
}
fn cleanup(
    program: &Program,
    first: usize,
    stamp: usize,
    visited: &mut [usize],
    seen_functions: &mut [bool],
    functions: &mut Vec<(usize, bool)>,
    operations: &mut Vec<Operation>,
    line: usize,
    column: usize,
) {
    let mut pending = vec![first];
    while let Some(ty) = pending.pop() {
        if visited[ty] == stamp {
            continue;
        }
        visited[ty] = stamp;
        for (owner, function) in &program.destructors {
            if *owner == ty {
                enqueue(*function, true, seen_functions, functions);
            }
        }
        match &program.types[ty].kind {
            TypeKind::Struct { fields, .. } => {
                for field in fields {
                    pending.push(field.ty);
                }
            }
            TypeKind::Enum { variants, .. } => {
                for variant in variants {
                    for field in &variant.fields {
                        pending.push(field.ty);
                    }
                }
            }
            TypeKind::Array { inner, len } => {
                if *len != 0 {
                    pending.push(*inner);
                }
            }
            TypeKind::Vec { inner } | TypeKind::Box { inner } => pending.push(*inner),
            TypeKind::File => operation(operations, "file_close", line, column, true),
            TypeKind::Directory => operation(operations, "directory_close", line, column, true),
            TypeKind::Listener => operation(operations, "listener_close", line, column, true),
            TypeKind::Child => {
                operation(operations, "child_terminate", line, column, true);
                operation(operations, "child_wait", line, column, true);
            }
            // A reference does not own its referent. In particular, a borrowed
            // aggregate must not make its custom destructor reachable.
            _ => {}
        }
    }
}
struct Block {
    start: usize,
    otherwise: Option<usize>,
    is_loop: bool,
}
fn reachable(function: &Function) -> Vec<bool> {
    let count = function.body.len();
    let mut ends = vec![count; count];
    let mut alternatives = vec![count; count];
    let mut starts = vec![count; count];
    let mut loop_targets = vec![count; count];
    let mut blocks: Vec<Block> = Vec::new();
    let mut loops: Vec<usize> = Vec::new();
    for (index, instruction) in function.body.iter().enumerate() {
        match instruction.op {
            Op::If { .. } => blocks.push(Block {
                start: index,
                otherwise: None,
                is_loop: false,
            }),
            Op::Loop => {
                blocks.push(Block {
                    start: index,
                    otherwise: None,
                    is_loop: true,
                });
                loops.push(index);
            }
            Op::Else => {
                let last = blocks.len() - 1;
                blocks[last].otherwise = Some(index);
            }
            Op::End => {
                let block = blocks.pop().unwrap();
                starts[index] = block.start;
                ends[block.start] = index;
                alternatives[block.start] = block.otherwise.unwrap_or(index);
                if let Some(otherwise) = block.otherwise {
                    ends[otherwise] = index;
                }
                if block.is_loop {
                    loops.pop();
                }
            }
            Op::Break | Op::Continue => loop_targets[index] = loops[loops.len() - 1],
            Op::BreakTo { depth } | Op::ContinueTo { depth } => {
                loop_targets[index] = loops[loops.len() - depth - 1]
            }
            _ => {}
        }
    }
    let mut seen = vec![false; count];
    let mut pending = vec![0usize];
    while let Some(index) = pending.pop() {
        if index == count || seen[index] {
            continue;
        }
        seen[index] = true;
        match function.body[index].op {
            Op::Return { .. } | Op::Trap { .. } => {}
            Op::If { .. } => {
                pending.push(index + 1);
                pending.push(alternatives[index] + 1);
            }
            Op::Else => pending.push(ends[index] + 1),
            Op::End if matches!(function.body[starts[index]].op, Op::Loop) => {
                pending.push(starts[index] + 1)
            }
            Op::Break | Op::BreakTo { .. } => pending.push(ends[loop_targets[index]] + 1),
            Op::Continue | Op::ContinueTo { .. } => pending.push(loop_targets[index] + 1),
            _ => pending.push(index + 1),
        }
    }
    seen
}
pub fn analyze(checked: &CheckedProgram, entry: usize) -> Result<Report, String> {
    let manifest = manifest(checked, entry)?;
    let program = checked.program();
    let mut seen = vec![false; program.functions.len()];
    let mut visited_types = vec![0usize; program.types.len()];
    let mut pending: Vec<(usize, bool)> = Vec::new();
    enqueue(entry, false, &mut seen, &mut pending);
    let mut functions: Vec<ReachableFunction> = Vec::new();
    let mut next = 0usize;
    while next < pending.len() {
        let (index, through_drop) = pending[next];
        next += 1;
        let function = &program.functions[index];
        let reachable = reachable(function);
        let mut operations: Vec<Operation> = Vec::new();
        for (position, instruction) in function.body.iter().enumerate() {
            if !reachable[position] {
                continue;
            }
            match &instruction.op {
                Op::Call { function, .. } => enqueue(*function, false, &mut seen, &mut pending),
                Op::Intrinsic { name, .. } if effect(*name) => operation(
                    &mut operations,
                    name.as_str(),
                    instruction.line,
                    instruction.column,
                    false,
                ),
                Op::Drop { place } | Op::DropIf { place } => cleanup(
                    program,
                    place_type(program, function, place),
                    index + 1,
                    &mut visited_types,
                    &mut seen,
                    &mut pending,
                    &mut operations,
                    instruction.line,
                    instruction.column,
                ),
                _ => {}
            }
        }
        functions.push(ReachableFunction {
            function: index,
            through_drop,
            operations,
        });
    }
    Ok(Report {
        entry,
        manifest,
        functions,
    })
}
fn name(text: &str) -> String {
    let mut out = String::new();
    for character in text.chars() {
        match character {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\\' => out.push_str("\\\\"),
            _ if (character as u32) < 32
                || character as u32 == 127
                || character as u32 == 0x2028
                || character as u32 == 0x2029 =>
            {
                out.push_str(&format!("\\u{:04x}", character as u32));
            }
            _ => out.push(character),
        }
    }
    out
}
pub fn render(checked: &CheckedProgram, report: &Report) -> String {
    let program = checked.program();
    let entry = &program.functions[report.entry];
    let mut text = format!("entry {}\ndeclared authority:\n", name(&entry.name));
    if report.manifest.full_world {
        text.push_str(&format!(
            "  {}: World (implicit full authority)\n",
            name(&entry.locals[0].name)
        ));
    } else if report.manifest.requirements.is_empty() {
        text.push_str("  none\n");
    } else {
        for requirement in &report.manifest.requirements {
            let value = if requirement.capability == "Directory" {
                "=PATH"
            } else if requirement.capability == "Listener" {
                "=ADDRESS"
            } else {
                ""
            };
            text.push_str(&format!(
                "  {}: {} (--grant {}{value})\n",
                name(&requirement.name),
                requirement.capability,
                name(&requirement.name)
            ));
        }
    }
    text.push_str("reachable operations (conservative checked-IR branches and owned cleanup):\n");
    let mut any = false;
    for function in &report.functions {
        if function.operations.is_empty() {
            continue;
        }
        any = true;
        text.push_str(&format!(
            "  {}{}:\n",
            name(&program.functions[function.function].name),
            if function.through_drop {
                " [custom Drop]"
            } else {
                ""
            }
        ));
        for operation in &function.operations {
            text.push_str(&format!(
                "    {}{}\n",
                operation.name,
                if operation.cleanup {
                    " [structural cleanup]"
                } else {
                    ""
                }
            ));
        }
    }
    if !any {
        text.push_str("  none\n");
    }
    text.push_str(&format!(
        "reachable functions ({}; each listed once):\n",
        report.functions.len()
    ));
    for function in &report.functions {
        text.push_str(&format!(
            "  {}{}\n",
            name(&program.functions[function.function].name),
            if function.through_drop {
                " [custom Drop]"
            } else {
                ""
            }
        ));
    }
    text
}
