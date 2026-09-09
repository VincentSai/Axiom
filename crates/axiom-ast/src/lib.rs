//! Core abstract syntax tree definitions for Axiom.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Type {
    Int,
    Float,
    Decimal,
    Bool,
    String,
    Unit,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldDef {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VariantDef {
    pub name: String,
    pub fields: Vec<FieldDef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypeDef {
    Record {
        name: String,
        fields: Vec<FieldDef>,
    },
    Enum {
        name: String,
        variants: Vec<VariantDef>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Literal {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Unit,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Pattern {
    Wildcard,
    Literal(Literal),
    Ident(String),
    Variant {
        enum_name: Option<String>,
        variant_name: String,
        fields: Vec<Pattern>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Expr,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    Equal,
    NotEqual,
    Is,
    And,
    Or,
    Implies,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnaryOperator {
    Negate,
    Not,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    Literal(Literal),
    Ident(String),
    Hole(String),
    BinaryOp {
        left: Box<Expr>,
        op: BinaryOperator,
        right: Box<Expr>,
    },
    UnaryOp {
        op: UnaryOperator,
        operand: Box<Expr>,
    },
    Let {
        name: String,
        value: Box<Expr>,
    },
    Block {
        statements: Vec<Expr>,
        tail: Option<Box<Expr>>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    FieldAccess {
        target: Box<Expr>,
        field: String,
    },
    RecordInit {
        ty_name: Option<String>,
        fields: Vec<(String, Expr)>,
    },
    RecordUpdate {
        base: Box<Expr>,
        updates: Vec<(String, Expr)>,
    },
    Match {
        target: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    Raw(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionParam {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectOp {
    pub name: String,
    pub params: Vec<FunctionParam>,
    pub return_type: Type,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectDecl {
    pub name: String,
    pub operations: Vec<EffectOp>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionDecl {
    pub name: String,
    pub params: Vec<FunctionParam>,
    pub return_type: Type,
    pub effects: Vec<String>,
    pub requires: Vec<Expr>,
    pub ensures: Vec<Expr>,
    pub body: Expr,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Module {
    pub types: Vec<TypeDef>,
    pub functions: Vec<FunctionDecl>,
    pub effects: Vec<EffectDecl>,
}

/// Recursively traverses an expression tree, invoking the callback for each sub-expression.
pub fn visit_expr<F: FnMut(&Expr)>(expr: &Expr, f: &mut F) {
    f(expr);
    match expr {
        Expr::Literal(_) | Expr::Ident(_) | Expr::Hole(_) | Expr::Raw(_) => {}
        Expr::BinaryOp { left, right, .. } => {
            visit_expr(left, f);
            visit_expr(right, f);
        }
        Expr::UnaryOp { operand, .. } => {
            visit_expr(operand, f);
        }
        Expr::Let { value, .. } => {
            visit_expr(value, f);
        }
        Expr::Block { statements, tail } => {
            for stmt in statements {
                visit_expr(stmt, f);
            }
            if let Some(tail_expr) = tail {
                visit_expr(tail_expr, f);
            }
        }
        Expr::Call { callee, args } => {
            visit_expr(callee, f);
            for arg in args {
                visit_expr(arg, f);
            }
        }
        Expr::FieldAccess { target, .. } => {
            visit_expr(target, f);
        }
        Expr::RecordInit { fields, .. } => {
            for (_, val) in fields {
                visit_expr(val, f);
            }
        }
        Expr::RecordUpdate { base, updates } => {
            visit_expr(base, f);
            for (_, val) in updates {
                visit_expr(val, f);
            }
        }
        Expr::Match { target, arms } => {
            visit_expr(target, f);
            for arm in arms {
                visit_expr(&arm.body, f);
            }
        }
    }
}

/// Recursively traverses and mutates an expression tree.
pub fn visit_expr_mut<F: FnMut(&mut Expr)>(expr: &mut Expr, f: &mut F) {
    f(expr);
    match expr {
        Expr::Literal(_) | Expr::Ident(_) | Expr::Hole(_) | Expr::Raw(_) => {}
        Expr::BinaryOp { left, right, .. } => {
            visit_expr_mut(left, f);
            visit_expr_mut(right, f);
        }
        Expr::UnaryOp { operand, .. } => {
            visit_expr_mut(operand, f);
        }
        Expr::Let { value, .. } => {
            visit_expr_mut(value, f);
        }
        Expr::Block { statements, tail } => {
            for stmt in statements {
                visit_expr_mut(stmt, f);
            }
            if let Some(tail_expr) = tail {
                visit_expr_mut(tail_expr, f);
            }
        }
        Expr::Call { callee, args } => {
            visit_expr_mut(callee, f);
            for arg in args {
                visit_expr_mut(arg, f);
            }
        }
        Expr::FieldAccess { target, .. } => {
            visit_expr_mut(target, f);
        }
        Expr::RecordInit { fields, .. } => {
            for (_, val) in fields {
                visit_expr_mut(val, f);
            }
        }
        Expr::RecordUpdate { base, updates } => {
            visit_expr_mut(base, f);
            for (_, val) in updates {
                visit_expr_mut(val, f);
            }
        }
        Expr::Match { target, arms } => {
            visit_expr_mut(target, f);
            for arm in arms {
                visit_expr_mut(&mut arm.body, f);
            }
        }
    }
}
