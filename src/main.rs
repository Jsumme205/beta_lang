#![feature(if_let_guard)]

mod betac_parser;
mod betac_tokenizer;
mod betac_util;

fn main() {
    let input = "let x: Int64 => 0;";
    let mut tokenizer = betac_tokenizer::Tokenizer::new(input);
    let token = tokenizer.advance_token();
    let next = tokenizer.advance_token();
    println!("token: {token:?}, next: {next:?}");
}
