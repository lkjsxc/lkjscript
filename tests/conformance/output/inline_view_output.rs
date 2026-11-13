#[derive(Clone, Copy)]
struct Views<'a> { first: &'a [u8], second: &'a [u8] }
fn swap<'a>(views: Views<'a>) -> Views<'a> {
    Views { first: views.second, second: views.first }
}
fn forward<'a>(bytes: &'a [u8]) -> &'a [u8] { bytes }
pub fn main(mut world: World) {
    let first = vec![65u8, 66];
    let second = vec![99u8, 100, 101, 102, 103];
    let views = Views { first: &first, second: &second };
    let swapped = swap(views.clone());
    let mut output = world.stdout();
    output.write(forward(swapped.second));
    output.write(forward(swapped.first));
    let message = "λ";
    output.write(message.as_bytes());
}
