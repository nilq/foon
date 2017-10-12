use std::rc::Rc;

use super::*;

pub struct Parser {
    traveler: Traveler,
}

impl Parser {
    pub fn new(traveler: Traveler) -> Parser {
        Parser {
            traveler,
        }
    }
    
    pub fn parse(&mut self) -> ParserResult<Vec<Statement>> {
        let mut stack = Vec::new();

        while self.traveler.remaining() > 1 {
            self.skip_whitespace()?;
            stack.push(self.statement()?);
        }

        Ok(stack)
    }
    
    pub fn skip_whitespace(&mut self) -> ParserResult<()> {
        while self.traveler.current_content() == "\n" ||
              self.traveler.current().token_type == TokenType::EOL ||
              self.traveler.current().token_type == TokenType::Indent {

            self.traveler.next();
            
            if self.traveler.remaining() < 2 {
                break
            }
        }

        Ok(())
    }
    
    fn expression(&mut self) -> ParserResult<Expression> {
        self.skip_whitespace()?;

        let expr = self.term()?;

        if expr == Expression::EOF {
            return Ok(expr)
        }

        if self.traveler.remaining() > 1 {
            self.skip_whitespace()?;
            if self.traveler.current().token_type == TokenType::Operator {
                return self.operation(expr)
            }
        }

        Ok(expr)
    }
    
    pub fn try_type(&mut self) -> ParserResult<Type> {
        if self.traveler.current_content() == "mut" {
            self.traveler.next();
            if let Some(t) = Type::from_str(&self.traveler.current_content()) {
                self.traveler.next();
                Ok(Type::Mut(Some(Rc::new(t))))
            } else {
                Ok(Type::Mut(None))
            }
        } else if let Some(t) = Type::from_str(&self.traveler.current_content()) {
            self.traveler.next();
            Ok(t)
        } else {
            Err(ParserError::new_pos(self.traveler.current().position, &format!("expected type: {}", self.traveler.current_content())))
        }
    }

    pub fn term(&mut self) -> ParserResult<Expression> {
        self.skip_whitespace()?;

        if self.traveler.remaining() < 2 {
            return Ok(Expression::EOF)
        }

        match self.traveler.current().token_type {
            TokenType::IntLiteral    => {
                let a = Ok(Expression::Number(self.traveler.current_content().parse::<f64>().unwrap()));
                self.traveler.next();
                a
            }

            TokenType::FloatLiteral  => {
                let a = Ok(Expression::Number(self.traveler.current_content().parse::<f64>().unwrap()));
                self.traveler.next();
                a
            }

            TokenType::BoolLiteral   => {
                let a = Ok(Expression::Bool(self.traveler.current_content() == "true"));
                self.traveler.next();
                a
            }

            TokenType::StringLiteral => {
                let a = Ok(Expression::Str(Rc::new(self.traveler.current_content().clone())));
                self.traveler.next();
                a
            }
            
            TokenType::CharLiteral => {
                let a = Ok(Expression::Char(self.traveler.current_content().clone().remove(0)));
                self.traveler.next();
                a
            }

            TokenType::Identifier => {
                let a = Ok(Expression::Identifier(Rc::new(self.traveler.current_content().clone())));
                self.traveler.next();
                a
            },
            _ => Err(ParserError::new_pos(self.traveler.current().position, &format!("unexpected: {}", self.traveler.current_content()))),
        }
    }
    
    fn assignment(&mut self, left: Rc<Expression>) -> ParserResult<Statement> {
        self.traveler.next();

        let right = Rc::new(self.expression()?);

        Ok(
            Statement::Assignment(
                Assignment {
                    left,
                    right,
                }
            )
        )
    }
    
    fn statement(&mut self) -> ParserResult<Statement> {
        self.skip_whitespace()?;
        match self.traveler.current().token_type {
            TokenType::Symbol => match self.traveler.current_content().as_str() {
                "\n" => {
                    self.traveler.next();
                    self.statement()
                },
                _ => Ok(Statement::Expression(Rc::new(self.expression()?))),
            },
            TokenType::Identifier => {
                let a = Expression::Identifier(Rc::new(self.traveler.current_content().clone()));
                self.traveler.next();
                
                if self.traveler.current_content() == "=" {
                    self.assignment(Rc::new(a))
                } else {
                    self.traveler.prev();
                    Ok(Statement::Expression(Rc::new(self.expression()?)))
                }
            },
            TokenType::Type => {
                let t = self.try_type()?;
                
                self.traveler.expect_content(":")?;
                self.traveler.next();
                
                let left = Rc::new(Expression::Identifier(Rc::new(self.traveler.expect(TokenType::Identifier)?)));
                self.traveler.next();
                
                if self.traveler.current_content() == "=" {
                    self.traveler.next();

                    let right = Some(Rc::new(self.expression()?));

                    Ok(Statement::Definition(Definition { t, left, right }))
                    
                } else {
                    Ok(Statement::Definition(Definition { t, left, right: None }))
                }
            },
            _ => Ok(Statement::Expression(Rc::new(self.expression()?))),
        }
    }
    
    fn operation(&mut self, expression: Expression) -> ParserResult<Expression> {
        let mut ex_stack = vec![expression];
        let mut op_stack: Vec<(Operand, u8)> = Vec::new();
        
        op_stack.push(Operand::from_str(&self.traveler.current_content()).unwrap());
        self.traveler.next();

        if self.traveler.current_content() == "\n" {
            self.traveler.next();
        }
        
        let term = self.term()?;

        ex_stack.push(term);
        
        let mut done = false;
        
        while ex_stack.len() > 1 {
            if !done {
                if self.traveler.current().token_type != TokenType::Operator {
                    done = true;
                    continue
                }
                
                let (op, precedence) = Operand::from_str(&self.traveler.current_content()).unwrap();
                self.traveler.next();

                if precedence >= op_stack.last().unwrap().1 {
                    let left  = ex_stack.pop().unwrap();
                    let right = ex_stack.pop().unwrap();

                    ex_stack.push(
                        Expression::Operation(
                            Operation {
                                right: Rc::new(left),
                                op:    op_stack.pop().unwrap().0,
                                left:  Rc::new(right)
                            }
                        )
                    );

                    let term = self.term()?;

                    ex_stack.push(term);
                    op_stack.push((op, precedence));

                    continue
                }

                let term = self.term()?;

                ex_stack.push(term);
                op_stack.push((op, precedence));
            }

            let left  = ex_stack.pop().unwrap();
            let right = ex_stack.pop().unwrap();

            ex_stack.push(
                Expression::Operation(
                    Operation {
                        right: Rc::new(left),
                        op:    op_stack.pop().unwrap().0,
                        left:  Rc::new(right)
                    }
                )
            );
        }
                
        Ok(ex_stack.pop().unwrap())
    }
}
