use crate::betac_lexer::ast_types::context::Context;
use crate::betac_util::session::BuildFxHasher;
use crate::betac_util::Yarn;
use std::collections::HashMap;

pub mod pack;

pub struct PackageLoader<'pack> {
    packages_in_scope: HashMap<Yarn<'pack>, pack::Package<'pack>, BuildFxHasher>,
}

impl<'pack> PackageLoader<'pack> {
    pub fn init() -> Self {
        Self {
            packages_in_scope: HashMap::default(),
        }
    }

    pub fn add_package_to_scope(&mut self, package: pack::Package<'pack>) {
        let name = package.name().clone();
        self.packages_in_scope.insert(name, package);
    }

    pub fn add_symbols_to_context(&mut self, ctx: &mut dyn Context<'pack>) {}
}
