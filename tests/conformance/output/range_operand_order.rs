fn tick(world: &mut World, byte: u8) {
    let mut out = world.stdout();
    out.write(&[byte]);
}
fn low(world: &mut World) -> usize {
    tick(world, 83);
    1
}
fn high(world: &mut World) -> usize {
    tick(world, 69);
    3
}
fn array<'a>(world: &mut World, value: &'a [u8; 4]) -> &'a [u8; 4] {
    tick(world, 65);
    value
}
fn vector<'a>(world: &mut World, value: &'a Vec<u8>) -> &'a Vec<u8> {
    tick(world, 86);
    value
}
fn slice<'a>(world: &mut World, value: &'a [u8]) -> &'a [u8] {
    tick(world, 76);
    value
}
fn writable<'a>(world: &mut World, value: &'a mut [u8; 4]) -> &'a mut [u8; 4] {
    tick(world, 77);
    value
}
pub fn main(mut world: World) {
    let first = [65u8, 66, 67, 68];
    {
        let part = &array(&mut world, &first)[low(&mut world)..high(&mut world)];
        let mut out = world.stdout();
        out.write(part);
    }
    let second = vec![65u8, 66, 67, 68];
    {
        let part = &vector(&mut world, &second)[low(&mut world)..high(&mut world)];
        let mut out = world.stdout();
        out.write(part);
    }
    {
        let part = &slice(&mut world, &second)[low(&mut world)..high(&mut world)];
        let mut out = world.stdout();
        out.write(part);
    }
    let mut last = [65u8, 66, 67, 68];
    {
        let part = &mut writable(&mut world, &mut last)[low(&mut world)..high(&mut world)];
        part[0] = 88;
        part[1] = 89;
    }
    let mut out = world.stdout();
    out.write(&last);
}
