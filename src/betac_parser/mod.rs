use std::sync::mpsc;

use crate::betac_tokenizer::{token::Token, Tokenizer};

pub struct Parser {
    rx: mpsc::Receiver<Token>,
}
