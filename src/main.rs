#![feature(if_let_guard)]
#![feature(let_chains)]
#![feature(unsize)]
#![feature(coerce_unsized)]
#![feature(pin_coerce_unsized_trait)]
#![recursion_limit = "256"]

use betac_runner::{parse_command_line_args, Response};

use std::io;

mod betac_ast;
mod betac_errors;
mod betac_parser;
mod betac_runner;
mod betac_tokenizer;
mod betac_util;

/// the main driver module
/// this does most of the heavy lifting in the main function
/// it has all the major functions for handling the parsing, cli, etc. \n
/// future functions will most likely be added
///
/// DRIVER TODOS:
///
///     when we stablize async in this project, switch all these types to async types
mod driver {
    use crate::betac_parser::{traits::Parse, GlobalParser};
    use crate::{betac_errors::EMITTER, betac_tokenizer};
    use std::io;
    use std::time::Instant;

    /// emits all the errors that have been collected throughout the process.
    ///
    /// currently, it's basically a NOOP, but it will eventually do something
    ///
    /// ARGS: \n
    ///     takes a mutable reference to a writer
    ///
    ///     TODO: change this to AsyncWrite once we stablize the type
    ///
    /// RETURNS:
    ///
    ///     this returns a result indicating whether any writes failed
    ///
    pub(super) fn cleanup<W>(w: &mut W) -> std::io::Result<()>
    where
        W: io::Write,
    {
        EMITTER.lock().unwrap().flush(w)?;
        Ok(())
    }

    /// prints a help list
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

    /// prints the current version
    /// right now, it will always write the string: "version: 0.0.1"
    pub(super) fn print_current_version<W>(writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        writeln!(writer, "version: {}", CURRENT_VERSION)
    }

    /// runs the compiler
    /// it also measures the amount of time it took and outputs it when finished
    pub(super) fn run<W>(w: &mut W, file_name: String) -> io::Result<()>
    where
        W: io::Write,
    {
        let start_time = Instant::now();
        let input = std::fs::read_to_string(&file_name)?;
        assert!(
            input.len() <= u16::MAX as usize,
            "File length must be less than {} bytes",
            u16::MAX
        );

        let iter = betac_tokenizer::run_tokenizer(&*input);

        let mut parser = GlobalParser::new(input.clone(), iter);

        while parser.next_expression() {}

        let now = start_time.elapsed();
        writeln!(w, "process finished in {}us", now.as_micros())?;
        Ok(())
    }

    pub(super) fn build<W>(writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        Ok(())
    }

    const CURRENT_VERSION: &str = "0.0.1";
}

/// TODOS:
/// 1. finish parser
/// 2. rewrite parser to be thread-safe
/// 4. DOCUMENTATION
fn main() -> io::Result<()> {
    let mut writer = io::stdout().lock();

    match parse_command_line_args()? {
        Response::Help => driver::print_help_list(&mut writer)?,
        Response::Version => driver::print_current_version(&mut writer)?,
        Response::Run { file_name } => driver::run(&mut writer, file_name)?,
        Response::Build => driver::build(&mut writer)?,
    }

    driver::cleanup(&mut writer)?;
    Ok(())
}
