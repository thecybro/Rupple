//! The language that aims to give you simplicity like C/zig
//! while removing as much overhead as possible between you and the OS,
//! while also making it harder for you to make mistakes by making
//! you control the data flow explicitly and with intent, so you
//! dont have to fight the borrow checker which gets annoying sometimes


/// A simple glue for the lexer and parser (and evaluater/code generator later)
/// which calls and takes return value from one and gives it to other
fn main() -> rupple::Result<()> {
    rupple::run()
}
