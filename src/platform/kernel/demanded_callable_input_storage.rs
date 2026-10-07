//! Bound every retained operand and type vector before a shared input is exposed.
use super::*;

pub(super) fn reserve<R: CallableClosureRead + ?Sized>(
    analysis: &mut Analysis<'_, R>,
    arguments: &[TypeObjectDigest],
    operands: &[ImplementationOperand],
) -> Result<(), Diagnostic> {
    analysis.steps(arguments.len())?;
    analysis.reserve::<TypeObjectDigest>(arguments.len())?;
    analysis.reserve::<&[ImplementationOperand]>(1)?;
    let mut pending = vec![operands];
    while let Some(operands) = pending.pop() {
        analysis.tick()?;
        analysis.reserve::<ImplementationOperand>(operands.len())?;
        for operand in operands {
            analysis.tick()?;
            if let ImplementationOperand::Concrete {
                type_arguments,
                implementations,
                ..
            } = operand
            {
                analysis.steps(type_arguments.len())?;
                analysis.reserve::<TypeObjectDigest>(type_arguments.len())?;
                if !implementations.is_empty() {
                    analysis.reserve::<&[ImplementationOperand]>(1)?;
                    pending.push(implementations);
                }
            }
        }
    }
    Ok(())
}
