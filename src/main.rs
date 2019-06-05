#![recursion_limit = "256"]
mod ast;
mod codegen;
mod lexer;
mod semantic;
mod token;

#[macro_use]
extern crate clap;

#[macro_use]
extern crate lalrpop_util;

lalrpop_mod!(pub pascal);

use std::fs;

fn lex(filename: &str) {
    let contents = fs::read_to_string(filename).expect("Something went wrong reading the file");
    let mut lexer = lexer::Lexer::new(&contents);
    for l in &mut lexer {
        match l {
            Ok(x) => println!("{:?}", x),
            Err(e) => println!("err {:?}", e),
        }
    }

    if contents.chars().count() > lexer.pos {
        println!(
            "Not all input was read\n{}/{}",
            contents.chars().count(),
            lexer.pos
        );
    }
}

fn parse(filename: &str) {
    let contents = fs::read_to_string(filename).expect("Something went wrong reading the file");
    match pascal::programParser::new().parse(lexer::Lexer::new(&contents)) {
        Ok(s) => println!("{:#?}", s),
        Err(e) => println!("{:#?}", e),
    }
}

fn symbols(filename: &str) {
    let contents = fs::read_to_string(filename).expect("Something went wrong reading the file");
    match pascal::programParser::new().parse(lexer::Lexer::new(&contents)) {
        Ok(s) => println!("{:#?}", semantic::get_symbols(s)),
        Err(e) => println!("{:#?}", e),
    }
}

fn typecheck(filename: &str) {
    let contents = fs::read_to_string(filename).expect("Something went wrong reading the file");
    let program = pascal::programParser::new()
        .parse(lexer::Lexer::new(&contents))
        .unwrap();
    match semantic::get_symbols(program) {
        Ok(symbols) => println!("{:#?}", semantic::check_variable_types(symbols)),
        Err(es) => println!("Errors while building symbols table: {:#?}", es),
    }
}

fn codegen(filename: &str) {
    let contents = fs::read_to_string(filename).expect("Something went wrong reading the file");
    let program = pascal::programParser::new()
        .parse(lexer::Lexer::new(&contents))
        .unwrap();
    unsafe {
        let mut cg = codegen::Codegen::new(program.name.clone());
        match cg.run(program) {
            Ok(_) => cg.write_llvm_bitcode_to_file("out.ll"),
            Err(e) => println!("{}", e),
        }
    }
}

fn main() {
    use clap::App;
    let yaml = load_yaml!("cli.yml");
    let m = App::from_yaml(yaml).get_matches();
    match m.subcommand() {
        ("lex", Some(lex_matches)) => lex(lex_matches.value_of("input").unwrap()),
        ("parse", Some(parse_matches)) => parse(parse_matches.value_of("input").unwrap()),
        ("symbols", Some(symbols_matches)) => symbols(symbols_matches.value_of("input").unwrap()),
        ("types", Some(types_matches)) => typecheck(types_matches.value_of("input").unwrap()),
        ("codegen", Some(codegen_matches)) => codegen(codegen_matches.value_of("input").unwrap()),
        _ => println!("Unknown subcommand"),
    }
}
