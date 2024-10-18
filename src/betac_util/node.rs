use crate::Globals;
use std::env::consts::{ARCH, FAMILY, OS};

use super::{session::Session, Yarn};

#[derive(Debug, Clone)]
pub enum OperatorNode {
    Binary {
        lhs: Box<Self>,
        rhs: Box<Self>,
        op: BinOp,
    },
    Unary {
        rhs: Box<Self>,
        op: UnaryOp,
    },
    Ident {
        id: String,
    },
}

impl OperatorNode {
    /// creates a new, empty node. \n
    /// this is basically a dummy node, however, you need to have it mutable to do anything
    /// useful
    pub fn new() -> Self {
        Self::Ident { id: String::new() }
    }

    /// creates a node from raw parts, this is an internal function to support
    /// `OperatorNode::add_op`.
    ///
    pub fn from_parts(lhs: String, op: String, rhs: String) -> Self {
        match op.as_str() {
            "||" | "&&" => {
                let lhs = lhs.split(' ').collect::<Vec<_>>();
                let rhs = rhs.split(' ').collect::<Vec<_>>();
                Self::Binary {
                    lhs: Box::new(Self::from_parts(
                        lhs[0].into(),
                        lhs[1].into(),
                        lhs[2].into(),
                    )),
                    rhs: Box::new(Self::from_parts(
                        rhs[0].into(),
                        rhs[1].into(),
                        rhs[2].into(),
                    )),
                    op: BinOp::parse(&*op),
                }
            }
            _ => Self::Binary {
                lhs: Box::new(Self::Ident { id: lhs }),
                rhs: Box::new(Self::Ident { id: rhs }),
                op: BinOp::parse(&*op),
            },
        }
    }

    pub fn add_op(mut self, kind: OpKind, args: &Vec<String>) -> Self {
        let middle = args.len() / 2;
        let (is_chained, idx) =
            match args
                .iter()
                .enumerate()
                .position(|(idx, s)| match s.as_str() {
                    "||" | "&&" if idx >= middle - 3 && idx <= middle + 3 => true,
                    _ => false,
                }) {
                Some(idx) => (true, idx),
                None => (false, 0),
            };

        match kind {
            OpKind::Unary if is_chained => panic!("This definitely isn't unary"),
            OpKind::Bin if is_chained => {
                let before_splits = &args[..idx];
                let after_splits = &args[idx..];
                self = Self::Binary {
                    lhs: Box::new(Self::from_parts(
                        before_splits[0].to_owned(),
                        before_splits[1].to_owned(),
                        before_splits[2].to_owned(),
                    )),
                    rhs: Box::new(Self::from_parts(
                        after_splits[0].to_owned(),
                        after_splits[1].to_owned(),
                        after_splits[2].to_owned(),
                    )),
                    op: BinOp::parse(&*args[idx]),
                };
                self
            }
            OpKind::Unary => {
                self = Self::Unary {
                    rhs: Box::new(Self::Ident {
                        id: args[0].to_owned(),
                    }),
                    op: UnaryOp::parse(&*args[0]),
                };
                self
            }
            OpKind::Bin => {
                self = Self::Binary {
                    lhs: Box::new(Self::Ident {
                        id: args[0].to_owned(),
                    }),
                    rhs: Box::new(Self::Ident {
                        id: args[2].to_owned(),
                    }),
                    op: BinOp::parse(&*args[1]),
                };
                self
            }
        }
    }

    /// evaluate the node at compile-time, consuming itself and returning a boolean
    /// representing whether the statement is `true` or `false`
    pub fn comptime_evaluate(self, sess: &mut Session) -> bool {
        match self {
            OperatorNode::Binary { lhs, rhs, op } => {
                match (Box::into_inner(lhs), Box::into_inner(rhs), op) {
                    (lhs @ Self::Binary { .. }, rhs @ Self::Binary { .. }, BinOp::And) => {
                        return rhs.clone().comptime_evaluate(sess)
                            && lhs.clone().comptime_evaluate(sess);
                    }
                    (lhs @ Self::Binary { .. }, rhs @ Self::Binary { .. }, BinOp::Or) => {
                        return lhs.clone().comptime_evaluate(sess)
                            || rhs.clone().comptime_evaluate(sess);
                    }
                    (Self::Ident { id: lid }, Self::Ident { id: rid }, op) => {
                        match (lid.as_str(), rid.as_str(), op) {
                            ("DEFINED", key, BinOp::Eq)
                            | (key, "DEFINED", BinOp::Eq)
                            | ("!DEFINED", key, BinOp::Ne)
                            | (key, "!DEFINED", BinOp::Ne) => {
                                //return crate::Globals::with_mut(|globals| {
                                //  globals.defined.get(&Yarn::borrowed(key)).is_some()
                                //})
                                return sess.globals.defined.get(&Yarn::borrowed(key)).is_some();
                            }
                            ("!DEFINED", key, BinOp::Eq)
                            | (key, "!DEFINED", BinOp::Eq)
                            | ("DEFINED", key, BinOp::Ne)
                            | (key, "DEFINED", BinOp::Ne) => {
                                //return crate::Globals::with_mut(|globals| {
                                //    globals.defined.get(&Yarn::borrowed(key)).is_some()
                                //})
                                return sess.globals.defined.get(&Yarn::borrowed(key)).is_some();
                            }
                            ("ARCH", arch, BinOp::Eq) | (arch, "ARCH", BinOp::Eq) => {
                                return arch == ARCH.to_uppercase();
                            }
                            ("ARCH", arch, BinOp::Ne) | (arch, "ARCH", BinOp::Ne) => {
                                return arch != arch.to_uppercase();
                            }
                            ("OS", os, BinOp::Eq)
                            | (os, "OS", BinOp::Eq)
                            | ("OPERATING_SYSTEM", os, BinOp::Eq)
                            | (os, "OPERATING_SYSTEM", BinOp::Eq) => match os {
                                "WIN" | "WIN32" | "WIN64" => {
                                    return OS == "windows";
                                }
                                _ => {
                                    return OS.to_uppercase() == os;
                                }
                            },
                            ("OS", os, BinOp::Ne)
                            | (os, "OS", BinOp::Ne)
                            | ("OPERATING_SYSTEM", os, BinOp::Ne)
                            | (os, "OPERATING_SYSTEM", BinOp::Ne) => match os {
                                "WIN" | "WIN32" | "WIN64" => {
                                    return OS != "windows";
                                }
                                _ => {
                                    return OS.to_uppercase() != os;
                                }
                            },
                            ("OS_FAMILY", fam, BinOp::Eq) | (fam, "OS_FAMILY", BinOp::Eq) => {
                                return fam == FAMILY.to_uppercase();
                            }
                            ("OS_FAMILY", fam, BinOp::Ne) | (fam, "OS_FAMILY", BinOp::Ne) => {
                                return fam != FAMILY.to_uppercase();
                            }
                            (key, value, BinOp::Eq) => {
                                return sess
                                    .globals
                                    .defined
                                    .get(&Yarn::borrowed(key))
                                    .is_some_and(|v| v == &value)
                            }
                            (key, value, BinOp::Ne) => {
                                return OptionExt::is_none_or(
                                    sess.globals.defined.get(&Yarn::borrowed(key)),
                                    |v| v != &value,
                                )
                            }
                            _ => unimplemented!(),
                        }
                    }
                    _ => panic!("Cannot reach this area, so far"),
                }
            }
            OperatorNode::Unary { rhs, .. } => match *rhs {
                rhs @ Self::Binary { .. } => return !rhs.comptime_evaluate(sess),
                _ => return false,
            },
            _ => panic!("unreachable"),
        }
    }
}

/// possible unary operators that are seen in the token stream
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OpKind {
    Bin,
    Unary,
}

/// possible binary operators that could be seen in the token stream
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BinOp {
    Or,
    And,
    Eq,
    Ne,
    Dot,
    Arrow,
    Unknown,
}

impl BinOp {
    fn parse(s: &str) -> Self {
        match s {
            "||" => Self::Or,
            "&&" => Self::And,
            "==" => Self::Eq,
            "!=" => Self::Ne,
            "." => Self::Dot,
            "->" => Self::Arrow,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UnaryOp {
    Not,
    Unknown,
}

impl UnaryOp {
    fn parse(s: &str) -> Self {
        if s.contains('!') {
            Self::Not
        } else {
            Self::Unknown
        }
    }
}

pub trait OptionExt<T> {
    fn is_none_or(self, f: impl FnOnce(T) -> bool) -> bool;
}

impl<T> OptionExt<T> for Option<T> {
    fn is_none_or(self, f: impl FnOnce(T) -> bool) -> bool {
        match self {
            Some(v) => f(v),
            None => true,
        }
    }
}
