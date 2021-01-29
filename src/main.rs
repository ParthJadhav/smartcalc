extern crate libsmartcalc;

use std::panic;
use libsmartcalc::executer::execute;

fn main() {
    let test_data = r"erhan = 3.".to_string();

    println!("{:?}", execute(&test_data, &"en".to_string()));
}

/*
cd libsmartcalc
cargo build --target wasm32-unknown-unknown --release
wasm-pack build
cd ../www/
npm run start
*/
