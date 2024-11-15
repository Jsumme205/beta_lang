#![feature(if_let_guard)]
#![recursion_limit = "256"]

use betac_runner::{parse_command_line_args, Response};

use std::io;

mod betac_ast;
mod betac_errors;
mod betac_parser;
mod betac_runner;
mod betac_tokenizer;
mod betac_util;

mod driver {
    use crate::{
        betac_ast::AstList,
        betac_errors::EMITTER,
        betac_parser::{self, Parse},
        betac_tokenizer,
    };
    use std::io;

    pub(super) fn cleanup<W>(w: &mut W) -> std::io::Result<()>
    where
        W: io::Write,
    {
        EMITTER.lock().unwrap().flush(w)?;
        Ok(())
    }

    pub(super) fn print_help_list<W>(writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        writeln!(writer, "USAGE: betac INPUT [OPTIONS]")?;
        writeln!(writer, "-h, --help: Display this message")?;
        writeln!(writer, "-v, --version: Display current version")?;
        writeln!(writer, "compile FILE [OPTIONS]: compiles FILE with OPTIONS")?;
        writeln!(
            writer,
            "build: builds all files in current directory and links them"
        )?;
        Ok(())
    }

    pub(super) fn print_current_version<W>(writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        writeln!(writer, "version: {}", CURRENT_VERSION)
    }

    pub(super) fn runner<W>(_w: &mut W, file_name: String) -> io::Result<()>
    where
        W: io::Write,
    {
        let input = std::fs::read_to_string(&file_name)?;
        assert!(
            input.len() <= u16::MAX as usize,
            "File length must be less than {} bytes",
            u16::MAX
        );
        let mut parser =
            betac_parser::Parser::new(&*input, betac_tokenizer::run_tokenizer(&*input));
        while !parser.parse_next_expr() {}
        let (list, _) = parser.complete();
        println!("list: {list:#?}");
        Ok(())
    }

    const CURRENT_VERSION: &str = "0.0.1";
}

/// TODOS:
/// 1. finish parser
/// 2. rewrite parser to be thread-safe
/// 3. change most `u32`'s to `u16`'s, we are rolling with 65535 bytes (which should be enough)
fn main() -> io::Result<()> {
    let mut writer = io::stdout().lock();

    match parse_command_line_args()? {
        Response::Help => driver::print_help_list(&mut writer)?,
        Response::Version => driver::print_current_version(&mut writer)?,
        Response::Run { file_name } => driver::runner(&mut writer, file_name)?,
    }

    driver::cleanup(&mut writer)?;
    Ok(())
}
