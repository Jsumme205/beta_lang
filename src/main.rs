#![feature(if_let_guard)]
#![feature(coerce_unsized)]

use std::io;

mod betac_ast;
mod betac_parser;
mod betac_tokenizer;
mod betac_util;

fn main() -> io::Result<()> {
    let input = std::fs::read_to_string("src/test.blp")?;
    let mut parser = betac_parser::Parser::new(&*input, betac_tokenizer::run_tokenizer(&*input));

    Ok(())
}
