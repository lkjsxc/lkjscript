use serde_json::{Value, json};

pub fn example() -> Value {
    json!({"roots":[10],"nodes":[
        {"id":10,"successors":[20,30]}, {"id":20,"successors":[40]},
        {"id":30,"successors":[40]}, {"id":40,"successors":[20]},
        {"id":50,"successors":[50]}, {"id":60,"successors":[]}]})
}
fn chain(length: usize, cycle: bool) -> Value {
    let identity = |position: usize| position as i64 * 13 - 2000;
    let nodes: Vec<_> = (0..length)
        .rev()
        .map(|position| {
            let successors = if position + 1 < length {
                vec![identity(position + 1)]
            } else if cycle {
                vec![identity(0)]
            } else {
                vec![]
            };
            json!({"id":identity(position),"successors":successors})
        })
        .collect();
    json!({"roots":[identity(0)],"nodes":nodes})
}
pub fn suite() -> Vec<Value> {
    let ids = [i64::MAX, -1, i64::MIN];
    let mut proposals = Vec::new();
    // Every directed graph on three fixed vertices, including all self-loops.
    for mask in 0..512_u32 {
        let nodes: Vec<_> = (0..3)
            .map(|source| {
                let mut successors: Vec<_> = (0..3)
                    .filter(|target| mask & (1 << (source * 3 + target)) != 0)
                    .map(|target| ids[target])
                    .collect();
                if mask & 1 != 0 {
                    successors.reverse();
                }
                json!({"id":ids[source],"successors":successors})
            })
            .collect();
        let mut roots: Vec<_> = (0..3)
            .filter(|position| mask & (1 << position) != 0)
            .map(|position| ids[position])
            .collect();
        if let Some(&first) = roots.first() {
            roots.push(first);
        }
        proposals.push(json!({"roots":roots,"nodes":nodes}));
    }
    proposals.push(json!({"roots":[],"nodes":[]}));
    proposals.push(example());
    proposals.push(chain(257, false));
    proposals.push(chain(257, true));
    let star: Vec<_> = (0..129)
        .map(|id| {
            json!({"id":id,"successors":
        if id == 0 { (1..129).collect::<Vec<_>>() } else { vec![0,0] }})
        })
        .collect();
    proposals.push(json!({"roots":[0,0],"nodes":star}));
    // Inclusive capacity boundaries, not just one-past-bound refusals.
    proposals.push(json!({"roots":[],"nodes":(0..4096).map(|id|
        json!({"id":id,"successors":[]})).collect::<Vec<_>>()}));
    proposals.push(json!({"roots":vec![-1;4096],"nodes":[{"id":-1,"successors":[]}]}));
    proposals.push(json!({"roots":[],"nodes":[{"id":i64::MIN,"successors":vec![i64::MIN;16384]}]}));
    let invalid = [
        json!({"roots":[90],"nodes":[{"id":2,"successors":[80]},{"id":2,"successors":[]}]}),
        json!({"roots":[90,80],"nodes":[{"id":1,"successors":[70]}]}),
        json!({"roots":[1],"nodes":[{"id":1,"successors":[]},{"id":2,"successors":[90,80]}]}),
        json!({"roots":[],"nodes":vec![json!({"id":1,"successors":[]});4097]}),
        json!({"roots":vec![1;4097],"nodes":[{"id":1,"successors":[]},{"id":1,"successors":[]}]}),
        json!({"roots":[9],"nodes":[{"id":1,"successors":vec![1;16385]},{"id":1,"successors":[]}]}),
    ];
    for proposal in invalid {
        proposals.push(proposal);
        proposals.push(example()); // Invalid input must not contaminate a later invocation.
    }
    proposals
}
