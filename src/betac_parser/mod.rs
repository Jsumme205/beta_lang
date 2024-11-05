use crate::betac_tokenizer::token::Token;


pub struct Parser<'a, I> {
    tokens: I,
    input: &'a str,
    idx: u32,
}

impl<'a, I> Parser<'a, I>
where
    I: Iterator<Item = Token> + 'a
{

    pub fn new(input: &'a str, iter: I) -> Self {
        Self {
            tokens: iter,
            input,
            idx: 0,
        }
    }

    fn reconstruct_from_start_len(&self, start: u32, len: u32) -> &str {
        println!("{start}..{end}", end = start + len);
        let end = start + len;
        &self.input[start as usize..end as usize]
    }

    fn reconstruct_from_token_slice(&self, tokens: &[Token]) -> &str {
        let first = tokens.first().unwrap();
        let last = tokens.last().unwrap();
        self.reconstruct_from_start_len(first.start, *last - *first)
    }

    fn reconstruct_from_token_pair(&self, token1: Token, token2: Token) -> &str {
        self.reconstruct_from_start_len(token1.start, token2 - token1)
    }


    fn eat_tokens_while

}
