//! Bidirectional source projection for content-addressed Axiom modules.

use std::error::Error;
use std::fmt::{self, Write};

use axiom_ast::{
    BinaryOperator, EffectDecl, Expr, FieldDef, FunctionDecl, FunctionParam, Literal, MatchArm,
    Module, Pattern, Type, TypeDef, UnaryOperator, VariantDef,
};
use axiom_db::{AstDatabase, DbError, ModuleRecord, NodeHash};
use axiom_parser::{parse_module, ParseError};

/// The result of parsing source and synchronizing it into the AST database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncSummary {
    pub module_hash: NodeHash,
    pub type_hashes: Vec<NodeHash>,
    pub function_hashes: Vec<NodeHash>,
    pub effect_hashes: Vec<NodeHash>,
    pub type_count: usize,
    pub function_count: usize,
    pub effect_count: usize,
}

impl From<ModuleRecord> for SyncSummary {
    fn from(record: ModuleRecord) -> Self {
        Self {
            type_count: record.type_hashes.len(),
            function_count: record.function_hashes.len(),
            effect_count: record.effect_hashes.len(),
            module_hash: record.module_hash,
            type_hashes: record.type_hashes,
            function_hashes: record.function_hashes,
            effect_hashes: record.effect_hashes,
        }
    }
}

/// Failures encountered while projecting modules between source and the AST database.
#[derive(Debug)]
pub enum VfsError {
    Parse(ParseError),
    Database(DbError),
    ModuleNotFound(NodeHash),
}

impl fmt::Display for VfsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => write!(formatter, "invalid Axiom source: {error}"),
            Self::Database(error) => write!(formatter, "AST database error: {error}"),
            Self::ModuleNotFound(hash) => {
                write!(
                    formatter,
                    "module `{hash}` was not found in the AST database"
                )
            }
        }
    }
}

impl Error for VfsError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Parse(error) => Some(error),
            Self::Database(error) => Some(error),
            Self::ModuleNotFound(_) => None,
        }
    }
}

impl From<ParseError> for VfsError {
    fn from(error: ParseError) -> Self {
        Self::Parse(error)
    }
}

impl From<DbError> for VfsError {
    fn from(error: DbError) -> Self {
        Self::Database(error)
    }
}

/// Exports a module as deterministic, parser-compatible Axiom source.
pub fn export_module_to_source(module: &Module) -> String {
    Projection::default().module(module)
}

/// Retrieves a module by content hash and exports it as Axiom source.
pub fn export_database_module(
    db: &AstDatabase,
    module_hash: &NodeHash,
) -> Result<String, VfsError> {
    let module = db
        .get_module(module_hash)?
        .ok_or_else(|| VfsError::ModuleNotFound(module_hash.clone()))?;
    Ok(export_module_to_source(&module))
}

/// Parses Axiom source and inserts the resulting module into the AST database.
pub fn sync_source_to_database(
    db: &mut AstDatabase,
    source: &str,
) -> Result<SyncSummary, VfsError> {
    let module = parse_module(source)?;
    Ok(db.insert_module(&module)?.into())
}

#[derive(Default)]
struct Projection {
    source: String,
}

impl Projection {
    fn module(mut self, module: &Module) -> String {
        let mut first = true;
        for definition in &module.types {
            self.separate_declaration(&mut first);
            self.type_definition(definition);
        }
        for effect in &module.effects {
            self.separate_declaration(&mut first);
            self.effect(effect);
        }
        for function in &module.functions {
            self.separate_declaration(&mut first);
            self.function(function);
        }
        self.source
    }

    fn separate_declaration(&mut self, first: &mut bool) {
        if *first {
            *first = false;
        } else {
            self.source.push_str("\n\n");
        }
    }

    fn type_definition(&mut self, definition: &TypeDef) {
        match definition {
            TypeDef::Record { name, fields } => {
                writeln!(self.source, "type {name} = {{").expect("writing to a String cannot fail");
                for field in fields {
                    writeln!(
                        self.source,
                        "    {}: {},",
                        field.name,
                        format_type(&field.ty)
                    )
                    .expect("writing to a String cannot fail");
                }
                self.source.push('}');
            }
            TypeDef::Enum { name, variants } => {
                write!(self.source, "type {name} =").expect("writing to a String cannot fail");
                for variant in variants {
                    self.source.push_str("\n    | ");
                    self.variant(variant);
                }
            }
        }
    }

    fn variant(&mut self, variant: &VariantDef) {
        self.source.push_str(&variant.name);
        if !variant.fields.is_empty() {
            self.source.push('(');
            self.fields(&variant.fields);
            self.source.push(')');
        }
    }

    fn fields(&mut self, fields: &[FieldDef]) {
        for (index, field) in fields.iter().enumerate() {
            if index > 0 {
                self.source.push_str(", ");
            }
            write!(self.source, "{}: {}", field.name, format_type(&field.ty))
                .expect("writing to a String cannot fail");
        }
    }

    fn effect(&mut self, effect: &EffectDecl) {
        writeln!(self.source, "effect {} {{", effect.name)
            .expect("writing to a String cannot fail");
        for operation in &effect.operations {
            write!(self.source, "    fn {}(", operation.name)
                .expect("writing to a String cannot fail");
            self.params(&operation.params);
            writeln!(self.source, "): {}", format_type(&operation.return_type))
                .expect("writing to a String cannot fail");
        }
        self.source.push('}');
    }

    fn function(&mut self, function: &FunctionDecl) {
        write!(self.source, "fn {}(", function.name).expect("writing to a String cannot fail");
        self.params(&function.params);
        write!(self.source, ") -> {}", format_type(&function.return_type))
            .expect("writing to a String cannot fail");

        if !function.effects.is_empty() {
            write!(self.source, "\n    with [{}]", function.effects.join(", "))
                .expect("writing to a String cannot fail");
        }
        self.contract("requires", &function.requires);
        self.contract("ensures", &function.ensures);
        self.source.push('\n');
        self.function_body(&function.body);
    }

    fn params(&mut self, params: &[FunctionParam]) {
        for (index, param) in params.iter().enumerate() {
            if index > 0 {
                self.source.push_str(", ");
            }
            write!(self.source, "{}: {}", param.name, format_type(&param.ty))
                .expect("writing to a String cannot fail");
        }
    }

    fn contract(&mut self, keyword: &str, clauses: &[Expr]) {
        if clauses.is_empty() {
            return;
        }
        write!(self.source, "\n    {keyword} {{").expect("writing to a String cannot fail");
        for clause in clauses {
            write!(self.source, "\n        {},", format_expr(clause, 0))
                .expect("writing to a String cannot fail");
        }
        self.source.push_str("\n    }");
    }

    fn function_body(&mut self, body: &Expr) {
        self.source.push('{');
        match body {
            Expr::Raw(raw) => {
                self.source.push('\n');
                self.source.push_str(raw);
                self.source.push('\n');
            }
            Expr::Block { statements, tail } => {
                for statement in statements {
                    self.source.push_str("\n    ");
                    match statement {
                        Expr::Let { name, value } => {
                            write!(self.source, "let {name} = {};", format_expr(value, 0))
                                .expect("writing to a String cannot fail");
                        }
                        expression => {
                            write!(self.source, "{};", format_expr(expression, 0))
                                .expect("writing to a String cannot fail");
                        }
                    }
                }
                if let Some(tail) = tail {
                    write!(self.source, "\n    {}", format_expr(tail, 0))
                        .expect("writing to a String cannot fail");
                }
                self.source.push('\n');
            }
            expression => {
                write!(self.source, "\n    {}\n", format_expr(expression, 0))
                    .expect("writing to a String cannot fail");
            }
        }
        self.source.push('}');
    }
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

fn format_expr(expr: &Expr, context_precedence: u8) -> String {
    let precedence = expr_precedence(expr);
    let mut result = match expr {
        Expr::Literal(literal) => format_literal(literal),
        Expr::Ident(name) => name.clone(),
        Expr::Hole(name) => format!("?{name}"),
        Expr::BinaryOp { left, op, right } => {
            let left_context = if matches!(
                op,
                BinaryOperator::Implies
                    | BinaryOperator::GreaterThan
                    | BinaryOperator::GreaterThanOrEqual
                    | BinaryOperator::LessThan
                    | BinaryOperator::LessThanOrEqual
                    | BinaryOperator::Equal
                    | BinaryOperator::NotEqual
                    | BinaryOperator::Is
            ) {
                precedence + 1
            } else {
                precedence
            };
            let right_context = if matches!(op, BinaryOperator::Implies) {
                precedence
            } else {
                precedence + 1
            };
            format!(
                "{} {} {}",
                format_expr(left, left_context),
                binary_operator(op),
                format_expr(right, right_context)
            )
        }
        Expr::UnaryOp { op, operand } => {
            format!("{}{}", unary_operator(op), format_expr(operand, precedence))
        }
        Expr::Let { name, value } => format!("let {name} = {}", format_expr(value, 0)),
        Expr::Block { statements, tail } => format_inline_block(statements, tail.as_deref()),
        Expr::Call { callee, args } => {
            let args = args
                .iter()
                .map(|argument| format_expr(argument, 0))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}({args})", format_expr(callee, precedence))
        }
        Expr::FieldAccess { target, field } => {
            format!("{}.{field}", format_expr(target, precedence))
        }
        Expr::RecordInit { ty_name, fields } => {
            let fields = format_record_fields(fields);
            match ty_name {
                Some(name) => format!("{name} {{ {fields} }}"),
                None => format!("{{ {fields} }}"),
            }
        }
        Expr::RecordUpdate { base, updates } => {
            let updates = format_record_fields(updates);
            if updates.is_empty() {
                format!("{{ ...{} }}", format_expr(base, 0))
            } else {
                format!("{{ ...{}, {updates} }}", format_expr(base, 0))
            }
        }
        Expr::Match { target, arms } => {
            let arms = arms
                .iter()
                .map(format_match_arm)
                .collect::<Vec<_>>()
                .join(", ");
            format!("match {} {{ {arms} }}", format_expr(target, 0))
        }
        Expr::Raw(raw) => raw.clone(),
    };

    if precedence < context_precedence {
        result = format!("({result})");
    }
    result
}

fn format_inline_block(statements: &[Expr], tail: Option<&Expr>) -> String {
    let mut result = String::from("{");
    for statement in statements {
        result.push(' ');
        match statement {
            Expr::Let { name, value } => {
                write!(result, "let {name} = {};", format_expr(value, 0))
                    .expect("writing to a String cannot fail");
            }
            expression => {
                write!(result, "{};", format_expr(expression, 0))
                    .expect("writing to a String cannot fail");
            }
        }
    }
    if let Some(tail) = tail {
        write!(result, " {}", format_expr(tail, 0)).expect("writing to a String cannot fail");
    }
    result.push_str(" }");
    result
}

fn expr_precedence(expr: &Expr) -> u8 {
    match expr {
        Expr::BinaryOp { op, .. } => match op {
            BinaryOperator::Implies => 1,
            BinaryOperator::Or => 2,
            BinaryOperator::And => 3,
            BinaryOperator::GreaterThan
            | BinaryOperator::GreaterThanOrEqual
            | BinaryOperator::LessThan
            | BinaryOperator::LessThanOrEqual
            | BinaryOperator::Equal
            | BinaryOperator::NotEqual
            | BinaryOperator::Is => 4,
            BinaryOperator::Add | BinaryOperator::Subtract => 5,
            BinaryOperator::Multiply | BinaryOperator::Divide | BinaryOperator::Modulo => 6,
        },
        Expr::UnaryOp { .. } => 7,
        Expr::Call { .. } | Expr::FieldAccess { .. } => 8,
        _ => 9,
    }
}

fn binary_operator(operator: &BinaryOperator) -> &'static str {
    match operator {
        BinaryOperator::Add => "+",
        BinaryOperator::Subtract => "-",
        BinaryOperator::Multiply => "*",
        BinaryOperator::Divide => "/",
        BinaryOperator::Modulo => "%",
        BinaryOperator::GreaterThan => ">",
        BinaryOperator::GreaterThanOrEqual => ">=",
        BinaryOperator::LessThan => "<",
        BinaryOperator::LessThanOrEqual => "<=",
        BinaryOperator::Equal => "==",
        BinaryOperator::NotEqual => "!=",
        BinaryOperator::Is => "is",
        BinaryOperator::And => "&&",
        BinaryOperator::Or => "||",
        BinaryOperator::Implies => "=>",
    }
}

fn unary_operator(operator: &UnaryOperator) -> &'static str {
    match operator {
        UnaryOperator::Negate => "-",
        UnaryOperator::Not => "!",
    }
}

fn format_literal(literal: &Literal) -> String {
    match literal {
        Literal::Int(value) => value.to_string(),
        Literal::Float(value) => {
            let mut value = value.to_string();
            if !value.contains('.') && !value.contains('e') && !value.contains('E') {
                value.push_str(".0");
            }
            value
        }
        Literal::Bool(value) => value.to_string(),
        Literal::String(value) => format!("\"{}\"", escape_string(value)),
        Literal::Unit => "()".into(),
    }
}

fn escape_string(value: &str) -> String {
    let mut escaped = String::new();
    for character in value.chars() {
        match character {
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            other => escaped.push(other),
        }
    }
    escaped
}

fn format_record_fields(fields: &[(String, Expr)]) -> String {
    fields
        .iter()
        .map(|(name, value)| format!("{name}: {}", format_expr(value, 0)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_match_arm(arm: &MatchArm) -> String {
    format!(
        "{} => {}",
        format_pattern(&arm.pattern),
        format_expr(&arm.body, 0)
    )
}

fn format_pattern(pattern: &Pattern) -> String {
    match pattern {
        Pattern::Wildcard => "_".into(),
        Pattern::Literal(literal) => format_literal(literal),
        Pattern::Ident(name) => name.clone(),
        Pattern::Variant {
            enum_name,
            variant_name,
            fields,
        } => {
            let mut result = enum_name
                .as_ref()
                .map_or_else(String::new, |name| format!("{name}."));
            result.push_str(variant_name);
            if !fields.is_empty() {
                result.push('(');
                result.push_str(
                    &fields
                        .iter()
                        .map(format_pattern)
                        .collect::<Vec<_>>()
                        .join(", "),
                );
                result.push(')');
            }
            result
        }
    }
}
