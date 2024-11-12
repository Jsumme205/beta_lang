#![feature(if_let_guard)]
#![feature(update_panic_count)]

use std::io;

use betac_errors::EMITTER;

mod betac_ast;
mod betac_errors;
mod betac_parser;
mod betac_tokenizer;
mod betac_util;

fn cleanup() -> std::io::Result<()> {
    EMITTER
        .lock()
        .unwrap()
        .flush(&mut std::io::stdout().lock())?;
    Ok(())
}

fn main() -> io::Result<()> {
    let input = std::fs::read_to_string("src/test.blp")?;
    let mut parser = betac_parser::Parser::new(&*input, betac_tokenizer::run_tokenizer(&*input));
    let expr = parser.parse_next_expr();

    println!("expr: {expr:#?}");
    cleanup()?;
    Ok(())
}
