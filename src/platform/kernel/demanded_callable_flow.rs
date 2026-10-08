//! Backward closure from declaration type slots. Every constructor edge starts at
//! such a slot; therefore every expanding cycle is in this closure. Pure witness
//! forwarding outside it cannot grow a type. Keep exact paths, never shape aliases.
#[path = "demanded_callable_inputs.rs"]
mod inputs;
#[cfg(test)]
#[path = "demanded_callable_read_tests.rs"]
mod read_tests;
#[path = "demanded_callable_scopes.rs"]
mod scopes;
#[path = "demanded_callable_transfer.rs"]
mod transfer;

use super::*;
use inputs::{Incoming, Inputs};

struct Request {
    context: usize,
    path: Vec<usize>,
    slots: Vec<usize>,
}
struct Demands {
    requests: Vec<Request>,
    indexes: BTreeMap<(usize, Vec<usize>), usize>,
    pending: Vec<usize>,
    incoming: Vec<Vec<Incoming>>,
    scopes: scopes::Scopes,
}
impl Demands {
    fn request<R: CallableClosureRead + ?Sized>(
        &mut self,
        analysis: &mut Analysis<'_, R>,
        context: usize,
        path: Vec<usize>,
    ) -> Result<usize, Diagnostic> {
        analysis.tick()?;
        analysis.comparison_work(path.len() + 1, self.indexes.len())?;
        let key = (context, path);
        if let Some(index) = self.indexes.get(&key) {
            return Ok(*index);
        }
        let width = self.width(analysis, context, &key.1)?;
        analysis.reserve::<Request>(1)?;
        analysis.reserve::<((usize, Vec<usize>), usize)>(1)?;
        analysis.reserve::<usize>(key.1.len())?;
        analysis.reserve::<usize>(width)?;
        analysis.reserve::<usize>(1)?;
        analysis.steps(key.1.len() + width)?;
        let slots = if key.1.is_empty() {
            analysis.contexts[context].slots[..width].to_vec()
        } else {
            analysis.reserve::<usize>(width)?;
            analysis.reserve::<(usize, usize)>(width)?;
            let mut slots = Vec::new();
            for _ in 0..width {
                analysis.tick()?;
                let slot = analysis.slot_owners.len();
                let ordinal = analysis.contexts[context].slots.len();
                analysis.slot_owners.push((context, ordinal));
                analysis.contexts[context].slots.push(slot);
                slots.push(slot);
            }
            slots
        };
        let index = self.requests.len();
        self.requests.push(Request {
            context,
            path: key.1.clone(),
            slots,
        });
        self.indexes.insert(key, index);
        // A zero-parameter witness may have typed children. Their own exact paths
        // are requested independently; expanding all children here defeats demand.
        if width != 0 {
            self.pending.push(index);
        }
        Ok(index)
    }
    fn width<R: CallableClosureRead + ?Sized>(
        &self,
        analysis: &mut Analysis<'_, R>,
        context: usize,
        path: &[usize],
    ) -> Result<usize, Diagnostic> {
        let Some((root, suffix)) = path.split_first() else {
            return Ok(analysis.contexts[context].parameters.len());
        };
        let mut selected = *analysis.contexts[context]
            .key
            .implementations
            .get(*root)
            .ok_or_else(missing_path)?;
        for child in suffix {
            analysis.tick()?;
            let Selection::Scheme { prerequisites, .. } = &analysis.selections[selected.0].shape
            else {
                return Err(missing_path());
            };
            selected = *prerequisites.get(*child).ok_or_else(missing_path)?;
        }
        if selected == UNKNOWN_SELECTION {
            return Err(missing_path());
        }
        Ok(analysis.selections[selected.0].parameters.len())
    }
    fn copy<R: CallableClosureRead + ?Sized>(
        &self,
        analysis: &mut Analysis<'_, R>,
        index: usize,
    ) -> Result<Request, Diagnostic> {
        let request = &self.requests[index];
        analysis.steps(request.path.len() + request.slots.len())?;
        analysis.reserve::<Request>(1)?;
        analysis.reserve::<usize>(request.path.len() + request.slots.len())?;
        Ok(Request {
            context: request.context,
            path: request.path.clone(),
            slots: request.slots.clone(),
        })
    }
    fn forward<R: CallableClosureRead + ?Sized>(
        &mut self,
        analysis: &mut Analysis<'_, R>,
        call: CallSite,
        target: &Request,
        ordinal: usize,
        suffix: &[usize],
    ) -> Result<(), Diagnostic> {
        analysis.steps(suffix.len() + 1)?;
        analysis.reserve::<usize>(suffix.len() + 1)?;
        let mut path = vec![ordinal];
        path.extend_from_slice(suffix);
        let source = self.request(analysis, call.from, path)?;
        let slots = &self.requests[source].slots;
        if slots.len() != target.slots.len() {
            return Err(semantic(
                "kernel_callable_flow_arity",
                "demanded witness scheme arity differs",
            ));
        }
        let owner = analysis.contexts[call.from].key.owner;
        for (argument, (from, to)) in slots.iter().zip(&target.slots).enumerate() {
            analysis.edge(*from, *to, owner, call.expression, argument, Vec::new())?;
        }
        Ok(())
    }
}
fn missing_path() -> Diagnostic {
    semantic(
        "kernel_callable_flow_context",
        "demanded provenance has no exact selected witness path",
    )
}

pub(super) fn connect<R: CallableClosureRead + ?Sized>(
    analysis: &mut Analysis<'_, R>,
    components: &[usize],
) -> Result<(), Diagnostic> {
    #[cfg(test)]
    let _phase = read_tests::Phase::enter();
    analysis.reserve::<Vec<Incoming>>(analysis.contexts.len())?;
    let mut demands = Demands {
        requests: Vec::new(),
        indexes: BTreeMap::new(),
        pending: Vec::new(),
        scopes: scopes::Scopes::default(),
        incoming: (0..analysis.contexts.len()).map(|_| Vec::new()).collect(),
    };
    for index in 0..analysis.calls.len() {
        analysis.tick()?;
        let call = analysis.calls[index];
        if components[call.from] == components[call.to] {
            analysis.reserve::<Incoming>(1)?;
            demands.incoming[call.to].push(Incoming::new(call));
        }
    }
    for context in 0..analysis.contexts.len() {
        analysis.tick()?;
        if !demands.incoming[context].is_empty()
            && !analysis.contexts[context].parameters.is_empty()
        {
            demands.request(analysis, context, Vec::new())?;
        }
    }
    while let Some(index) = demands.pending.pop() {
        analysis.tick()?;
        let request = demands.copy(analysis, index)?;
        for index in 0..demands.incoming[request.context].len() {
            // Cache hits retain cancellation/work admission and exact path transfer.
            analysis.tick()?;
            let incoming = &mut demands.incoming[request.context][index];
            let call = incoming.call;
            let inputs = incoming.resolve(|call| inputs::load(analysis, call))?;
            transfer::connect(analysis, &mut demands, &request, call, &inputs)?;
        }
    }
    Ok(())
}
