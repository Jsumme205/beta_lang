#![feature(pattern)]
#![feature(let_chains)]
#![feature(box_into_inner)]
#![feature(allocator_api)]
#![feature(iterator_try_collect)]
#![feature(cell_update)]

use std::{collections::HashMap, fmt::Debug};

use betac_backend::{IrCodegen, WriteIr};
use betac_lexer::ast_types::context::{GlobalContext, PackageContext};
use betac_util::{session::Session, sso::OwnedYarn, Yarn};

pub const DEFAULT_PATTERN: [char; 2] = [';', ' '];

const TOKEN_PATTERNS: [betac_lexer::ast_types::Token<'static>; 1] =
    [betac_lexer::ast_types::Token::Semi];

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
    let mut package_ctx = PackageContext::init();
    let ctx = GlobalContext::init("dummy", &mut package_ctx);

    let input = yarn!("pub defun foo(x: Uint32) => Uint32 {{ x + y }}");
    let mut lexer = betac_lexer::Lexer::init(&input, &mut session);
    let expr = lexer.parse_next_expr(ctx).unwrap();
    println!("expr: {expr:#?}");
    //let next_expr = lexer.parse_next_expr(&mut ctx).unwrap();
    //println!("{next_expr:#?}");
    let mut codegen = IrCodegen::init();
    let _ = expr.lower(&mut codegen);

    println!("result:\n{}", codegen.as_str());

    //out_file.write_all(&out_buf)?;
    Ok(())
}
