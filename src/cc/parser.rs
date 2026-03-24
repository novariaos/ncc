use super::ast::*;
use super::token::Token;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    fn peek2(&self) -> &Token {
        self.tokens.get(self.pos + 1).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) -> Token {
        let tok = self.tokens.get(self.pos).cloned().unwrap_or(Token::Eof);
        self.pos += 1;
        tok
    }

    fn expect(&mut self, expected: &Token) -> Result<(), String> {
        let tok = self.advance();
        if &tok == expected {
            Ok(())
        } else {
            Err(format!(
                "Expected {:?}, got {:?} at token position {}",
                expected, tok, self.pos - 1
            ))
        }
    }

    fn expect_ident(&mut self) -> Result<String, String> {
        match self.advance() {
            Token::Ident(s) => Ok(s),
            other => Err(format!(
                "Expected identifier, got {:?} at token position {}",
                other, self.pos - 1
            )),
        }
    }

    fn at(&self, tok: &Token) -> bool {
        self.peek() == tok
    }

    fn eat(&mut self, tok: &Token) -> bool {
        if self.peek() == tok {
            self.advance();
            true
        } else {
            false
        }
    }

    pub fn parse_program(&mut self) -> Result<Program, String> {
        let mut program = Program {
            structs: Vec::new(),
            globals: Vec::new(),
            functions: Vec::new(),
        };

        while !self.at(&Token::Eof) {
            if self.at(&Token::Struct) && self.is_struct_def() {
                program.structs.push(self.parse_struct_def()?);
                self.expect(&Token::Semi)?;
                continue;
            }

            let is_static = self.eat(&Token::Static);
            let is_const = self.eat(&Token::Const);
            let _ = is_const;

            let base_ty = self.parse_base_type()?;
            let ty = self.parse_pointer_type(base_ty);
            let name = self.expect_ident()?;

            if self.at(&Token::LParen) {
                let (params, is_variadic) = self.parse_param_list()?;
                let body = if self.at(&Token::LBrace) {
                    Some(self.parse_block()?)
                } else {
                    self.expect(&Token::Semi)?;
                    None
                };
                program.functions.push(FuncDef {
                    name,
                    return_ty: ty,
                    params,
                    is_variadic,
                    body,
                    is_static,
                });
            } else {
                let (final_ty, init) = self.parse_var_suffix(ty)?;
                self.expect(&Token::Semi)?;
                program.globals.push(GlobalDecl {
                    name,
                    ty: final_ty,
                    init,
                });
            }
        }

        Ok(program)
    }

    fn is_struct_def(&self) -> bool {
        if let Token::Struct = self.peek() {
            if let Token::Ident(_) = self.tokens.get(self.pos + 1).unwrap_or(&Token::Eof) {
                if let Token::LBrace = self.tokens.get(self.pos + 2).unwrap_or(&Token::Eof) {
                    return true;
                }
            }
        }
        false
    }

    fn parse_struct_def(&mut self) -> Result<StructDef, String> {
        self.expect(&Token::Struct)?;
        let name = self.expect_ident()?;
        self.expect(&Token::LBrace)?;

        let mut fields = Vec::new();
        while !self.at(&Token::RBrace) {
            let base_ty = self.parse_base_type()?;
            let ty = self.parse_pointer_type(base_ty);
            let field_name = self.expect_ident()?;
            let final_ty = if self.eat(&Token::LBracket) {
                let size = self.parse_array_size()?;
                self.expect(&Token::RBracket)?;
                CType::Array(Box::new(ty), size)
            } else {
                ty
            };
            self.expect(&Token::Semi)?;
            fields.push((field_name, final_ty));
        }
        self.expect(&Token::RBrace)?;

        Ok(StructDef { name, fields })
    }

    fn parse_base_type(&mut self) -> Result<CType, String> {
        match self.peek().clone() {
            Token::Void => {
                self.advance();
                Ok(CType::Void)
            }
            Token::Int => {
                self.advance();
                Ok(CType::Int)
            }
            Token::Char => {
                self.advance();
                Ok(CType::Char)
            }
            Token::Struct => {
                self.advance();
                let name = self.expect_ident()?;
                Ok(CType::Struct(name))
            }
            _ => Err(format!(
                "Expected type, got {:?} at position {}",
                self.peek(),
                self.pos
            )),
        }
    }

    fn parse_pointer_type(&mut self, mut ty: CType) -> CType {
        while self.eat(&Token::Star) {
            ty = CType::Pointer(Box::new(ty));
        }
        ty
    }

    fn parse_param_list(&mut self) -> Result<(Vec<Param>, bool), String> {
        self.expect(&Token::LParen)?;
        let mut params = Vec::new();
        let mut is_variadic = false;

        if self.at(&Token::Void) {
            if let Token::RParen = *self.tokens.get(self.pos + 1).unwrap_or(&Token::Eof) {
                self.advance();
                self.advance();
                return Ok((params, false));
            }
        }

        if !self.at(&Token::RParen) {
            loop {
                if self.at(&Token::Ellipsis) {
                    self.advance();
                    is_variadic = true;
                    break;
                }

                let _const = self.eat(&Token::Const);
                let base_ty = self.parse_base_type()?;
                let ty = self.parse_pointer_type(base_ty);

                let name = if let Token::Ident(_) = self.peek() {
                    self.expect_ident()?
                } else {
                    String::new()
                };

                let final_ty = if self.eat(&Token::LBracket) {
                    if self.at(&Token::RBracket) {
                        self.advance();
                        CType::Pointer(Box::new(ty))
                    } else {
                        let size = self.parse_array_size()?;
                        self.expect(&Token::RBracket)?;
                        CType::Array(Box::new(ty), size)
                    }
                } else {
                    ty
                };

                params.push(Param { name, ty: final_ty });

                if !self.eat(&Token::Comma) {
                    break;
                }
            }
        }

        self.expect(&Token::RParen)?;
        Ok((params, is_variadic))
    }

    fn parse_var_suffix(&mut self, ty: CType) -> Result<(CType, Option<Expr>), String> {
        let final_ty = if self.eat(&Token::LBracket) {
            if self.at(&Token::RBracket) {
                self.advance();
                CType::Array(Box::new(ty), 0)
            } else {
                let size = self.parse_array_size()?;
                self.expect(&Token::RBracket)?;
                CType::Array(Box::new(ty), size)
            }
        } else {
            ty
        };

        let init = if self.eat(&Token::Eq) {
            if self.at(&Token::LBrace) {
                Some(self.parse_init_list()?)
            } else {
                Some(self.parse_expr()?)
            }
        } else {
            None
        };

        let final_ty = match (&final_ty, &init) {
            (CType::Array(elem, 0), Some(Expr::InitList(items))) => {
                CType::Array(elem.clone(), items.len() as u32)
            }
            _ => final_ty,
        };

        Ok((final_ty, init))
    }

    fn parse_init_list(&mut self) -> Result<Expr, String> {
        self.expect(&Token::LBrace)?;
        let mut items = Vec::new();
        if !self.at(&Token::RBrace) {
            items.push(self.parse_expr()?);
            while self.eat(&Token::Comma) {
                if self.at(&Token::RBrace) { break; }
                items.push(self.parse_expr()?);
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(Expr::InitList(items))
    }

    fn parse_array_size(&mut self) -> Result<u32, String> {
        match self.advance() {
            Token::IntLit(n) => Ok(n as u32),
            other => Err(format!("Expected array size (integer), got {:?}", other)),
        }
    }

    fn parse_block(&mut self) -> Result<Block, String> {
        self.expect(&Token::LBrace)?;
        let mut stmts = Vec::new();
        while !self.at(&Token::RBrace) && !self.at(&Token::Eof) {
            stmts.push(self.parse_stmt()?);
        }
        self.expect(&Token::RBrace)?;
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, String> {
        match self.peek().clone() {
            Token::LBrace => {
                let block = self.parse_block()?;
                Ok(Stmt::Block(block))
            }
            Token::If => self.parse_if(),
            Token::While => self.parse_while(),
            Token::Do => self.parse_do_while(),
            Token::For => self.parse_for(),
            Token::Return => self.parse_return(),
            Token::Switch => self.parse_switch(),
            Token::Break => {
                self.advance();
                self.expect(&Token::Semi)?;
                Ok(Stmt::Break)
            }
            _ if self.is_local_decl() => self.parse_local_decl(),
            _ => {
                let expr = self.parse_expr()?;
                self.expect(&Token::Semi)?;
                Ok(Stmt::Expr(expr))
            }
        }
    }

    fn is_local_decl(&self) -> bool {
        let mut i = self.pos;

        loop {
            match self.tokens.get(i).unwrap_or(&Token::Eof) {
                Token::Const | Token::Static => i += 1,
                _ => break,
            }
        }

        match self.tokens.get(i).unwrap_or(&Token::Eof) {
            Token::Int | Token::Char | Token::Void => {
                i += 1;
                while self.tokens.get(i) == Some(&Token::Star) {
                    i += 1;
                }
                matches!(self.tokens.get(i), Some(Token::Ident(_)))
            }
            Token::Struct => {
                i += 1;
                if let Some(Token::Ident(_)) = self.tokens.get(i) {
                    i += 1;
                    while self.tokens.get(i) == Some(&Token::Star) {
                        i += 1;
                    }
                    matches!(self.tokens.get(i), Some(Token::Ident(_)))
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn parse_local_decl(&mut self) -> Result<Stmt, String> {
        let _const = self.eat(&Token::Const);
        let _static = self.eat(&Token::Static);
        let base_ty = self.parse_base_type()?;

        let mut stmts: Vec<Stmt> = Vec::new();
        loop {
            let ty = self.parse_pointer_type(base_ty.clone());
            let name = self.expect_ident()?;
            let (final_ty, init) = self.parse_var_suffix(ty)?;
            stmts.push(Stmt::Local { name, ty: final_ty, init });

            if !self.eat(&Token::Comma) {
                break;
            }
        }
        self.expect(&Token::Semi)?;

        if stmts.len() == 1 {
            Ok(stmts.remove(0))
        } else {
            Ok(Stmt::Block(stmts))
        }
    }

    fn parse_if(&mut self) -> Result<Stmt, String> {
        self.expect(&Token::If)?;
        self.expect(&Token::LParen)?;
        let cond = self.parse_expr()?;
        self.expect(&Token::RParen)?;

        let then_body = if self.at(&Token::LBrace) {
            self.parse_block()?
        } else {
            vec![self.parse_stmt()?]
        };

        let else_body = if self.eat(&Token::Else) {
            if self.at(&Token::LBrace) {
                Some(self.parse_block()?)
            } else {
                Some(vec![self.parse_stmt()?])
            }
        } else {
            None
        };

        Ok(Stmt::If {
            cond,
            then_body,
            else_body,
        })
    }

    fn parse_switch(&mut self) -> Result<Stmt, String> {
        self.expect(&Token::Switch)?;
        self.expect(&Token::LParen)?;
        let expr = self.parse_expr()?;
        self.expect(&Token::RParen)?;
        self.expect(&Token::LBrace)?;

        let mut cases: Vec<(i32, Block)> = Vec::new();
        let mut default: Option<Block> = None;

        while !self.at(&Token::RBrace) && !self.at(&Token::Eof) {
            if self.eat(&Token::Case) {
                let val = match self.advance() {
                    Token::IntLit(n) => n,
                    Token::CharLit(n) => n,
                    Token::Minus => {
                        if let Token::IntLit(n) = self.advance() {
                            -n
                        } else {
                            return Err("Expected integer after '-' in case".to_string());
                        }
                    }
                    other => return Err(format!("Expected case value, got {:?}", other)),
                };
                self.expect(&Token::Colon)?;
                let body = self.parse_case_body()?;
                cases.push((val, body));
            } else if self.eat(&Token::Default) {
                self.expect(&Token::Colon)?;
                default = Some(self.parse_case_body()?);
            } else {
                return Err(format!("Expected 'case' or 'default' in switch, got {:?}", self.peek()));
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(Stmt::Switch { expr, cases, default })
    }

    fn parse_case_body(&mut self) -> Result<Block, String> {
        let mut stmts = Vec::new();
        loop {
            match self.peek() {
                Token::Case | Token::Default | Token::RBrace | Token::Eof => break,
                _ => stmts.push(self.parse_stmt()?),
            }
        }
        Ok(stmts)
    }

    fn parse_while(&mut self) -> Result<Stmt, String> {
        self.expect(&Token::While)?;
        self.expect(&Token::LParen)?;
        let cond = self.parse_expr()?;
        self.expect(&Token::RParen)?;
        let body = if self.at(&Token::LBrace) {
            self.parse_block()?
        } else {
            vec![self.parse_stmt()?]
        };
        Ok(Stmt::While { cond, body })
    }

    fn parse_do_while(&mut self) -> Result<Stmt, String> {
        self.expect(&Token::Do)?;
        let body = self.parse_block()?;
        self.expect(&Token::While)?;
        self.expect(&Token::LParen)?;
        let cond = self.parse_expr()?;
        self.expect(&Token::RParen)?;
        self.expect(&Token::Semi)?;
        Ok(Stmt::DoWhile { body, cond })
    }

    fn parse_for(&mut self) -> Result<Stmt, String> {
        self.expect(&Token::For)?;
        self.expect(&Token::LParen)?;

        let init = if self.at(&Token::Semi) {
            self.advance();
            None
        } else if self.is_local_decl() {
            Some(Box::new(self.parse_local_decl()?))
        } else {
            let expr = self.parse_expr()?;
            self.expect(&Token::Semi)?;
            Some(Box::new(Stmt::Expr(expr)))
        };

        let cond = if self.at(&Token::Semi) {
            None
        } else {
            Some(self.parse_expr()?)
        };
        self.expect(&Token::Semi)?;

        let step = if self.at(&Token::RParen) {
            None
        } else {
            Some(self.parse_expr()?)
        };
        self.expect(&Token::RParen)?;

        let body = if self.at(&Token::LBrace) {
            self.parse_block()?
        } else {
            vec![self.parse_stmt()?]
        };

        Ok(Stmt::For {
            init,
            cond,
            step,
            body,
        })
    }

    fn parse_return(&mut self) -> Result<Stmt, String> {
        self.expect(&Token::Return)?;
        let val = if self.at(&Token::Semi) {
            None
        } else {
            Some(self.parse_expr()?)
        };
        self.expect(&Token::Semi)?;
        Ok(Stmt::Return(val))
    }

    pub fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_assign()
    }

    fn parse_assign(&mut self) -> Result<Expr, String> {
        let lhs = self.parse_logical_or()?;

        match self.peek() {
            Token::Eq => {
                self.advance();
                let rhs = self.parse_assign()?;
                Ok(Expr::Assign {
                    target: Box::new(lhs),
                    value: Box::new(rhs),
                })
            }
            Token::PlusEq => {
                self.advance();
                let rhs = self.parse_assign()?;
                Ok(Expr::CompoundAssign { op: BinOp::Add, target: Box::new(lhs), value: Box::new(rhs) })
            }
            Token::MinusEq => {
                self.advance();
                let rhs = self.parse_assign()?;
                Ok(Expr::CompoundAssign { op: BinOp::Sub, target: Box::new(lhs), value: Box::new(rhs) })
            }
            Token::StarEq => {
                self.advance();
                let rhs = self.parse_assign()?;
                Ok(Expr::CompoundAssign { op: BinOp::Mul, target: Box::new(lhs), value: Box::new(rhs) })
            }
            Token::SlashEq => {
                self.advance();
                let rhs = self.parse_assign()?;
                Ok(Expr::CompoundAssign { op: BinOp::Div, target: Box::new(lhs), value: Box::new(rhs) })
            }
            Token::PercentEq => {
                self.advance();
                let rhs = self.parse_assign()?;
                Ok(Expr::CompoundAssign { op: BinOp::Mod, target: Box::new(lhs), value: Box::new(rhs) })
            }
            _ => Ok(lhs),
        }
    }

    fn parse_logical_or(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_logical_and()?;
        while self.at(&Token::PipePipe) {
            self.advance();
            let rhs = self.parse_logical_and()?;
            lhs = Expr::BinOp { op: BinOp::LogicalOr, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn parse_logical_and(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_equality()?;
        while self.at(&Token::AmpAmp) {
            self.advance();
            let rhs = self.parse_equality()?;
            lhs = Expr::BinOp { op: BinOp::LogicalAnd, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn parse_equality(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_relational()?;
        loop {
            let op = match self.peek() {
                Token::EqEq => BinOp::Eq,
                Token::BangEq => BinOp::Neq,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_relational()?;
            lhs = Expr::BinOp { op, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn parse_relational(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_additive()?;
        loop {
            let op = match self.peek() {
                Token::Lt => BinOp::Lt,
                Token::LtEq => BinOp::LtEq,
                Token::Gt => BinOp::Gt,
                Token::GtEq => BinOp::GtEq,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_additive()?;
            lhs = Expr::BinOp { op, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn parse_additive(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_multiplicative()?;
        loop {
            let op = match self.peek() {
                Token::Plus => BinOp::Add,
                Token::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_multiplicative()?;
            lhs = Expr::BinOp { op, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Token::Star => BinOp::Mul,
                Token::Slash => BinOp::Div,
                Token::Percent => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_unary()?;
            lhs = Expr::BinOp { op, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        match self.peek().clone() {
            Token::Minus => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::UnaryOp { op: UnaryOp::Neg, expr: Box::new(expr) })
            }
            Token::Bang => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::UnaryOp { op: UnaryOp::Not, expr: Box::new(expr) })
            }
            Token::Star => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::Deref(Box::new(expr)))
            }
            Token::Amp => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::AddrOf(Box::new(expr)))
            }
            Token::PlusPlus => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::PreIncDec { op: IncDec::Inc, expr: Box::new(expr) })
            }
            Token::MinusMinus => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::PreIncDec { op: IncDec::Dec, expr: Box::new(expr) })
            }
            Token::LParen if self.is_cast() => {
                self.advance();
                let ty = self.parse_full_type()?;
                self.expect(&Token::RParen)?;
                let expr = self.parse_unary()?;
                Ok(Expr::Cast { ty, expr: Box::new(expr) })
            }
            Token::Sizeof => {
                self.advance();
                if self.eat(&Token::LParen) {
                    if self.peek().is_type_keyword() || *self.peek() == Token::Const {
                        let ty = self.parse_full_type()?;
                        self.expect(&Token::RParen)?;
                        Ok(Expr::SizeofType(ty))
                    } else {
                        let expr = self.parse_expr()?;
                        self.expect(&Token::RParen)?;
                        Ok(Expr::SizeofExpr(Box::new(expr)))
                    }
                } else {
                    let expr = self.parse_unary()?;
                    Ok(Expr::SizeofExpr(Box::new(expr)))
                }
            }
            _ => self.parse_postfix(),
        }
    }

    fn is_cast(&self) -> bool {
        if self.peek() != &Token::LParen {
            return false;
        }
        let next = self.tokens.get(self.pos + 1).unwrap_or(&Token::Eof);
        match next {
            Token::Int | Token::Char | Token::Void | Token::Struct | Token::Const => true,
            _ => false,
        }
    }

    fn parse_full_type(&mut self) -> Result<CType, String> {
        let _const = self.eat(&Token::Const);
        let base = self.parse_base_type()?;
        Ok(self.parse_pointer_type(base))
    }

    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;
        loop {
            match self.peek() {
                Token::LParen => {
                    if let Expr::Var(name) = expr {
                        self.advance();
                        let args = self.parse_arg_list()?;
                        self.expect(&Token::RParen)?;
                        expr = Expr::Call { func: name, args };
                    } else {
                        break;
                    }
                }
                Token::LBracket => {
                    self.advance();
                    let index = self.parse_expr()?;
                    self.expect(&Token::RBracket)?;
                    expr = Expr::Index { array: Box::new(expr), index: Box::new(index) };
                }
                Token::Dot => {
                    self.advance();
                    let name = self.expect_ident()?;
                    expr = Expr::Field { expr: Box::new(expr), name };
                }
                Token::Arrow => {
                    self.advance();
                    let name = self.expect_ident()?;
                    expr = Expr::ArrowField { expr: Box::new(expr), name };
                }
                Token::PlusPlus => {
                    self.advance();
                    expr = Expr::PostIncDec { op: IncDec::Inc, expr: Box::new(expr) };
                }
                Token::MinusMinus => {
                    self.advance();
                    expr = Expr::PostIncDec { op: IncDec::Dec, expr: Box::new(expr) };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_arg_list(&mut self) -> Result<Vec<Expr>, String> {
        let mut args = Vec::new();
        if !self.at(&Token::RParen) {
            args.push(self.parse_expr()?);
            while self.eat(&Token::Comma) {
                args.push(self.parse_expr()?);
            }
        }
        Ok(args)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.peek().clone() {
            Token::IntLit(n) => { self.advance(); Ok(Expr::IntLit(n)) }
            Token::CharLit(c) => { self.advance(); Ok(Expr::CharLit(c)) }
            Token::StrLit(s) => { self.advance(); Ok(Expr::StrLit(s)) }
            Token::Ident(name) => { self.advance(); Ok(Expr::Var(name)) }
            Token::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                Ok(expr)
            }
            other => Err(format!(
                "Unexpected token {:?} at position {} (expected expression)",
                other, self.pos
            )),
        }
    }
}
