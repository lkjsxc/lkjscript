struct M { out: Stdout, b: u8 }
impl Drop for M { fn drop(&mut self) { self.out.write(&[self.b]); } }
struct P { a: M, b: M }
fn tick(world: &mut World, byte: u8) { let mut out = world.stdout(); out.write(&[byte]); }
fn make(world: &mut World, byte: u8) -> M { tick(world, byte+100); M { out: world.stdout(), b: byte } }
fn option_input(world:&mut World,yes:bool,b:u8)->Option<M>{tick(world,90u8);if yes{Some(make(world,b))}else{None}}
fn result_input(world:&mut World,yes:bool,b:u8)->Result<M,M>{tick(world,91u8);if yes{Ok(make(world,b))}else{Err(make(world,b))}}

pub fn main(mut world:World) {
{let answer=option_input(&mut world,true,2u8).filter({tick(&mut world,79u8);let capture=make(&mut world,19u8);let mut out:Stdout=world.stdout();move|value:&M|{out.write(&[99u8]);drop(capture);value.b==3u8}});tick(&mut world,70u8);drop(answer);}{let answer=option_input(&mut world,false,2u8).filter({tick(&mut world,79u8);let capture=make(&mut world,19u8);let mut out:Stdout=world.stdout();move|value:&M|{out.write(&[99u8]);drop(capture);value.b==3u8}});tick(&mut world,70u8);drop(answer);}{let answer=result_input(&mut world,true,2u8).map_err({tick(&mut world,79u8);let capture=make(&mut world,19u8);let mut out:Stdout=world.stdout();move|value:M|{out.write(&[99u8]);drop(capture);value}});tick(&mut world,70u8);drop(answer);}{let answer=result_input(&mut world,false,2u8).map_err({tick(&mut world,79u8);let capture=make(&mut world,19u8);let mut out:Stdout=world.stdout();move|value:M|{out.write(&[99u8]);drop(capture);value}});tick(&mut world,70u8);drop(answer);}
tick(&mut world,98u8);
}
