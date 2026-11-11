struct Cell{value:i64}
struct Pair{left:Cell,right:Cell}
fn choose<'a>(owner:&'a Pair)->&'a Cell{&owner.right}
fn probe()->i64{
    let mut owner=Pair{left:Cell{value:1},right:Cell{value:2}};
    let selected=choose(&owner);
    let child=&selected.value;
    drop(selected);
    owner.right.value=9;
    *child
}
