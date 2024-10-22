#![feature(pattern)]
#![feature(let_chains)]
#![feature(box_into_inner)]
#![feature(allocator_api)]
#![feature(iterator_try_collect)]
#![feature(iter_collect_into)]
#![feature(iter_array_chunks)]
#![feature(cell_update)]

use std::{collections::HashMap, fmt::Debug};

use betac_errors::Emitter;
use betac_lexer::{ast_types::context::GlobalContext, Lexer, SourceCodeReader};
use betac_util::{session::Session, Yarn};

pub const DEFAULT_PATTERN: [char; 2] = [';', ' '];

#[derive(Debug)]
pub struct Globals {
    pub id_start: Option<Yarn<'static>>,
    pub defined: HashMap<Yarn<'static>, Yarn<'static>>,
}

unsafe impl Send for Globals {}
unsafe impl Sync for Globals {}

impl Globals {
    pub fn new() -> Self {
        Self {
            id_start: None,
            defined: HashMap::new(),
        }
    }
}

mod betac_backend;
mod betac_errors;
mod betac_lexer;
mod betac_packer;
mod betac_pp;
mod betac_util;

fn main() -> betac_util::CompileResult<()> {
    let mut session = Session::builder().input("src/test.blp").build()?;
    let input = yarn!("pub defun foo(x: Int64, y: Int64) => Int64 {{ ret x + y; }}");
    let mut lexer = Lexer::init(&input, &mut session);
    let expr = lexer.parse_next_expr::<GlobalContext>(None);
    println!("expr: {:#?}", expr);
    lexer.drain()?;
    Ok(())
}
