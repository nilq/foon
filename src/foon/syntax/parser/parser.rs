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
    
    fn array_type(&mut self) -> ParserResult<Type> {
        self.traveler.next();
        
        let t = Rc::new(Type::from_str(&self.traveler.expect(TokenType::Type)?).unwrap_or(Type::Any));
        self.traveler.next();

        if self.traveler.current_content() == ";" {
            self.traveler.next();

            let len = self.term()?;

            self.traveler.expect_content("]")?;
            self.traveler.next();

            Ok(Type::Array(t, Some(len)))
        } else {
            self.traveler.expect_content("]")?;
            self.traveler.next();

            Ok(Type::Array(t, None))
        }
    }
    
    pub fn try_type(&mut self) -> ParserResult<Type> {
        if self.traveler.current_content() == "mut" {
            self.traveler.next();

            let t: Option<Rc<Type>>;
            if self.traveler.current_content() == "[" {
                t = Some(Rc::new(self.array_type()?));
            } else if let Some(tt) = Type::from_str(&self.traveler.current_content()) {
                self.traveler.next();
                t = Some(Rc::new(tt));
            } else {
                t = None;
            }
            
            Ok(Type::Mut(t))

        } else if let Some(t) = Type::from_str(&self.traveler.current_content()) {
            self.traveler.next();
            Ok(t)
        } else if self.traveler.current_content() == "[" {
            Ok(self.array_type()?)
        } else {
            Err(ParserError::new_pos(self.traveler.current().position, &format!("expected type: {}", self.traveler.current_content())))
        }
    }
    
    fn try_call(&mut self, callee: Expression) -> ParserResult<Expression> {
        match self.traveler.current().token_type {
            TokenType::IntLiteral    |
            TokenType::FloatLiteral  |
            TokenType::BoolLiteral   |
            TokenType::StringLiteral |
            TokenType::CharLiteral   |
            TokenType::Identifier => self.call(callee),
            TokenType::Symbol     => match self.traveler.current_content().as_str() {
                "(" => self.call(callee),
                _   => Err(ParserError::new_pos(self.traveler.current().position, &format!("unexpected symbol: {}", self.traveler.current_content()))),
            },

            _ => Ok(callee),
        }
    }
    
    fn params(&mut self) -> ParserResult<Vec<(Option<Type>, Rc<String>)>> {
        self.traveler.expect_content("(")?;
        self.traveler.next();
        
        let mut params = Vec::new();
        
        while self.traveler.current_content() != ")" {
            if self.traveler.current_content() == "," {
                self.traveler.next();
            }
            match self.traveler.current().token_type {
                TokenType::Type => {
                    let t = self.try_type()?;

                    self.traveler.expect_content(":")?;
                    self.traveler.next();
                    
                    let id = self.traveler.expect(TokenType::Identifier)?;
                    self.traveler.next();
                    
                    params.push((Some(t), Rc::new(id)))
                },
                
                TokenType::Identifier => {
                    params.push((None, Rc::new(self.traveler.current_content())));
                    self.traveler.next();
                },
                
                _ => return Err(ParserError::new_pos(self.traveler.current().position, &format!("expected parameter: {}", self.traveler.current_content()))),
            }
        }
        
        self.traveler.next();
        
        Ok(params)
    }

    fn block(&mut self) -> ParserResult<Expression> {
        let mut stack = Vec::new();
        loop {
            if self.traveler.current().token_type == TokenType::Indent {
                self.traveler.next();
                if self.traveler.current_content() == "\n" {
                    self.traveler.next();
                    break
                }
            } else if self.traveler.current_content() == "\n" {
                stack.push(self.traveler.current().clone());
                self.traveler.next();

                if self.traveler.current().token_type == TokenType::Indent {
                    self.traveler.next();
                } else {
                    break
                }
            }

            if self.traveler.remaining() < 2 {
                break
            }

            stack.push(self.traveler.current().clone());
            self.traveler.next();
        }        

        let mut parser = Parser::new(Traveler::new(stack));

        match parser.parse() {
            Ok(s)    => Ok(Expression::Block(s)),
            Err(why) => Err(ParserError::new(&format!("{}", why))),
        }
    }
    
    fn body(&mut self) -> ParserResult<Expression> {
        if self.traveler.current_content() == "\n" {
            self.block()
        } else {
            self.expression()
        }
    }
    
    fn body_item(&mut self) -> ParserResult<Expression> {
        if self.traveler.current_content() == "\n" {
            let a = self.block()?;
            
            match a {
                Expression::Block(ref s) => match *s.get(0).unwrap() {
                    Statement::Expression(ref e) => Ok((&*e.clone()).clone()),
                    _ => Err(ParserError::new_pos(self.traveler.current().position, &format!("expected expression .. found statement"))),
                },
                _ => unreachable!(),
            }
        } else {
            self.expression()
        }
    }
    
    fn lambda_with_type(&mut self, t: Type) -> ParserResult<Expression> {
        let t = Rc::new(t);
        
        let params = self.params()?;
        
        let body = Rc::new(self.body()?);

        Ok(Expression::Lambda(Lambda {t, params, body}))
    }

    fn lambda(&mut self) -> ParserResult<Expression> {
        let t = Rc::new(self.try_type()?);

        self.traveler.expect_content(":")?;
        self.traveler.next();
        
        let params = self.params()?;
        
        let body = Rc::new(self.body()?);

        Ok(Expression::Lambda(Lambda {t, params, body}))
    }
    
    fn index(&mut self, id: Rc<Expression>) -> ParserResult<Expression> {
        self.traveler.next();

        let index = Rc::new(self.expression()?);

        self.traveler.expect_content("]")?;
        self.traveler.next();
        
        Ok(
            Expression::Index(
                Index {
                    id,
                    index,
                }
            )
        )
    }
    
    fn array(&mut self) -> ParserResult<Expression> {
        self.traveler.next();
        
        let mut content = Vec::new();
        
        let mut acc = 0;

        while self.traveler.current_content() != "}" {
            if self.traveler.current_content() == "," {
                self.traveler.next();
                content.push(Rc::new(self.expression()?));

            } else if acc == 0 {
                content.push(Rc::new(self.expression()?));

            } else {
                self.traveler.prev();
                if self.traveler.current_content() != "," {
                    self.traveler.next();
                }
                break
            }

            acc += 1
        }

        self.traveler.next();

        if self.traveler.current_content() == "[" {
            self.index(Rc::new(Expression::Array(content)))
        } else {
            Ok(Expression::Array(content))
        }
    }

    pub fn term(&mut self) -> ParserResult<Expression> {
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
                let a = Expression::Identifier(Rc::new(self.traveler.current_content().clone()));
                self.traveler.next();

                if self.traveler.remaining() > 1 {
                    match self.traveler.current_content().as_str() {
                        "," | ")" => Ok(a),
                        "["       => self.index(Rc::new(a)),
                        _         => self.try_call(a),
                    }
                } else {
                    Ok(a)
                }
            },
            
            TokenType::Symbol => match self.traveler.current_content().as_str() {
                "(" => {
                    self.traveler.next();
                    if self.traveler.current_content() == ")" {
                        return Err(ParserError::new_pos(self.traveler.current().position, &format!("empty clause '()'")))
                    }
                    
                    let a = self.expression()?;

                    self.skip_whitespace()?;
                    self.traveler.expect_content(")")?;
                    self.traveler.next();

                    if self.traveler.current_content() == "[" {
                        self.index(Rc::new(a))
                    } else if self.traveler.remaining() > 1 {
                        self.try_call(a)
                    } else {
                        Ok(a)
                    }
                }
                "{" => self.array(),
                _ => Err(ParserError::new_pos(self.traveler.current().position, &format!("unexpected symbol: {}", self.traveler.current_content()))),
            },
            
            TokenType::Type => self.lambda(),
            
            _ => Err(ParserError::new_pos(self.traveler.current().position, &format!("unexpected: {}", self.traveler.current_content()))),
        }
    }
    
    fn assignment(&mut self, left: Rc<Expression>) -> ParserResult<Statement> {
        self.traveler.next();

        let right = Rc::new(self.body_item()?);

        Ok(
            Statement::Assignment(
                Assignment {
                    left,
                    right,
                }
            )
        )
    }
    
    fn definition(&mut self) -> ParserResult<Statement> {
        let t = self.try_type()?;
        
        self.traveler.expect_content(":")?;
        self.traveler.next();

        let mut names = Vec::new();

        if self.traveler.current_content() == "\n" {
            self.traveler.next();

            loop {
                if self.traveler.current().token_type == TokenType::Indent {
                    self.traveler.next();
                    if self.traveler.current_content() == "\n" {
                        self.traveler.next();
                        break
                    }
                } else if self.traveler.current_content() == "\n" {
                    self.traveler.next();

                    if self.traveler.current().token_type == TokenType::Indent {
                        self.traveler.next();
                    } else {
                        break
                    }
                }

                if self.traveler.current_content() == "=" {
                    break
                }

                if self.traveler.remaining() < 2 {
                    break
                }

                names.push(Rc::new(Expression::Identifier(Rc::new(self.traveler.expect(TokenType::Identifier)?))));
                self.traveler.next();
            }
        } else if self.traveler.current_content() == "(" {
            return Ok(Statement::Expression(Rc::new(self.lambda_with_type(t)?)))
        } else {
            let a = self.traveler.expect(TokenType::Identifier)?;
            self.traveler.next();
            
            if self.traveler.current_content() == "(" {
                return Ok(Statement::Function(Rc::new(a), Rc::new(self.lambda_with_type(t)?)))
            } else {
                names.push(Rc::new(Expression::Identifier(Rc::new(a))));
            }
        }
        
        if self.traveler.current_content() == "=" {
            self.traveler.next();

            let right = Some(Rc::new(self.body_item()?));

            Ok(Statement::Definition(Definition { t, names, right }))
            
        } else {
            Ok(Statement::Definition(Definition { t, names, right: None }))
        }
    }

    fn statement(&mut self) -> ParserResult<Statement> {
        self.skip_whitespace()?;
        match self.traveler.current().token_type {
            TokenType::Symbol => match self.traveler.current_content().as_str() {
                "\n" => {
                    self.traveler.next();
                    self.statement()
                },
                "[" => self.definition(),
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
            TokenType::Type => self.definition(),
            _ => Ok(Statement::Expression(Rc::new(self.expression()?))),
        }
    }
    
    fn call(&mut self, caller: Expression) -> ParserResult<Expression> {
        let mut args = Vec::new();

        let mut acc = 0;

        while self.traveler.current_content() != "\n" {
            if self.traveler.current_content() == "," {
                self.traveler.next();
                
                let expr = Rc::new(self.expression()?);

                if *expr == Expression::EOF {
                    break
                }

                args.push(expr);

            } else if acc == 0 {
                let expr = Rc::new(self.expression()?);
                
                if *expr == Expression::EOF {
                    break
                }

                args.push(expr);

            } else {
                self.traveler.prev();
                if self.traveler.current_content() != "!" || self.traveler.current_content() != "," {
                    self.traveler.next();
                }
                break
            }
            
            acc += 1
        }
        
        Ok(
            Expression::Call(
                Call {
                    callee: Rc::new(caller),
                    args,
                }
            )
        )
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
