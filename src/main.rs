mod ast;
mod semantic;

#[macro_use]
extern crate clap;

#[macro_use]
extern crate lalrpop_util;

lalrpop_mod!(pub pascal);

use std::fs;

fn parse(filename: &str) {
    let contents = fs::read_to_string(filename).expect("Something went wrong reading the file");
    match pascal::programParser::new().parse(&contents) {
        Ok(s) => println!("{:?}", s),
        Err(e) => println!("{:?}", e),
    }
}

fn symbols(filename: &str) {
    let contents = fs::read_to_string(filename).expect("Something went wrong reading the file");
    match pascal::programParser::new().parse(&contents) {
        Ok(s) => println!("{:?}", semantic::get_symbols(s)),
        Err(e) => println!("{:?}", e),
    }
}

fn main() {
    use clap::App;
    let yaml = load_yaml!("cli.yml");
    let m = App::from_yaml(yaml).get_matches();
    match m.subcommand() {
        ("parse", Some(parse_matches)) => parse(parse_matches.value_of("input").unwrap()),
        ("symbols", Some(symbols_matches)) => symbols(symbols_matches.value_of("input").unwrap()),
        _ => println!("Unknown subcommand"),
    }
}
