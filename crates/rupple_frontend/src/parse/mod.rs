mod ast;
mod lexer;
mod parser;
mod token;

pub use lexer::*;
pub use parser::{ParseError, Parser};
pub use token::*;

// Raw syntax nodes are available inside the frontends crate, so the semantic
// checker can consume them, but they are not part of the public frontend API.
pub(crate) use ast::*;

#[derive(Debug, Clone)]
pub struct LexedOutput<'a> {
    pub tokens: Box<[Token<'a>]>,
}

pub(crate) fn parse_source(source: &str) -> RawProgram {
    let tokens = Lexer::new(source)
        .lex()
        .expect("\ntest source should lex\n");

    Parser::new(tokens)
        .parse()
        .expect("\ntest source should parse\n")
}