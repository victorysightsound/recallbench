mod config;
mod datasets;
mod errors;
mod llm;
mod systems;
mod traits;
mod types;

fn main() {
    println!("recallbench v{}", env!("CARGO_PKG_VERSION"));
    println!("A universal benchmark harness for AI memory systems.");
}
