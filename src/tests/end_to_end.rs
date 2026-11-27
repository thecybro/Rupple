#![cfg(test)]

use rupple_frontend::{ArithmeticOp, RuppleFrontendError, SourceFile, Type, TypeError, checked_program};


#[test]
fn checker_rejects_non_integer_addition() {
    let file = SourceFile::new("sample.rpl", "let x = false + 2;");
    let result = checked_program(&file);

    assert!(matches!(
        result,
        Err(RuppleFrontendError::Type(
            TypeError::InvalidArithmeticOperation{
                left: Type::Bool,
                operator: ArithmeticOp::Add,
                right: Type::Int
            }
        ))
    ))
}
