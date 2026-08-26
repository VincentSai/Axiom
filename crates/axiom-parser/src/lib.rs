//! Pure parsing entry points for Axiom source code.

use std::fmt;

use axiom_ast::{
    BinaryOperator, EffectDecl, EffectOp, Expr, FunctionDecl, FunctionParam, Literal, Module, Type,
    UnaryOperator,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} at line {}, column {}",
            self.message, self.line, self.column
        )
    }
}

impl std::error::Error for ParseError {}

pub fn parse_module(source: &str) -> Result<Module, ParseError> {
    let mut parser = Parser::new(source);
    parser.skip_whitespace();
    let mut functions = Vec::new();
    let mut effects = Vec::new();
    while !parser.is_at_end() {
        if parser.starts_word("fn") {
            functions.push(parser.parse_function()?);
        } else if parser.starts_word("effect") {
            effects.push(parser.parse_effect()?);
        } else if parser.starts_word("type") {
            parser.skip_type_declaration()?;
        } else {
            return Err(parser.error("expected `type`, `fn`, or `effect` declaration"));
        }
        parser.skip_whitespace();
    }
    Ok(Module { functions, effects })
}

pub fn parse_expr(source: &str) -> Result<Expr, ParseError> {
    let mut parser = Parser::new(source);
    let expr = parser.parse_expression()?;
    parser.skip_whitespace();
    if !parser.is_at_end() {
        return Err(parser.error("expected end of input"));
    }
    Ok(expr)
}

struct Parser<'a> {
    source: &'a str,
    offset: usize,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str) -> Self {
        Self { source, offset: 0 }
    }

    fn parse_function(&mut self) -> Result<FunctionDecl, ParseError> {
        self.expect_word("fn")?;
        let name = self.parse_ident()?;
        self.expect_char('(')?;
        let mut params = Vec::new();
        self.skip_whitespace();
        if self.peek_char() != Some(')') {
            loop {
                let param_name = self.parse_ident()?;
                self.expect_char(':')?;
                let ty = self.parse_type()?;
                params.push(FunctionParam {
                    name: param_name,
                    ty,
                });
                self.skip_whitespace();
                if self.peek_char() != Some(',') {
                    break;
                }
                self.bump_char();
            }
        }
        self.expect_char(')')?;
        self.expect_text("->")?;
        let return_type = self.parse_type()?;
        let mut effects = Vec::new();
        let mut requires = Vec::new();
        let mut ensures = Vec::new();
        loop {
            if self.starts_word("with") {
                self.expect_word("with")?;
                self.expect_char('[')?;
                if !self.consume_char(']') {
                    loop {
                        effects.push(self.parse_ident()?);
                        if !self.consume_char(',') {
                            break;
                        }
                    }
                    self.expect_char(']')?;
                }
            } else if self.starts_word("requires") {
                self.expect_word("requires")?;
                requires.extend(self.parse_contract_block()?);
            } else if self.starts_word("ensures") {
                self.expect_word("ensures")?;
                ensures.extend(self.parse_contract_block()?);
            } else {
                break;
            }
        }
        self.expect_char('{')?;
        let body_start = self.offset;
        let parsed_body = self
            .parse_block_contents()
            .and_then(|body| self.expect_char('}').map(|()| body));
        let body = match parsed_body {
            Ok(body) => body,
            Err(error) => {
                self.offset = body_start;
                let raw = self.capture_block_contents()?;
                if !raw.contains("match ") && !raw.contains("if ") {
                    return Err(error);
                }
                Expr::Raw(raw.trim().to_owned())
            }
        };
        Ok(FunctionDecl {
            name,
            params,
            return_type,
            effects,
            requires,
            ensures,
            body,
        })
    }

    fn skip_type_declaration(&mut self) -> Result<(), ParseError> {
        self.expect_word("type")?;
        self.parse_ident()?;
        self.expect_char('=')?;

        let mut braces = 0usize;
        let mut parentheses = 0usize;
        let mut brackets = 0usize;
        loop {
            self.skip_whitespace();
            if self.is_at_end() {
                return Ok(());
            }
            if braces == 0
                && parentheses == 0
                && brackets == 0
                && (self.starts_word("type")
                    || self.starts_word("effect")
                    || self.starts_word("fn"))
            {
                return Ok(());
            }

            match self.bump_char() {
                Some('{') => braces += 1,
                Some('}') => braces = braces.saturating_sub(1),
                Some('(') => parentheses += 1,
                Some(')') => parentheses = parentheses.saturating_sub(1),
                Some('[') => brackets += 1,
                Some(']') => brackets = brackets.saturating_sub(1),
                Some(_) => {}
                None => return Ok(()),
            }
        }
    }

    fn capture_block_contents(&mut self) -> Result<String, ParseError> {
        let start = self.offset;
        let mut depth = 1usize;
        while !self.is_at_end() {
            if self.source[self.offset..].starts_with("//") {
                self.offset += 2;
                while !matches!(self.peek_char(), None | Some('\n')) {
                    self.bump_char();
                }
                continue;
            }
            if self.source[self.offset..].starts_with("/*") {
                self.offset += 2;
                while !self.is_at_end() && !self.source[self.offset..].starts_with("*/") {
                    self.bump_char();
                }
                if self.is_at_end() {
                    return Err(self.error("unterminated block comment"));
                }
                self.offset += 2;
                continue;
            }
            if self.peek_char() == Some('"') {
                self.bump_char();
                loop {
                    match self.bump_char() {
                        Some('\\') => {
                            if self.bump_char().is_none() {
                                return Err(self.error("unterminated string literal"));
                            }
                        }
                        Some('"') => break,
                        Some(_) => {}
                        None => return Err(self.error("unterminated string literal")),
                    }
                }
                continue;
            }

            match self.bump_char() {
                Some('{') => depth += 1,
                Some('}') => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(self.source[start..self.offset - 1].to_owned());
                    }
                }
                Some(_) => {}
                None => break,
            }
        }
        Err(self.error("expected `}`"))
    }

    fn parse_contract_block(&mut self) -> Result<Vec<Expr>, ParseError> {
        self.expect_char('{')?;
        let mut clauses = Vec::new();
        if !self.consume_char('}') {
            loop {
                clauses.push(self.parse_expression()?);
                if !self.consume_char(',') {
                    break;
                }
                if self.consume_char('}') {
                    return Ok(clauses);
                }
            }
            self.expect_char('}')?;
        }
        Ok(clauses)
    }

    fn parse_effect(&mut self) -> Result<EffectDecl, ParseError> {
        self.expect_word("effect")?;
        let name = self.parse_ident()?;
        self.expect_char('{')?;
        let mut operations = Vec::new();

        loop {
            self.skip_whitespace();
            if self.consume_char('}') {
                break;
            }

            self.expect_word("fn")?;
            let operation_name = self.parse_ident()?;
            self.expect_char('(')?;
            let params = self.parse_params()?;
            self.expect_char(')')?;
            if !self.consume_char(':') {
                self.expect_text("->")?;
            }
            let return_type = self.parse_type()?;
            self.consume_char(';');
            operations.push(EffectOp {
                name: operation_name,
                params,
                return_type,
            });
        }

        Ok(EffectDecl { name, operations })
    }

    fn parse_params(&mut self) -> Result<Vec<FunctionParam>, ParseError> {
        let mut params = Vec::new();
        self.skip_whitespace();
        if self.peek_char() == Some(')') {
            return Ok(params);
        }

        loop {
            let name = self.parse_ident()?;
            self.expect_char(':')?;
            let ty = self.parse_type()?;
            params.push(FunctionParam { name, ty });
            if !self.consume_char(',') {
                break;
            }
        }
        Ok(params)
    }

    fn parse_block_contents(&mut self) -> Result<Expr, ParseError> {
        let mut statements = Vec::new();
        let tail = loop {
            self.skip_whitespace();
            if self.peek_char() == Some('}') {
                break None;
            }
            if self.starts_word("return") {
                self.expect_word("return")?;
                let value = self.parse_expression()?;
                self.consume_char(';');
                self.skip_whitespace();
                if self.peek_char() != Some('}') {
                    return Err(self.error("expected `}` after return value"));
                }
                break Some(Box::new(value));
            }
            if self.starts_word("let") {
                self.expect_word("let")?;
                let name = self.parse_ident()?;
                self.expect_char('=')?;
                let value = self.parse_expression()?;
                self.expect_char(';')?;
                statements.push(Expr::Let {
                    name,
                    value: Box::new(value),
                });
                continue;
            }

            let expr = self.parse_expression()?;
            if self.consume_char(';') {
                statements.push(expr);
            } else {
                break Some(Box::new(expr));
            }
        };

        if statements.is_empty() {
            if let Some(tail) = tail {
                return Ok(*tail);
            }
        }
        Ok(Expr::Block { statements, tail })
    }

    fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        self.parse_implication()
    }

    fn parse_implication(&mut self) -> Result<Expr, ParseError> {
        let left = self.parse_comparison()?;
        if self.consume_text("=>") {
            return Ok(Expr::BinaryOp {
                left: Box::new(left),
                op: BinaryOperator::Implies,
                right: Box::new(self.parse_implication()?),
            });
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_additive()?;
        let op = if self.consume_text(">=") {
            Some(BinaryOperator::GreaterThanOrEqual)
        } else if self.consume_text("==") {
            Some(BinaryOperator::Equal)
        } else if self.consume_text("!=") {
            Some(BinaryOperator::NotEqual)
        } else if self.consume_char('>') {
            Some(BinaryOperator::GreaterThan)
        } else if self.starts_word("is") {
            self.expect_word("is")?;
            Some(BinaryOperator::Is)
        } else {
            None
        };
        if let Some(op) = op {
            let right = self.parse_additive()?;
            expr = Expr::BinaryOp {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_additive(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_multiplicative()?;
        loop {
            let op = if self.consume_char('+') {
                BinaryOperator::Add
            } else if self.consume_char('-') {
                BinaryOperator::Subtract
            } else {
                break;
            };
            let right = self.parse_multiplicative()?;
            expr = Expr::BinaryOp {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_unary()?;
        loop {
            let op = if self.consume_char('*') {
                BinaryOperator::Multiply
            } else if self.consume_char('/') {
                BinaryOperator::Divide
            } else {
                break;
            };
            let right = self.parse_unary()?;
            expr = Expr::BinaryOp {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        if self.consume_char('-') {
            return Ok(Expr::UnaryOp {
                op: UnaryOperator::Negate,
                operand: Box::new(self.parse_unary()?),
            });
        }
        if self.consume_char('!') {
            return Ok(Expr::UnaryOp {
                op: UnaryOperator::Not,
                operand: Box::new(self.parse_unary()?),
            });
        }
        self.parse_call()
    }

    fn parse_call(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;
        while self.consume_char('(') {
            let mut args = Vec::new();
            if !self.consume_char(')') {
                loop {
                    args.push(self.parse_expression()?);
                    if !self.consume_char(',') {
                        break;
                    }
                }
                self.expect_char(')')?;
            }
            expr = Expr::Call {
                callee: Box::new(expr),
                args,
            };
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        self.skip_whitespace();
        if self.consume_char('(') {
            if self.consume_char(')') {
                return Ok(Expr::Literal(Literal::Unit));
            }
            let expr = self.parse_expression()?;
            self.expect_char(')')?;
            return Ok(expr);
        }
        if self.peek_char() == Some('"') {
            return self.parse_string();
        }
        if matches!(self.peek_char(), Some(ch) if ch.is_ascii_digit()) {
            return self.parse_number();
        }
        if matches!(self.peek_char(), Some(ch) if ch == '_' || ch.is_ascii_alphabetic()) {
            let ident = self.parse_ident()?;
            return Ok(match ident.as_str() {
                "true" => Expr::Literal(Literal::Bool(true)),
                "false" => Expr::Literal(Literal::Bool(false)),
                _ => Expr::Ident(ident),
            });
        }
        match self.peek_char() {
            Some(ch) => Err(self.error(format!("unexpected token `{ch}`"))),
            None => Err(self.error("unexpected end of input")),
        }
    }

    fn parse_string(&mut self) -> Result<Expr, ParseError> {
        self.expect_char('"')?;
        let mut value = String::new();
        loop {
            match self.bump_char() {
                Some('"') => return Ok(Expr::Literal(Literal::String(value))),
                Some('\\') => match self.bump_char() {
                    Some('n') => value.push('\n'),
                    Some('r') => value.push('\r'),
                    Some('t') => value.push('\t'),
                    Some('"') => value.push('"'),
                    Some('\\') => value.push('\\'),
                    Some(ch) => return Err(self.error(format!("unsupported escape `\\{ch}`"))),
                    None => return Err(self.error("unterminated string literal")),
                },
                Some(ch) => value.push(ch),
                None => return Err(self.error("unterminated string literal")),
            }
        }
    }

    fn parse_number(&mut self) -> Result<Expr, ParseError> {
        self.skip_whitespace();
        let start = self.offset;
        while matches!(self.peek_char(), Some(ch) if ch.is_ascii_digit()) {
            self.bump_char();
        }
        let has_decimal_point = self.peek_char() == Some('.');
        if has_decimal_point {
            self.bump_char();
            while matches!(self.peek_char(), Some(ch) if ch.is_ascii_digit()) {
                self.bump_char();
            }
        }
        let text = &self.source[start..self.offset];
        if has_decimal_point {
            let value = text
                .parse::<f64>()
                .map_err(|_| self.error(format!("invalid number `{text}`")))?;
            Ok(Expr::Literal(Literal::Float(value)))
        } else {
            let value = text
                .parse::<i64>()
                .map_err(|_| self.error(format!("invalid number `{text}`")))?;
            Ok(Expr::Literal(Literal::Int(value)))
        }
    }

    fn parse_type(&mut self) -> Result<Type, ParseError> {
        if self.consume_char('(') {
            self.expect_char(')')?;
            return Ok(Type::Unit);
        }
        let mut name = self.parse_ident()?;
        let simple_type = match name.as_str() {
            "Int" => Ok(Type::Int),
            "Float" => Ok(Type::Float),
            "Decimal" => Ok(Type::Decimal),
            "Bool" => Ok(Type::Bool),
            "String" => Ok(Type::String),
            _ => Ok(Type::Custom(name.clone())),
        };
        if !self.consume_char('<') {
            return simple_type;
        }

        let mut arguments = Vec::new();
        loop {
            arguments.push(self.parse_type()?);
            if !self.consume_char(',') {
                break;
            }
        }
        self.expect_char('>')?;
        name.push('<');
        name.push_str(
            &arguments
                .iter()
                .map(Self::format_type)
                .collect::<Vec<_>>()
                .join(", "),
        );
        name.push('>');
        Ok(Type::Custom(name))
    }

    fn format_type(ty: &Type) -> String {
        match ty {
            Type::Int => "Int".into(),
            Type::Float => "Float".into(),
            Type::Decimal => "Decimal".into(),
            Type::Bool => "Bool".into(),
            Type::String => "String".into(),
            Type::Unit => "()".into(),
            Type::Custom(name) => name.clone(),
        }
    }

    fn parse_ident(&mut self) -> Result<String, ParseError> {
        self.skip_whitespace();
        let start = self.offset;
        while matches!(self.peek_char(), Some(ch) if ch == '_' || ch.is_ascii_alphanumeric()) {
            self.bump_char();
        }
        if self.offset == start {
            return Err(self.error("expected identifier"));
        }
        Ok(self.source[start..self.offset].to_owned())
    }

    fn expect_word(&mut self, expected: &str) -> Result<(), ParseError> {
        self.expect_text(expected)?;
        if matches!(self.peek_char(), Some(ch) if ch == '_' || ch.is_ascii_alphanumeric()) {
            return Err(self.error(format!("expected keyword `{expected}`")));
        }
        Ok(())
    }

    fn starts_word(&mut self, expected: &str) -> bool {
        self.skip_whitespace();
        let remaining = &self.source[self.offset..];
        remaining.starts_with(expected)
            && !matches!(remaining[expected.len()..].chars().next(), Some(ch) if ch == '_' || ch.is_ascii_alphanumeric())
    }

    fn expect_text(&mut self, expected: &str) -> Result<(), ParseError> {
        self.skip_whitespace();
        if self.source[self.offset..].starts_with(expected) {
            self.offset += expected.len();
            Ok(())
        } else {
            Err(self.error(format!("expected `{expected}`")))
        }
    }

    fn consume_text(&mut self, expected: &str) -> bool {
        self.skip_whitespace();
        if self.source[self.offset..].starts_with(expected) {
            self.offset += expected.len();
            true
        } else {
            false
        }
    }

    fn expect_char(&mut self, expected: char) -> Result<(), ParseError> {
        self.skip_whitespace();
        if self.peek_char() == Some(expected) {
            self.bump_char();
            Ok(())
        } else {
            Err(self.error(format!("expected `{expected}`")))
        }
    }

    fn consume_char(&mut self, expected: char) -> bool {
        self.skip_whitespace();
        if self.peek_char() == Some(expected) {
            self.bump_char();
            true
        } else {
            false
        }
    }

    fn skip_whitespace(&mut self) {
        loop {
            while matches!(self.peek_char(), Some(ch) if ch.is_whitespace() || ch == '\u{feff}') {
                self.bump_char();
            }
            if self.source[self.offset..].starts_with("//") {
                self.offset += 2;
                while !matches!(self.peek_char(), None | Some('\n')) {
                    self.bump_char();
                }
                continue;
            }
            if self.source[self.offset..].starts_with("/*") {
                self.offset += 2;
                while !self.is_at_end() && !self.source[self.offset..].starts_with("*/") {
                    self.bump_char();
                }
                if !self.is_at_end() {
                    self.offset += 2;
                }
                continue;
            }
            break;
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.source[self.offset..].chars().next()
    }

    fn bump_char(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        self.offset += ch.len_utf8();
        Some(ch)
    }

    fn is_at_end(&self) -> bool {
        self.offset == self.source.len()
    }

    fn error(&self, message: impl Into<String>) -> ParseError {
        let before = &self.source[..self.offset];
        let line = before.chars().filter(|ch| *ch == '\n').count() + 1;
        let column = before
            .rsplit_once('\n')
            .map_or(before.chars().count() + 1, |(_, tail)| {
                tail.chars().count() + 1
            });
        ParseError {
            line,
            column,
            message: message.into(),
        }
    }
}
