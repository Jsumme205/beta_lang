use crate::{
    betac_lexer::ast_types::context::{Context, SymbolKind},
    betac_util::{session::BuildFxHasher, Yarn},
};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PackageKind {
    Internal,
    External,
}

#[derive(Debug, Clone)]
pub struct Package<'a> {
    name: Yarn<'a>,
    fully_qualified_name: Yarn<'a>,
    path: Yarn<'a>,
    symbols: HashMap<Yarn<'a>, Symbol<'a>, BuildFxHasher>,
    vis: Vis,
    kind: PackageKind,
}

impl<'a> Package<'a> {
    pub fn from_name_and_ctx(
        name: Yarn<'a>,
        path: Yarn<'a>,
        fully_qualified_name: Yarn<'a>,
        vis: Vis,
        kind: PackageKind,
        ctx: &dyn Context<'a>,
    ) -> Self {
        let symbols = ctx.symbols_in_context().clone();
        let symbols = symbols
            .into_iter()
            .map(|(k, v)| {
                let key_clone = k.clone();
                let vis = v.vis();
                (
                    k,
                    Symbol {
                        ident: key_clone,
                        symbol_kind: v,
                        vis,
                    },
                )
            })
            .collect::<HashMap<_, _, BuildFxHasher>>();
        Self {
            name,
            fully_qualified_name,
            path,
            symbols,
            vis,
            kind,
        }
    }

    pub fn name(&self) -> &Yarn<'a> {
        &self.name
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Vis {
    Public,
    Private,
    FilePub,
    PackPub,
}

#[derive(Debug, Clone)]
pub struct Symbol<'a> {
    ident: Yarn<'a>,
    symbol_kind: SymbolKind,
    vis: Vis,
}
