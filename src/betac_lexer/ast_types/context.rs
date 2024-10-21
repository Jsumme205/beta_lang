use crate::{
    betac_packer::pack::Vis,
    betac_util::{session::BuildFxHasher, Yarn},
};
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    rc::Rc,
    sync::RwLock,
};

use super::Ty;

type FxHashSet<T> = HashSet<T, BuildFxHasher>;
type FxHashMap<K, V> = HashMap<K, V>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SymbolKind {
    Assignment(Ty, Vis),
    Alias(Vis),
    Function(Ty, Vec<Ty>, Vis),
}

impl SymbolKind {
    pub fn vis(&self) -> Vis {
        match self {
            Self::Assignment(_, v) => *v,
            Self::Alias(v) => *v,
            Self::Function(_, _, v) => *v,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContextKind {
    Global,
    Object,
    Composition,
    Function,
    Block,
}

pub trait Context<'scope> {
    fn symbol_is_in_scope(&self, sym: &Yarn<'_>) -> bool;

    fn kind_for_symbol(&self, sym: &Yarn<'_>) -> Option<SymbolKind>;

    fn enter_symbol_into_scope(&mut self, sym: Yarn<'scope>, kind: SymbolKind) -> bool;

    fn kind(&self) -> ContextKind;

    fn new_child_context<'child>(
        &'child mut self,
        kind: ContextKind,
    ) -> Rc<RwLock<dyn Context<'child> + 'child>>;

    fn parent_context_kind(&self) -> ContextKind;

    fn symbols_in_context(&self) -> &FxHashMap<Yarn<'scope>, SymbolKind>;

    fn symbols_in_context_mut(&mut self) -> &mut FxHashMap<Yarn<'scope>, SymbolKind>;

    fn symbols_in_parent_ctx(&self) -> &FxHashMap<Yarn<'_>, SymbolKind>;

    fn enter_symbol_into_parent_scope(&mut self, sym: Yarn<'static>, kind: SymbolKind) -> bool;
}

#[derive(Debug)]
pub struct PackageContext {
    global_contexts: HashMap<Yarn<'static>, Rc<RwLock<GlobalContext>>>,
}

impl PackageContext {
    pub fn init() -> Self {
        Self {
            global_contexts: Default::default(),
        }
    }

    pub fn get_global_context(&self, name: &'static str) -> Option<Rc<RwLock<GlobalContext>>> {
        self.global_contexts
            .get(&Yarn::constant(name))
            .map(|rc| rc.clone())
    }
}

#[derive(Debug)]
pub struct GlobalContext {
    global_symbols: FxHashMap<Yarn<'static>, SymbolKind>,
}

impl GlobalContext {
    pub fn init(name: &'static str, ctx: &mut PackageContext) -> &'static str {
        let this = Self {
            global_symbols: Default::default(),
        };
        let this = Rc::new(RwLock::new(this));
        ctx.global_contexts.insert(Yarn::constant(name), this);
        name
    }

    pub fn global_symbols(&self) -> &FxHashMap<Yarn<'static>, SymbolKind> {
        &self.global_symbols
    }
}

impl Context<'static> for GlobalContext {
    fn symbol_is_in_scope(&self, sym: &Yarn<'_>) -> bool {
        self.global_symbols.contains_key(sym)
    }

    fn enter_symbol_into_scope(&mut self, sym: Yarn<'static>, kind: SymbolKind) -> bool {
        !self.global_symbols.insert(sym, kind).is_some()
    }

    fn kind(&self) -> ContextKind {
        ContextKind::Global
    }

    fn new_child_context<'child>(
        &'child mut self,
        kind: ContextKind,
    ) -> Rc<RwLock<dyn Context<'child> + 'child>> {
        Rc::new(RwLock::new(SubContext::init(self, kind)))
    }

    fn parent_context_kind(&self) -> ContextKind {
        ContextKind::Global
    }

    fn kind_for_symbol(&self, sym: &Yarn<'_>) -> Option<SymbolKind> {
        self.global_symbols.get(sym).map(|opt| opt.clone())
    }

    fn symbols_in_context(&self) -> &FxHashMap<Yarn<'static>, SymbolKind> {
        self.global_symbols()
    }

    fn symbols_in_parent_ctx(&self) -> &FxHashMap<Yarn<'_>, SymbolKind> {
        self.global_symbols()
    }

    fn symbols_in_context_mut(&mut self) -> &mut FxHashMap<Yarn<'static>, SymbolKind> {
        &mut self.global_symbols
    }

    fn enter_symbol_into_parent_scope(&mut self, sym: Yarn<'static>, kind: SymbolKind) -> bool {
        unimplemented!()
    }
}

pub struct SubContext<'this, 'parent> {
    symbols_in_context: FxHashMap<Yarn<'this>, SymbolKind>,
    pub parent_context: &'this mut dyn Context<'parent>,
    kind: ContextKind,
}

impl<'this, 'parent> SubContext<'this, 'parent> {
    pub fn init(parent: &'this mut dyn Context<'parent>, kind: ContextKind) -> Self {
        Self {
            symbols_in_context: FxHashMap::default(),
            parent_context: parent,
            kind,
        }
    }
}

impl<'this, 'parent> Context<'this> for SubContext<'this, 'parent> {
    fn symbol_is_in_scope(&self, sym: &Yarn<'_>) -> bool {
        self.symbols_in_context.contains_key(sym) || self.parent_context.symbol_is_in_scope(sym)
    }

    fn enter_symbol_into_scope(&mut self, sym: Yarn<'this>, kind: SymbolKind) -> bool {
        !self.symbols_in_context.insert(sym, kind).is_some()
    }

    fn kind(&self) -> ContextKind {
        self.kind
    }

    fn new_child_context<'child>(
        &'child mut self,
        kind: ContextKind,
    ) -> Rc<RwLock<dyn Context<'child> + 'child>> {
        Rc::new(RwLock::new(SubContext::init(self, kind)))
    }

    fn parent_context_kind(&self) -> ContextKind {
        self.parent_context.kind()
    }

    fn kind_for_symbol(&self, sym: &Yarn<'_>) -> Option<SymbolKind> {
        match self.symbols_in_context.get(sym).map(|opt| opt.clone()) {
            Some(v) => Some(v),
            None => self.parent_context.kind_for_symbol(sym),
        }
    }

    fn symbols_in_context(&self) -> &FxHashMap<Yarn<'this>, SymbolKind> {
        &self.symbols_in_context
    }

    fn symbols_in_parent_ctx(&self) -> &FxHashMap<Yarn<'_>, SymbolKind> {
        self.parent_context.symbols_in_context()
    }

    fn symbols_in_context_mut(&mut self) -> &mut FxHashMap<Yarn<'this>, SymbolKind> {
        &mut self.symbols_in_context
    }

    fn enter_symbol_into_parent_scope(&mut self, sym: Yarn<'static>, kind: SymbolKind) -> bool {
        self.parent_context.enter_symbol_into_scope(sym, kind)
    }
}
