use std::{
    collections::HashMap,
    env::{self, Args},
    io::ErrorKind,
    iter::Skip,
};

pub mod fx_hasher;
use fx_hasher::FxHashMap;

use std::sync::{LazyLock, RwLock, RwLockReadGuard, RwLockWriteGuard};

static SESSION: LazyLock<RwLock<Session>> = LazyLock::new(|| {
    RwLock::new(Session {
        defined_symbols_in_global_scope: HashMap::default(),
        contents: String::new(),
        flags: 0,
    })
});

#[derive(Debug)]
pub struct Options {
    flags: u8,
}

pub struct Flags;

impl Flags {
    pub const TREE_FULL_BACKTRACE: u8 = 1 << 0;
    pub const DEBUG_MODE: u8 = 1 << 1;
    pub const RELEASE_MODE: u8 = 1 << 2;
}

enum SymbolKind {
    Function,
    Static,
    Constexpr,
}

pub struct Session {
    pub defined_symbols_in_global_scope: FxHashMap<String, SymbolKind>,
    pub contents: String,
    flags: u16,
}

impl Session {
    const FULL_BACKTRACE_TREE: u16 = 1 << 0;
    const DEBUG_MODE: u16 = 1 << 1;
    const BUILD_MODE: u16 = 1 << 2;
    const COMPILE_MODE: u16 = 1 << 3;

    pub fn enter_write_critical_section<F, R>(f: F) -> R
    where
        F: FnOnce(RwLockWriteGuard<'_, Session>) -> R,
    {
        f(SESSION.write().unwrap())
    }

    pub fn enter_read_section<F, R>(f: F) -> R
    where
        F: FnOnce(RwLockReadGuard<'_, Session>) -> R,
    {
        f(SESSION.read().unwrap())
    }

    pub fn set_full_tree_backtrace() {
        Self::enter_write_critical_section(|mut lock| {
            lock.flags |= Self::FULL_BACKTRACE_TREE;
        })
    }

    pub fn set_debug_mode() {
        Self::enter_write_critical_section(|mut lock| lock.flags |= Self::DEBUG_MODE)
    }

    pub fn set_compile_mode_flag() {
        Self::enter_write_critical_section(|mut lock| lock.flags |= Self::COMPILE_MODE)
    }

    pub fn has_compile_mode_flag_set() -> bool {
        Self::enter_read_section(|lock| lock.flags & Self::COMPILE_MODE != 0)
    }

    pub fn set_build_mode_flag() {
        Self::enter_write_critical_section(|mut lock| lock.flags |= Self::BUILD_MODE)
    }

    pub fn has_build_mode_flag_set() -> bool {
        Self::enter_read_section(|lock| lock.flags & Self::BUILD_MODE != 0)
    }

    pub fn has_debug_mode_enabled() -> bool {
        Self::enter_read_section(|lock| lock.flags & Self::DEBUG_MODE != 0)
    }

    pub fn has_full_tree_backtrace_set() -> bool {
        Self::enter_read_section(|lock| lock.flags & Self::FULL_BACKTRACE_TREE != 0)
    }
}

pub enum Response {
    Help,
    Version,
    Run { file_name: String },
}

pub fn parse_command_line_args() -> Result<Response, std::io::Error> {
    let mut args = env::args().skip(1);
    if args.len() < 1 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "not enough args found",
        ));
    }

    match args.next().unwrap().as_str() {
        "--help" => return Ok(Response::Help),
        "--version" => return Ok(Response::Version),
        "compile" => {
            Session::set_compile_mode_flag();
            let file_name = args.next().unwrap();
            parse_options(args).unwrap();
            return Ok(Response::Run {
                file_name: file_name.to_string(),
            });
        }
        "build" => {
            Session::set_build_mode_flag();
            todo!()
        }
        s => {
            return Err(std::io::Error::new(
                ErrorKind::NotFound,
                format!("{s} is not a valid option"),
            ))
        }
    }
}

fn parse_options(iter: Skip<Args>) -> Option<()> {
    if iter.len() == 0 {
        return Some(());
    } else {
        for arg in iter {
            match arg.as_str() {
                "-fbt" | "--set-full-backtrace" => {
                    if !Session::has_full_tree_backtrace_set() {
                        Session::set_full_tree_backtrace();
                    } else {
                        return None;
                    }
                }
                "-dbg" | "--debug" => {
                    if !Session::has_debug_mode_enabled() {
                        Session::set_debug_mode();
                    } else {
                        return None;
                    }
                }
                _ => todo!(),
            }
        }
        Some(())
    }
}
