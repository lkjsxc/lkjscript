//! Scalar storage selection. Intervals preserve loop-carried values and addresses.
//! Caller-saved registers are used only across scalar instructions; three saved
//! registers also carry values through calls. This does not rewrite checked IR.
use super::*;

struct Intervals {
    first: Vec<usize>,
    last: Vec<usize>,
    address: Vec<bool>,
}
impl Intervals {
    fn access(&mut self, local: usize, point: usize, loops: &[usize], ends: &[usize]) {
        self.first[local] = self.first[local].min(point);
        self.last[local] = self.last[local].max(point);
        for start in loops {
            if self.first[local] <= *start * 2 {
                self.last[local] = self.last[local].max(ends[*start] * 2);
                break;
            }
        }
    }
    fn place(
        &mut self,
        place: &Place,
        point: usize,
        loops: &[usize],
        ends: &[usize],
        address: bool,
    ) {
        self.access(place.local, point, loops, ends);
        if address || !place.projections.is_empty() {
            self.address[place.local] = true;
        }
        for projection in &place.projections {
            if let Projection::Index(local) = projection {
                self.access(*local, point, loops, ends);
            }
        }
    }
}

pub(super) struct Registers {
    pub locals: Vec<Option<u8>>,
    pub saved: Vec<u8>,
}
impl Registers {
    pub fn new(program: &Program, function: &Function, layout: &Layouts) -> Self {
        let count = function.locals.len();
        let mut spans = Intervals {
            first: vec![!0usize; count],
            last: vec![0; count],
            address: vec![false; count],
        };
        for local in 0..function.params {
            spans.first[local] = 0;
        }
        let mut ends = vec![0usize; function.body.len()];
        let mut regions: Vec<(usize, bool)> = Vec::new();
        for index in 0..function.body.len() {
            match function.body[index].op {
                Op::If { .. } => regions.push((index, false)),
                Op::Loop => regions.push((index, true)),
                Op::End => {
                    if let Some((start, true)) = regions.pop() {
                        ends[start] = index + 1;
                    }
                }
                _ => {}
            }
        }
        let mut loops: Vec<usize> = Vec::new();
        let mut blocked = vec![0usize];
        for index in 0..function.body.len() {
            let op = &function.body[index].op;
            let read = index * 2;
            let write = read + 1;
            match op {
                Op::Loop => {
                    regions.push((index, true));
                    loops.push(index);
                }
                Op::If { .. } => regions.push((index, false)),
                Op::End => {
                    if let Some((_, true)) = regions.pop() {
                        loops.pop();
                    }
                }
                _ => {}
            }
            match op {
                Op::Int { dst, .. }
                | Op::Float { dst, .. }
                | Op::Unit { dst }
                | Op::Text { dst, .. } => spans.access(*dst, write, &loops, &ends),
                Op::Read { dst, place, .. } => {
                    spans.place(place, read, &loops, &ends, false);
                    spans.access(*dst, write, &loops, &ends);
                }
                Op::Borrow { dst, place, .. } | Op::SliceBorrow { dst, place, .. } => {
                    spans.place(place, read, &loops, &ends, true);
                    spans.access(*dst, write, &loops, &ends);
                }
                Op::ViewRange {
                    dst,
                    place,
                    start,
                    end,
                    ..
                } => {
                    spans.place(place, read, &loops, &ends, true);
                    spans.access(*start, read, &loops, &ends);
                    spans.access(*end, read, &loops, &ends);
                    spans.access(*dst, write, &loops, &ends);
                }
                Op::Assign { place, src } => {
                    spans.access(*src, read, &loops, &ends);
                    spans.place(
                        place,
                        if place.projections.is_empty() {
                            write
                        } else {
                            read
                        },
                        &loops,
                        &ends,
                        false,
                    );
                }
                Op::Drop { place } | Op::DropIf { place } => {
                    if layout.storage(function.locals[place.local].ty).cleanup
                        || !place.projections.is_empty()
                    {
                        spans.place(place, read, &loops, &ends, true);
                    }
                }
                Op::Unary { dst, src, .. } => {
                    spans.access(*src, read, &loops, &ends);
                    spans.access(*dst, write, &loops, &ends);
                }
                Op::Binary {
                    dst, left, right, ..
                } => {
                    spans.access(*left, read, &loops, &ends);
                    spans.access(*right, read, &loops, &ends);
                    spans.access(*dst, write, &loops, &ends);
                }
                Op::Call { dst, args, .. }
                | Op::Intrinsic { dst, args, .. }
                | Op::Array {
                    dst,
                    elements: args,
                }
                | Op::Struct { dst, fields: args }
                | Op::Variant {
                    dst, fields: args, ..
                } => {
                    for arg in args {
                        spans.access(*arg, read, &loops, &ends);
                    }
                    spans.access(*dst, write, &loops, &ends);
                }
                Op::Discriminant { dst, src } => {
                    spans.access(*src, read, &loops, &ends);
                    spans.access(*dst, write, &loops, &ends);
                }
                Op::TagTest { dst, place, .. } => {
                    spans.place(place, read, &loops, &ends, true);
                    spans.access(*dst, write, &loops, &ends);
                }
                Op::If { condition } => spans.access(*condition, read, &loops, &ends),
                Op::Return { src } => spans.access(*src, read, &loops, &ends),
                _ => {}
            }
            let scalar = match op {
                Op::Int { .. }
                | Op::Float { .. }
                | Op::Unit { .. }
                | Op::Unary { .. }
                | Op::Binary { .. }
                | Op::If { .. }
                | Op::Else
                | Op::Loop
                | Op::End
                | Op::Break
                | Op::Continue
                | Op::BreakTo { .. }
                | Op::ContinueTo { .. } => true,
                Op::Read { dst, place, .. } => {
                    place.projections.is_empty() && layout.storage(function.locals[*dst].ty).scalar
                }
                Op::Assign { place, src } => {
                    place.projections.is_empty() && layout.storage(function.locals[*src].ty).scalar
                }
                Op::Drop { place } | Op::DropIf { place } => {
                    place.projections.is_empty()
                        && !layout.storage(function.locals[place.local].ty).cleanup
                }
                Op::Return { src } => layout.storage(function.locals[*src].ty).scalar,
                Op::Intrinsic {
                    name: IntrinsicOp::Assert | IntrinsicOp::Cast,
                    ..
                } => true,
                _ => false,
            };
            blocked.push(blocked[index] + if scalar { 0 } else { 1 });
        }
        let mut order = Vec::new();
        for local in 0..count {
            let ty = function.locals[local].ty;
            if spans.first[local] != !0usize
                && !spans.address[local]
                && layout.storage(ty).scalar
                && !matches!(program.types[ty].kind, TypeKind::Unit)
            {
                order.push((spans.first[local], local));
            }
        }
        order.sort();
        let pool = [R8, R9, R10, DI, SI, B, R14, R15];
        let mut occupied: Vec<Option<usize>> = vec![None; pool.len()];
        let mut result = Self {
            locals: vec![None; count],
            saved: Vec::new(),
        };
        for (first, local) in order {
            let begin = first / 2 + first % 2;
            let end = (spans.last[local] / 2 + 1).min(function.body.len());
            let volatile =
                local >= function.params && (begin >= end || blocked[begin] == blocked[end]);
            for index in 0..pool.len() {
                let available = match occupied[index] {
                    Some(previous) => spans.last[previous] < first,
                    None => true,
                };
                if available && (index >= 5 || volatile) {
                    result.locals[local] = Some(pool[index]);
                    occupied[index] = Some(local);
                    if index >= 5 && !result.saved.contains(&pool[index]) {
                        result.saved.push(pool[index]);
                    }
                    break;
                }
            }
        }
        result
    }
}
