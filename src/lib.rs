//! The language that aims to give you simplicity like C/zig
//! while removing as much overhead as possible between you and the OS,
//! while also making it harder for you to make mistakes by making
//! you control the data flow explicitly and with intent, so you
//! dont have to fight the borrow checker which gets annoying sometimes

mod tests;
use rupple_frontend::RuppleFrontendError;
pub use rupple_interpreter::{
    EvalError,
    Interpreter,
};
pub use rupple_frontend::{
    LexError,
    ParseError,
    TypeError,

    SourceFile,
    Lexer,
    Parser,
    Type,
    TypeChecker,
    TypedProgram,
    checked_program,
};


#[derive(Debug, PartialEq)]
pub enum Error {
    Frontend(RuppleFrontendError),
    Interpreter(EvalError),
}

impl From<RuppleFrontendError> for Error {
    fn from(err: RuppleFrontendError) -> Self {
        Error::Frontend(err)
    }
}

impl From<EvalError> for Error {
    fn from(err: EvalError) -> Self {
        Error::Interpreter(err)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// Library entry point for rupple frontend
/// 
/// Call this if you want it to get the source from a file
/// without you having to read it and give it the source
pub fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut path = String::new();

    if args.len() > 1 {
        path = args[1].clone();
    } else {
        println!("Rupple")
    }

    let msg = format!("Couldn't read file {path}");

    let source = std::fs::read_to_string(path.trim()).expect(&msg);

    let file = SourceFile::new(path, source);
    let typed_program = checked_program(&file)?;
    
    Interpreter::new().evaluate(&typed_program)?;
    Ok(())
}

/// Call this function if you already have the filename and source
/// available
pub fn run_from_source(filename: &str, source: &str) -> Result<()> {
    let file = SourceFile::new(filename.to_string(), source.to_string());
    let typed_program = checked_program(&file)?;
    
    Interpreter::new().evaluate(&typed_program)?;
    Ok(())
}

#[test]
fn test_multiple_features_at_once() {
    let source = "
    // This code must be valid before we add more features
    // Including this comment here
    
    let mut resu = add(10, 20);
    
    fn add(x: int, y: int) -> int {
        return x + y;
    }
    
    let mut result: int = add(2, 3);
    result += 4;
    
    if result > 5 {
        result = result - 1;
    }
";

    assert_eq!(
        run_from_source("sample.rpl", &source), 
        Ok(())
    )
}

#[test]
fn test_immutable_assignment_on_unary_expression() {
    let source = 
    "
    let x = 1;
    x++;
    ";

    assert_eq!(run_from_source("sample.rpl", &source), 
        Err(
            Error::Frontend(
                RuppleFrontendError::Type(
                        TypeError::ImmutableAssignment("x".to_string())
                    )
                )
            )
    )
}