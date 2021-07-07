use xapian_rusty::{Database, Document};

// export CARGO_MANIFEST_DIR=/Users/ssosik/workspace/xapian-rusty
// export CARGO_TARGET_DIR=target/foo
// cargo run

fn main() {
    let db = Database::new();
    println!("Hello, world!");
}
