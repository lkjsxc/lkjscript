//! Conditional cleanup state belongs to a frame, never to an aggregate layout.
use super::*;

pub(super) struct Ownership {
    paths: Vec<Place>,
    offsets: Vec<i64>,
    by_local: Vec<Vec<usize>>,
    pub depth: usize,
}
fn prefix(parent: &Place, child: &Place) -> bool {
    if parent.local != child.local || parent.projections.len() > child.projections.len() {
        return false;
    }
    for index in 0..parent.projections.len() {
        if parent.projections[index] != child.projections[index] {
            return false;
        }
    }
    true
}
impl Ownership {
    pub(super) fn new(function: &Function) -> Self {
        let mut places: BTreeSet<Place> = BTreeSet::new();
        for instruction in &function.body {
            match &instruction.op {
                Op::Read {
                    place,
                    moving: true,
                    ..
                } if !place.projections.is_empty() => {
                    places.insert(place.clone());
                }
                Op::DropIf { place } => {
                    places.insert(place.clone());
                }
                Op::Drop { place } if !place.projections.is_empty() => {
                    places.insert(place.clone());
                }
                _ => {}
            }
        }
        let mut paths: Vec<Place> = Vec::new();
        let mut by_local: Vec<Vec<usize>> = vec![Vec::new(); function.locals.len()];
        let mut depth: usize = 0;
        for place in &places {
            depth = depth.max(place.projections.len());
            by_local[place.local].push(paths.len());
            paths.push(place.clone());
        }
        Self {
            offsets: vec![0; paths.len()],
            paths,
            by_local,
            depth,
        }
    }
    pub(super) fn allocate(&mut self, frame: &mut u64, site: Span) -> Result<(), Diagnostic> {
        for index in 0..self.offsets.len() {
            *frame = match frame.checked_add(1) {
                Some(value) => value,
                None => {
                    return Err(Diagnostic::new(
                        site,
                        "E_EMIT",
                        "native flag extent exceeds the address representation",
                    ));
                }
            };
            if *frame > 2147483647 {
                return Err(Diagnostic::new(
                    site,
                    "E_EMIT",
                    "native frame exceeds signed 32-bit displacement representation",
                ));
            }
            self.offsets[index] = -(*frame as i64);
        }
        Ok(())
    }
    pub(super) fn initialize(&self, a: &mut Assembler, parameters: usize) {
        a.immediate(R11, 0);
        for offset in &self.offsets {
            a.store_width(BP, *offset, R11, 1);
        }
        for local in 0..parameters {
            self.change(a, &Place::local(local), true);
        }
    }
    pub(super) fn change(&self, a: &mut Assembler, place: &Place, present: bool) {
        let mut emitted = false;
        for index in &self.by_local[place.local] {
            if prefix(place, &self.paths[*index]) {
                if !emitted {
                    a.immediate(R11, present as u64);
                    emitted = true;
                }
                a.store_width(BP, self.offsets[*index], R11, 1);
            }
        }
    }
    pub(super) fn flag(&self, place: &Place) -> Option<i64> {
        for index in &self.by_local[place.local] {
            if self.paths[*index] == *place {
                return Some(self.offsets[*index]);
            }
        }
        None
    }
    pub(super) fn descendants(&self, place: &Place) -> bool {
        for index in &self.by_local[place.local] {
            if self.paths[*index].projections.len() > place.projections.len()
                && prefix(place, &self.paths[*index])
            {
                return true;
            }
        }
        false
    }
}
