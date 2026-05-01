//! Tiny formatting helpers shared by the unterm-cli subcommand modules.

use serde_json::Value;

pub fn print_json(value: &Value) {
    match serde_json::to_string_pretty(value) {
        Ok(s) => println!("{}", s),
        Err(_) => println!("{}", value),
    }
}

pub fn print_kv(label: &str, value: &str) {
    println!("{}: {}", label, value);
}
