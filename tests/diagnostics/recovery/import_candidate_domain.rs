use missing::PossibleTrait;
struct Known {}
impl Known { fn value(&self) -> i64 { 42 } }
fn inherent(value: Known) -> i64 { value.value() + true }
fn absent(value: Known) -> i64 { value.unrelated() }
mod other {
    pub trait Present { fn supplied(&self) -> i64; }
    impl Present for i64 { fn supplied(&self) -> i64 { 42 } }
}
fn uncertain(value: i64) -> i64 { value.supplied() }
