
mod parse;
mod source;
mod language;
mod semantic;

pub use parse::*;
pub use source::*;
pub use language::*;
pub use semantic::*;

#[derive(Debug, Clone, PartialEq)]
pub enum RuppleFrontendError {
    Lex(LexError),
    Parse(ParseError),
    Type(TypeError),
}

/// Since the root package just wants to get the typed program from the frontend
/// and doesn't need to manually assemble Lexer, Parser, TypeChecker etc,
/// so expose a clean API to make it easier.
/// 
/// Tho the modules are still public just incase anything wants to use them directly
pub fn checked_program(
    file: &SourceFile, 
) -> Result<TypedProgram, RuppleFrontendError> {    
    let lexed = Lexer::new(file.text())
        .lex()
        .map_err(RuppleFrontendError::Lex)?;
    
    let program = Parser::new(lexed).parse()
        .map_err(RuppleFrontendError::Parse)?;
    
    TypeChecker::new().check_program(program)
        .map_err(RuppleFrontendError::Type)
}