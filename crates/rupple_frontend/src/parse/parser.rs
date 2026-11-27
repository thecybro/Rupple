

use crate::language::{
    ArithmeticOp, AssignOp, ComparisonOp, Type, UnaryFixity, UnaryOp,
};
use super::{
    RawExpr,
    RawExprKind,
    RawStmtKind,
    RawFunctionParameter,
    Keyword,
    LexedOutput,
    RawProgram,
    RawStmt,
    Token,
    TokenKind,
};
use crate::source::{Span, Spanned};

#[derive(Debug)]
pub struct Parser<'a> {
    tokens: Box<[Token<'a>]>,
    position: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    Eof,
    InvalidToken(String),
    Type(String),
    // UnexpectedToken(String),
    TokenExpected(String),
    NotAKeyword(String),
    KeywordNotSupported(String),
    UnsupportedConditionalStatement(String),
    // UnsupportedKeyword(String),
    StringToF64,
    StringToI64,
}

type Result<T> = std::result::Result<T, ParseError>;

/// Public methods for the parser
impl<'a> Parser<'a> {
    /// The method that the caller needs to first call by passing a vector of
    /// tokens to later parse those tokens
    /// 
    /// Initialize the parser
    pub fn new(output: LexedOutput<'a>) -> Self {
        Self {
            tokens: output.tokens,
            position: 0
        }
    }

    /// Create an ast by calling the entry point parser function, which later
    /// leads to a recursive calling between itself, add operations parser,
    /// multiplication parser and primary parser which calls the add/multiplication
    /// parser or simply returns a number or identifier/variable by pattern matching
    pub fn parse(&mut self) -> Result<RawProgram> {
        let mut ast = vec![];

        // Loop over the tokens and parse the statements until we reach
        // the end of the tokens
        loop {
            if self.is_at_end() {
                break
            }

            ast.push(self.parse_statement()?)
        }

        // If somehow we didn't reach the end (which is unlikely cuz
        // the loop above ensures it runs until we reach the end)
        // But still, we aren't losing anything by keeping this, 
        // are we?
        if !self.is_at_end() {
            return Err(
                ParseError::InvalidToken(
                    self.current()?.lexeme.to_string()
                )
            )
        }
            
        Ok(
            RawProgram{
                ast: ast.into_boxed_slice()
            }
        )
    }
}

/// Private parsing methods for statements
impl<'a> Parser<'a> {
    /// Look at the tokens we have just gotten and see if we have any keywords
    /// in it, if we found any, call the respective parser to parse that keyword's 
    /// section.
    /// 
    /// Note that it returns after parsing the code/tokens associated with that statement
    /// like for example in the following code, if our position is 0, it will parse the
    /// let statement and then return, it won't parse the if statement
    /// 
    /// ```no_run
    /// let x = a + b
    /// if x > 10 {
    ///     // do something
    /// }
    /// ```
    fn parse_statement(&mut self) -> Result<RawStmt> {   
        match self.current()?.kind {
            TokenKind::Keyword(Keyword::Let) => self.parse_let_stmt(),
            TokenKind::Keyword(Keyword::If) => self.parse_if_stmt(),
            TokenKind::Keyword(Keyword::While) => self.parse_while_stmt(),
            TokenKind::Keyword(Keyword::Fn) => self.parse_function_stmt(),
            TokenKind::Keyword(Keyword::Return) => self.parse_return_stmt(),
            
            TokenKind::Identifier => {
                if self.peek_assign_operator().is_some() {
                    self.parse_assign_stmt()
                } else {
                    self.parse_expression_stmt()
                }
            },

            _ => {
                let curr_lexeme = self.curr_lexeme()?;
                Err(ParseError::KeywordNotSupported(
                    format!("'{curr_lexeme}' doesn't start a valid statement/expression")
                ))
            }
        }
    }
    
    /// Parse the statement if it starts with let
    fn parse_let_stmt(&mut self) -> Result<RawStmt> {  
        let keyword_start = self.expect_keyword(Keyword::Let)?
            .span
            .start;

        let mut mutable = false;
        if self.take_if(TokenKind::Keyword(Keyword::Mut)) {
            mutable = true
        }
        let identifier = *self.expect_identifier()?;
        
        let mut annotation = None;
        if self.take_if(TokenKind::Colon) {
            annotation = Some(self.expect_type()?)
        }
        
        self.expect(TokenKind::Equals)?;

        let expression = self.parse_expression()?;

        let sem_end = self.expect_semicolon()?
            .span
            .end;

        let ident = Spanned {
            node: identifier.lexeme.to_string(),
            span: identifier.span
        };
        
        Ok(RawStmt {
            node: RawStmtKind::Let {
                identifier: ident,
                mutable,
                annotation,
                expression,
            },
            span: Span {
                start: keyword_start,
                end: sem_end
            }
        })
        
    }

    /// Parse the statement if it starts with an identifier
    /// which is likely to be an assignment statement
    fn parse_assign_stmt(&mut self) -> Result<RawStmt> {
        let identifier = *self.expect_identifier()?; // Consume the identifier

        let operator = self.expect_assign_operator()?;
        
        let expression = self.parse_expression()?;

        let sem_end = self.expect_semicolon()?
            .span
            .end;

        let span = Span {
            start: identifier.span.start,
            end: sem_end,
        };

        let ident = Spanned {
            node: identifier.lexeme.to_string(),
            span: identifier.span
        };
        
        Ok(
            RawStmt {
                node: RawStmtKind::Assign {
                    identifier: ident,
                    operator,
                    expression
                },
                span
            }
        )
    }

    /// If and while statements have same 2 things: a conditional expression and
    /// a code block, so they can be parsed in the same way
    /// 
    /// Parse the section of the tokens that come under the if statement
    fn parse_if_stmt(&mut self) -> Result<RawStmt> {
        self.parse_condition_stmt(Keyword::If)
    }

    /// Parse the section that the while loop should cover 
    fn parse_while_stmt(&mut self) -> Result<RawStmt> {
        self.parse_condition_stmt(Keyword::While)
    }

    /// Function parser
    /// A function statement is the declaration/implementation of a function; not the call to it.
    /// For example (statement)
    /// ```no_run
    /// fn add_one(a: int) -> int {
    ///     return a + 1
    /// }
    /// ```
    /// 
    /// The function expression looks like this
    /// ```no_run
    /// 
    /// let x = add_one(1) 
    /// ```
    fn parse_function_stmt(&mut self) -> Result<RawStmt> {
        let keyword_start = self.expect_keyword(Keyword::Fn)?
            .span
            .start;

        let name = *self.expect_identifier()?;
        
        self.expect(TokenKind::LParen)?;

        let parameters = self.parse_comma_separated(|parser| {
            let name = *parser.expect_identifier()?;
            parser.expect(TokenKind::Colon)?;

            let data_type = parser.expect_type()?;

            let nm = Spanned {
                node: name.lexeme.to_string(),
                span: name.span
            };
            
            Ok(RawFunctionParameter {
                name: nm,
                data_type
            })
        })?.into_boxed_slice();
        
        let mut return_type = Type::Unit;
        if self.take_if(TokenKind::Arrow) {
            return_type = self.expect_type()?;
        }
        
        self.expect(TokenKind::LBrace)?;

        let mut body = Vec::new();

        while !self.at(TokenKind::RBrace) {
            if self.is_at_end() {
                return Err(ParseError::TokenExpected(
                    "Expected '}' before end of file!".to_string()
                ));
            }

            body.push(self.parse_statement()?);
        }

        let rbrace_end = self.expect(TokenKind::RBrace)?
            .span
            .end;

        let nm = Spanned {
            node: name.lexeme.to_string(),
            span: name.span
        };
        
        Ok(
            RawStmt {
                node: RawStmtKind::Function {
                    name: nm,
                    parameters,
                    return_type,
                    body: body.into_boxed_slice()
                },
                span: Span {
                    start: keyword_start,
                    end: rbrace_end
                }
            }
        )
    }

    /// Parse the statements starting with return
    /// # Example:
    /// ```no_run
    /// fn main() -> int {
    ///     return 1 // return statement
    /// }
    /// ```
    fn parse_return_stmt(&mut self) -> Result<RawStmt> {
        let keyword_start = self.expect_keyword(Keyword::Return)?
            .span
            .start;
        
        let value = self.parse_expression()?;

        let sem_end = self.expect_semicolon()?
            .span
            .end;
        
        Ok(
            RawStmt {
                node: RawStmtKind::Return(Some(value)),
                span: Span {
                    start: keyword_start,
                    end: sem_end
                }
            }
        )
    }

    /// Parse the expression statements 
    /// # Example 
    /// ```no_run
    /// // this is treated as an expression statement because this is standalone
    /// x++;  
    /// 
    /// // this (x++) is treated as a normal expression because it is part of a
    /// // statement
    /// let y = x++;
    /// ```
    /// 
    /// Note that not all expressions are valid expression statements
    fn parse_expression_stmt(&mut self) -> Result<RawStmt> {
        if self.at(TokenKind::Identifier) {
            let expr = self.parse_expression()?;

            let sem = self.expect_semicolon()?;

            let span = Span {
                start: expr.span.start,
                end: sem.span.end
            };
            
            Ok(RawStmt {
                node: RawStmtKind::Expression(expr),
                span
            })
        } else {
            Err(ParseError::TokenExpected(
                "Expected an identifier to form an expression statement!".to_string()
            ))
        }
    }
}

/// Private parsing methods for expressions
impl<'a> Parser<'a> {
    /// Start the parsing by calling the operation parser with least precedence;
    /// here, we called conditional cuz binary operations need to be done before
    /// conditional operations
    fn parse_expression(&mut self) -> Result<RawExpr> {
        self.parse_comparison()
    }

    /// Since we need to make sure to parse the binary operations if there are any,
    /// we call them from inside the conditional parser
    /// 
    /// If there are any binary operators, we would get binary tree,
    /// and if there are no binary operators, we would just get the identifiers or 
    /// numbers
    fn parse_comparison(&mut self) -> Result<RawExpr> {
        // get the expression to put at the left of the conditional operator 
        let mut left = self.parse_addition()?; 

        // If we haven't reached EOF yet (which is indicated by Ok()), check if 
        // there are any conditional operators, if there are, get the operator
        // and move forward, if not, break the loop and just return the binary tree
        // or identifier or number we had gotten from the first time we called
        // self.parse_addition()?; 
        while let Ok(current) = self.current() {
            let operator = match current.kind {
                TokenKind::EqualsEquals => ComparisonOp::IsEqualTo,
                TokenKind::GreaterThan => ComparisonOp::IsGreaterThan,
                TokenKind::LessThan => ComparisonOp::IsLessThan,
                TokenKind::GreaterOrEquals => ComparisonOp::IsGreaterOrEqualTo,
                TokenKind::LessOrEquals => ComparisonOp::IsLessOrEqualTo,
                TokenKind::NotEquals => ComparisonOp::IsNotEqualTo,
                _ => break
            };

            // We reaching this line means we have gotten a comparison operator,
            // so consume it
            self.advance()?;

            // Now parse the right hand side expression (which will give either a
            // number or identifier or a binary)
            let right = self.parse_addition()?;

            let span = Span {
                start: left.span.start,
                end: right.span.end
            };
            
            left = RawExpr {
                node: RawExprKind::Comparison {
                    left: Box::new(left),
                    operator,
                    right: Box::new(right)
                },
                span
            };
        }
        Ok(left)
    }
    
    ///
    /// Since multiplication/division has higher precedence, we call parse_multiplication()
    /// to check if we have any multiplication/division operation, if there is any, it will
    /// set the return value in left expression
    /// 
    /// If we haven't reached the end of file yet; meaning we still get the current token,
    /// we will look for plus/minus operator and then consume it (it's just the increase in
    /// our current positino actually).
    /// And then we will call the parse_multiplication() to do the operation again and set it
    /// in the right operation by putting plus/minus operator in between.
    /// 
    /// The reason we dont have to check if there are still tokens left after consuming the operator
    /// is that; the parse_multiplication() already does that internally
    ///
    fn parse_addition(&mut self) -> Result<RawExpr> {
        let mut left = self.parse_multiplication()?;
        
        while let Ok(token) = self.current() {
            let operator = match token.kind {
                TokenKind::Plus => ArithmeticOp::Add,
                TokenKind::Minus => ArithmeticOp::Subtract,
                _ => break
            };

            self.advance()?;
            let right = self.parse_multiplication()?;

            let span = Span {
                start: left.span.start,
                end: right.span.end
            };
            
            left = RawExpr {
                node: RawExprKind::Arithmetic {
                    left: Box::new(left),
                    operator,
                    right: Box::new(right)
                },
                span,
            };
        }
        
        Ok(left)
    }

    /// If there is multiplication of division operator in the remaining tokens, 
    /// parse them and return the parsed expression
    fn parse_multiplication(&mut self) -> Result<RawExpr> {
        /*
         * Call parse_unary() to get the numbers/identifiers
         * or a binary (either containing numbers/identifiers or other binaries)
         */
        let mut left = self.parse_unary()?;

        while let Ok(token) = self.current() {
            let operator = match token.kind {
                TokenKind::Slash => ArithmeticOp::Divide,
                TokenKind::Star => ArithmeticOp::Multiply,
                _ => break,
            };

            /*
             * Consume the operator and call again to get the right numbers/identifiers
             * or a binary
             */
            self.advance()?;
            let right = self.parse_unary()?;

            let span = Span {
                start: left.span.start,
                end: right.span.end
            };
            
            left = RawExpr {
                node: RawExprKind::Arithmetic{
                    left: Box::new(left),
                    operator,
                    right: Box::new(right)
                },
                span,
            };
        }
        Ok(left)
    }

    /// Parse and return unary expressions if present
    /// # Example
    /// `x++`
    /// `+x` etc
    fn parse_unary(&mut self) -> Result<RawExpr> {
        // We check for prefix unary operator first cuz they have higher
        // precedence than postfix ones
        if let Some(operator) = self.current_prefix_operator() {
            let consumed_operator = *self.advance()?;
            
            let operand = self.parse_unary()?;

            let span = Span {
                start: consumed_operator.span.start,
                end: operand.span.end
            };
            
            return Ok(RawExpr {
                node: RawExprKind::Unary {
                    operator,
                    operand: Box::new(operand),
                    fixity: UnaryFixity::Prefix
                },
                span
            });
        }
        self.parse_postfix()
    }


    /// Parse postfix unary operators/operands if present and return expression
    fn parse_postfix(&mut self) -> Result<RawExpr> {
        let mut operand = self.parse_primary()?;
        
        while let Some(current) = self.current_kind() {
            let operator = match current {
                TokenKind::PlusPlus => UnaryOp::Increment,
                TokenKind::MinusMinus => UnaryOp::Decrement,
                _ => break
            };

            let consumed_operator = self.advance()?;

            let span = Span {
                start: operand.span.start,
                end: consumed_operator.span.end
            };
            
            operand = RawExpr {
                node: RawExprKind::Unary {
                    operator,
                    operand: Box::new(operand),
                    fixity: UnaryFixity::Postfix
                },
                span,
            };
        }

        Ok(operand)
    }
    
    /// This is the actual fundamental function that identifies what we have and returns
    /// numbers or identifiers or expressions by pattern matching
    /// Others just indirectly call it and add respective operators; 
    /// they dont do the underlying work
    fn parse_primary(&mut self) -> Result<RawExpr> {
        let token = self.current()?;

        let token_span = Span {
            start: token.span.start,
            end: token.span.end,
        };
        
        match token.kind {
            TokenKind::IntegerLiteral => {
                if let Some(integer) = parse_to_i64(token.lexeme) {
                    self.advance()?;
                    Ok(RawExpr {
                        node: RawExprKind::Integer(integer),
                        span: token_span
                    })
                } else {
                    Err(ParseError::StringToI64)
                }
            },
            TokenKind::FloatLiteral => {
                if let Some(float) = parse_to_f64(token.lexeme) {
                    self.advance()?;
                    Ok(RawExpr {
                        node: RawExprKind::Float(float),
                        span: token_span
                    })
                } else {
                    Err(ParseError::StringToF64)
                }
            },
            TokenKind::StringLiteral => {
                Ok(RawExpr {
                    node: RawExprKind::String(
                        self.advance()?.lexeme.to_string()
                    ),
                    span: token_span
                })
            },
            TokenKind::BooleanLiteral(boolean_val) => {
                self.advance()?;
                Ok(RawExpr {
                    node: RawExprKind::Boolean(boolean_val),
                    span: token_span
                })
            },
            TokenKind::Identifier => {
                let identifier = *self.advance()?;

                if self.is_at_end() || self.current()?.kind != TokenKind::LParen {
                    return Ok(RawExpr {
                        node: RawExprKind::Identifier(identifier.lexeme.to_string()),
                        span: token_span
                    });
                }
                
                self.advance()?; // consume '('

                // Since we are at the condition like `foo(`
                // We can assume that it's gonna be a function call,
                // and since parse_comma_separated() slready handles
                // empty (), we can just call it and get the arguments back
                // Vec<> of arguments if there were any, empty Vec<> if there
                // were none
                let arguments = self.parse_comma_separated(|parser| {
                    parser.parse_expression()
                })?.into_boxed_slice();

                let spanned_ident = Spanned {
                    node: identifier.lexeme.to_string(),
                    span: identifier.span
                };
                
                Ok(RawExpr {
                    node: RawExprKind::FunctionCall {
                        name: spanned_ident,
                        arguments
                    },
                    span: Span {
                        start: token_span.start,
                        end: self.previous()?.span.end,
                    }
                })                
            },
            /*
             * This is the start of other expression inclosed in () so we have to consume the (
             * and parse the expression again, which will not touch the closing parenthesis ),
             * so we have to consume the closing parenthesis ) if we encountered it, consume it 
             * and return the expression; indicating success.
             * 
             * If we didn't encounter the closing parenthesis, we return error saying we expected
             * a closing parenthesis
             */
            TokenKind::LParen => {
                self.advance()?;

                // Parse the expression inside (
                let mut expr = self.parse_expression()?;
                
                if self.is_at_end() {
                    return Err(
                        ParseError::TokenExpected(
                            "')' expected at the end".to_string()
                        )
                    )
                }
 
                if self.current_kind() != Some(TokenKind::RParen) {
                    let lex = self.curr_lexeme()?;
                    return Err(ParseError::TokenExpected(
                               format!("Expected ')' but got '{lex} instead'")
                        ));
                }

                // We now consume the ) and mark it as the end of this expression
                let end = self.advance()?.span.end;

                expr.span = Span {
                    start: token_span.start,
                    end,
                };
                
                Ok(expr)
            },
            _ => {
                Err(
                    ParseError::InvalidToken(
                    token.lexeme.to_string()
                ))
            }
        }
    }
}

/// Major complimentary helpers method that do much of the work
impl<'a> Parser<'a> {
    fn parse_comma_separated<T, F>(
        &mut self,
        // The method to call to parse the item
        mut parse_item: F,
    ) -> Result<Vec<T>>
    where 
        F: FnMut(&mut Self) -> Result<T>,
    {
        let mut items = Vec::new();

        // We do take_if() instead of at() because we want to consume and return if
        // we are already at )
        // 
        // Another reason for us to take RParen instead of returning/letting the caller
        // consume is that the caller calls this method only if they have already found/consumed
        // LParen, so we dont make them think whether or not this method has finished parsing or not
        // 
        // We always finish everything inside (), consume the last ) and only return 
        if self.take_if(TokenKind::RParen) {
            return Ok(items);
        }

        loop {
            items.push(parse_item(self)?);

            if self.take_if(TokenKind::RParen) {
                break;
            }

            self.expect(TokenKind::Comma)?;

            if self.at(TokenKind::RParen) {
                return Err(ParseError::TokenExpected(
                    "Expected an item after ','".to_string()
                ));
            }
        }

        Ok(items)
    }

    /// Parse any code block typically enclosed between {}}
    fn parse_block(&mut self) -> Result<Box<[RawStmt]>> {
        // The block needs to start with a {
        self.expect(TokenKind::LBrace)?;

        let mut stmts = Vec::new();

        // Keep going until we get the stop kind or }
        while !self.at(TokenKind::RBrace) {
            if self.is_at_end() {
                return Err(ParseError::TokenExpected(
                    "Expected '}' before end of file!".to_string()
                ));
            }

            stmts.push(self.parse_statement()?);
        }
        
        // We must get a } or stop_kind after we have finished parsing a block
        self.expect(TokenKind::RBrace)?;

        Ok(stmts.into_boxed_slice())
    }

    fn parse_condition_stmt(&mut self, keyword: Keyword) -> Result<RawStmt> {
        let consumed_start = self.expect_keyword(keyword)?
            .span
            .start;

        match keyword {
            Keyword::If => Ok(RawStmt {
                node: RawStmtKind::If { 
                    condition: self.parse_expression()?,
                    body: self.parse_block()?
                },
                span: Span {
                    start: consumed_start,
                    end: self.previous()?.span.end
                }
            }),

            Keyword::While => Ok(RawStmt {
                node: RawStmtKind::While { 
                    condition: self.parse_expression()?,
                    body: self.parse_block()?
                },
                span: Span {
                    start: consumed_start,
                    end: self.previous()?.span.end
                }
            }),

            _ => {
                Err(ParseError::UnsupportedConditionalStatement(
                    "Unsupported conditional statement was found!\n Use if or while instead".to_string()
                ))
            }
        }
    }

}

/// General token helper methods
impl<'a> Parser<'a> {
    /// Returns `true` if we are currently at the end
    fn is_at_end(&self) -> bool {
        self.position >= self.tokens.len()
    }
    
    /// Get the current but give None if there's non (eof)
    fn current_opt(&self) -> Option<&Token<'a>> {
        self.tokens.get(self.position)
    }

    /// Returns current token if present; error if not
    fn current(&self) -> Result<&Token<'a>> {
        self.current_opt().ok_or(ParseError::Eof)
    }

    /// Returns the kind of current token
    fn current_kind(&self) -> Option<TokenKind> {
        self.current_opt().map(|token| token.kind)
    }

    /// Returns the lexeme (literal) of the current token
    fn curr_lexeme(&self) -> Result<String> {
        Ok(self.current()?.lexeme.to_string())
    }

    /// Returns next token without consuming if present
    fn peek_opt(&self) -> Option<&Token<'a>> {
        self.tokens.get(self.position + 1)
    }

    /// Returns kind of next token without consuming if present
    fn peek_kind(&self) -> Option<TokenKind> {
        self.peek_opt().map(|token| token.kind)
    }

    /// Returns previous token if present; error if not
    fn previous(&self) -> Result<&Token<'a>> {
        self.prev_opt().ok_or(ParseError::Eof)
    }

    /// Returns previous token if present; None if not
    fn prev_opt(&self) -> Option<&Token<'a>> {
        self.tokens.get(self.position -1)
    }
    
    /// Are we at this tokenkind? True if yes
    fn at(&self, kind: TokenKind) -> bool {
        self.current_kind() == Some(kind)
    }

    /// Advance/consume if the current kind is this
    fn take_if(&mut self, kind: TokenKind) -> bool {
        if self.at(kind) {
            self.position += 1;
            true
        } else {
            false
        }
    }

    /// Consume (advance the cursor) the current token
    fn advance(&mut self) -> Result<&Token<'a>> {
        if self.is_at_end() {
            return Err(ParseError::Eof);
        }
    
        self.position += 1;
        // Return the token from 1 position prior, which will be the one
        // which we just consumed, because we had just done +1
        self.tokens
            .get(self.position - 1)
            .ok_or(ParseError::Eof)
    }    
}

/// Specific token related helpers
impl<'a> Parser<'a> {
    /// We expect a certain kind.
    /// Consume and return if we got it; 
    /// and throw an error if we didn't get it
    fn expect(&mut self, expected: TokenKind) -> Result<&Token<'a>> {
        if self.take_if(expected) {
            self.previous()
        } else if self.is_at_end() {
            Err(ParseError::TokenExpected(
                format!("Expected {expected:#?}, but reached end of file")
            ))
        } else {
            let actual = self.curr_lexeme()?;

            Err(ParseError::TokenExpected(
                format!("Expected {expected:#?}, got '{actual}'")
            ))
        }
    }

    /// If the current kind is an identifier (confirmed if expect() passed)
    /// we return the current token; being sure it's an identifier
    fn expect_identifier(&mut self) -> Result<&Token<'a>> {
        self.expect(TokenKind::Identifier)
    }

    /// Consume and return the expected keyword, throw error if we didn't find it
    fn expect_keyword(&mut self, expected: Keyword) -> Result<&Token<'a>> {
        match self.current()?.kind {
            TokenKind::Keyword(keyword) if keyword == expected => {
                Ok(self.advance()?)
            }

            TokenKind::Keyword(keyword) => {
                Err(ParseError::TokenExpected(
                    format!("Expected keyword '{expected:#?}', got '{keyword:#?}'")
                ))
            }

            _ => {
                let actual = self.curr_lexeme()?;

                Err(ParseError::TokenExpected(
                    format!("Expected keyword, got '{actual}'")
                ))
            }
        }
    }

    /// Consume and return the data type if we got it.
    /// But throw an error if there is no data type currently
    fn expect_type(&mut self) -> Result<Type> {
        self.take_type().ok_or_else(|| {
            ParseError::TokenExpected(
                "Expected a data type (eg: int, float, sting)".to_string()
            )
        })
    }

    /// Consume and return the assign operator if present,
    /// error otherwise
    fn expect_assign_operator(&mut self) -> Result<AssignOp> {
        self.take_assign_operator().ok_or_else(|| {
            ParseError::TokenExpected(
                "Expected an assign operator (eg: =, +=, -=)".to_string()
            )
        })
    }

    /// Consume and return the semicolon if present;
    /// error otherwise
    fn expect_semicolon(&mut self) -> Result<&Token<'a>> {
        self.expect(TokenKind::SemiColon)
    }
    
    /// Keyword data type means int, float etc
    fn current_type(&self) -> Option<Type> {
        match self.current_kind()? {
            TokenKind::Keyword(Keyword::Type(_type)) => Some(_type),
            _ => None
        }
    }

    /// Consume the data type if present
    fn take_type(&mut self) -> Option<Type> {
        let dt = self.current_type()?;
        self.advance().ok();
        Some(dt)
    }

    /// Consume and return if current token is assign operator
    fn take_assign_operator(&mut self) -> Option<AssignOp> {
        let op = self.current_assign_operator()?;
        self.advance().ok();
        Some(op)
    }

    /// Returns the unary operator if present
    /// None otherwise
    fn current_prefix_operator(&self) -> Option<UnaryOp> {
        match self.current_kind()? {
            TokenKind::Plus => Some(UnaryOp::Plus),
            TokenKind::Minus => Some(UnaryOp::Minus),
            TokenKind::Not => Some(UnaryOp::Not),
            TokenKind::PlusPlus => Some(UnaryOp::Increment),
            TokenKind::MinusMinus => Some(UnaryOp::Decrement),
            _ => None
        }
    }

    /// Returns the assign operator if present
    /// None otherwise
    fn current_assign_operator(&self) -> Option<AssignOp> {
        match self.current_kind()? {
            TokenKind::Equals => Some(AssignOp::Assign),
            TokenKind::PlusEquals => Some(AssignOp::AddAssign),
            TokenKind::MinusEquals => Some(AssignOp::SubtractAssign),
            _ => None
        }
    }

    /// Returns the unary operator if the next one is that
    /// None otherwise
    fn peek_assign_operator(&self) -> Option<AssignOp> {
        match self.peek_kind()? {
            TokenKind::Equals => Some(AssignOp::Assign),
            TokenKind::PlusEquals => Some(AssignOp::AddAssign),
            TokenKind::MinusEquals => Some(AssignOp::SubtractAssign),
            _ => None
        }
    }
}

/// An external helper for parsing strings to f64
fn parse_to_f64(input: &str) -> Option<f64> {
    input.parse::<f64>().ok()
}

/// An external helper for parsing strings to i64
fn parse_to_i64(input: &str) -> Option<i64> {
    input.parse::<i64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_source;

    #[test]
    fn binary_expressions_nodes_preserve_spans() {
        let program = parse_source("let result = a + b;");
        let statement = &program.ast[0];

        let RawStmtKind::Let { expression, ..} = &statement.node else {
            panic!("\nexpected a let statement\n");
        };

        // Let statement consumed upto the ;, so byte offset is [13, 18]
        assert_eq!(
            expression.span,
            Span {
                start: 13,
                end: 18
            }
        );

        let RawExprKind::Arithmetic { left, right, .. } = &expression.node else {
            panic!("\nexpected an arithmetic expression\n");
        };

        assert_eq!(
            left.span,
            Span {
                start: 13,
                end: 14
            }
        );

        assert_eq!(
            right.span,
            Span {
                start: 17,
                end: 18
            }
        );
    }

    #[test]
    fn paren_expressions_nodes_preserve_spans() {
        let program = parse_source("let x = (a+b);");
        let statement = &program.ast[0];

        let RawStmtKind::Let { expression, .. } = &statement.node else {
            panic!("\nexpected a let statement\n")
        };

        // token upto ; is counted as part of expression,
        // so we consider its span as well
        assert_eq!(
            expression.span,
            Span {
                start: 8,
                end: 13
            }
        );

        let RawExprKind::Arithmetic { left, right, .. } = &expression.node else {
            panic!("\nexpected an arithmetic expression\n");
        };

        assert_eq!(
            left.span,
            Span {
                start: 9,
                end: 10
            }
        );

        assert_eq!(
            right.span,
            Span {
                start: 11,
                end: 12
            }
        );
    }

    #[test]
    fn func_call_expressions_nodes_preserve_sapans() {
        let program = parse_source("let x = add(a,b);");
        let statement = &program.ast[0];

        let RawStmtKind::Let { expression, .. } = &statement.node else {
            panic!("\nexpected a let statement\n");
        };

        assert_eq!(
            expression.span,
            Span {
                start: 8,
                end: 16
            }
        );

        let RawExprKind::FunctionCall { arguments, .. } = &expression.node else {
            panic!("\nexpected a function call expression\n");
        };

        let mut args = arguments.iter();
        
        let (Some(arg1), Some(arg2)) = (args.next(), args.next()) else {
            panic!("\nexpected arguments to be present in the function call expression\n");
        };
    
        assert_eq!(
            arg1.span,
            Span {
                start: 12,
                end: 13
            }
        );

        assert_eq!(
            arg2.span,
            Span {
                start: 14,
                end: 15
            }
        );
    }
}
