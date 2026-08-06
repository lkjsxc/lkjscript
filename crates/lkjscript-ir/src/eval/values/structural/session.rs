use lkjscript_core::{StructuralValueRuntime, StructuralValueRuntimeLimits};

use crate::{Constant, InstructionKind, Program, SsaType};

use super::{AggregateMode, ClosureReconstructor, StaticStringArtifact};

#[derive(Debug)]
pub(crate) struct EvaluatorStructuralSession {
    pub(crate) runtime: StructuralValueRuntime,
    static_strings: Vec<StaticStringArtifact>,
    static_symbols: Vec<Box<str>>,
}

impl EvaluatorStructuralSession {
    pub(crate) fn new(
        program: &Program,
        limits: StructuralValueRuntimeLimits,
    ) -> Result<Self, String> {
        preflight(program, limits)?;
        Ok(Self {
            runtime: StructuralValueRuntime::new(limits).map_err(|error| error.to_string())?,
            static_strings: collect_static_strings(program)?,
            static_symbols: collect_static_symbols(program)?,
        })
    }

    pub(crate) fn static_string(&self, identity: u64) -> Result<&str, String> {
        self.static_strings
            .get(
                usize::try_from(identity)
                    .map_err(|_| "evaluator static string index exceeds host usize")?,
            )
            .filter(|artifact| artifact.identity == identity)
            .map(|artifact| artifact.text.as_ref())
            .ok_or_else(|| "stale evaluator static string artifact".into())
    }

    pub(crate) fn static_string_identity(&self, text: &str) -> Result<u64, String> {
        self.static_strings
            .iter()
            .find(|artifact| artifact.text.as_ref() == text)
            .map(|artifact| artifact.identity)
            .ok_or_else(|| "evaluator static string artifact table mismatch".into())
    }

    pub(crate) fn static_string_count(&self) -> usize {
        self.static_strings.len()
    }

    pub(crate) fn static_symbol_identity(&self, symbol: &str) -> Result<u64, String> {
        self.static_symbols
            .iter()
            .position(|known| known.as_ref() == symbol)
            .and_then(|index| u64::try_from(index).ok())
            .ok_or_else(|| "evaluator static symbol table mismatch".into())
    }

    pub(crate) fn static_symbol(&self, identity: u64) -> Result<&str, String> {
        self.static_symbols
            .get(
                usize::try_from(identity)
                    .map_err(|_| "evaluator static symbol index exceeds host usize")?,
            )
            .map(AsRef::as_ref)
            .ok_or_else(|| "stale evaluator static symbol artifact".into())
    }
}

pub(crate) fn aggregate_mode(
    program: &Program,
    limits: StructuralValueRuntimeLimits,
    ty: &SsaType,
) -> Result<AggregateMode, String> {
    if let SsaType::Product(product) = ty {
        if program
            .region_products
            .iter()
            .any(|metadata| metadata.product == *product)
        {
            return Ok(AggregateMode::Region);
        }
    }
    let mode = ClosureReconstructor::new(program, limits.max_tree_nodes, limits.max_tree_depth)
        .aggregate_mode(ty);
    if !structural_eligible(program, ty) {
        return Ok(match mode {
            Ok(AggregateMode::ResourceAdapter) => AggregateMode::ResourceAdapter,
            _ => AggregateMode::Legacy,
        });
    }
    mode
}

pub(crate) fn structural_eligible(program: &Program, ty: &SsaType) -> bool {
    use crate::StructuralValueCategory::{Destination, Owner, View};

    let Some(type_id) = program.memory.type_for(ty).map(|item| item.id) else {
        return false;
    };
    [Owner, View, Destination].into_iter().all(|category| {
        program
            .memory
            .representations
            .iter()
            .any(|item| item.type_id == type_id && item.category == category)
    })
}

fn preflight(program: &Program, limits: StructuralValueRuntimeLimits) -> Result<(), String> {
    for instruction in program
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .flat_map(|block| &block.instructions)
    {
        if matches!(
            instruction.kind,
            InstructionKind::ProductValue { .. }
                | InstructionKind::WithProductField { .. }
                | InstructionKind::EnumValue { .. }
        ) {
            aggregate_mode(program, limits, &instruction.ty)?;
        }
    }
    Ok(())
}

fn collect_static_strings(program: &Program) -> Result<Vec<StaticStringArtifact>, String> {
    let mut output = Vec::<StaticStringArtifact>::new();
    for text in program
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .flat_map(|block| &block.instructions)
        .filter_map(|instruction| match &instruction.kind {
            InstructionKind::Constant(Constant::Str(text)) => Some(text.as_str()),
            _ => None,
        })
    {
        if output.iter().any(|artifact| artifact.text.as_ref() == text) {
            continue;
        }
        let identity = u64::try_from(output.len())
            .map_err(|_| "evaluator static string artifact identity exceeds u64".to_owned())?;
        output
            .try_reserve(1)
            .map_err(|_| "evaluator static string artifact allocation failed".to_owned())?;
        output.push(StaticStringArtifact {
            identity,
            text: text.into(),
        });
    }
    Ok(output)
}

fn collect_static_symbols(program: &Program) -> Result<Vec<Box<str>>, String> {
    let mut output = Vec::<Box<str>>::new();
    for symbol in program
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .flat_map(|block| &block.instructions)
        .filter_map(|instruction| match &instruction.kind {
            InstructionKind::Constant(Constant::Symbol(symbol)) => Some(symbol.as_str()),
            _ => None,
        })
    {
        if !output.iter().any(|known| known.as_ref() == symbol) {
            output
                .try_reserve(1)
                .map_err(|_| "evaluator static symbol allocation failed".to_owned())?;
            output.push(symbol.into());
        }
    }
    Ok(output)
}
