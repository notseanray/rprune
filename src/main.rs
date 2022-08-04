mod chunk;
use std::env;

use chunk::World;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let world = World::new(&args[0]);
    world.run(args[1].parse().unwrap()).expect("failed to run");
}
