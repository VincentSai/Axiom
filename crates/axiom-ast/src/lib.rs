//! Core abstract syntax tree definitions for Axiom.

pub mod analysis;
pub mod capability;
pub mod merge;
pub mod type_env;

pub use capability::{FunctionCapabilitySummary, ModuleCapabilityMap};
pub use type_env::TypeEnv;

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

pub const DECIMAL_SCALE: i64 = 1_000_000;

pub fn float_to_decimal_micros(f: f64) -> i64 {
    (f * (DECIMAL_SCALE as f64)).round() as i64
}

pub fn decimal_micros_to_float(micros: i64) -> f64 {
    (micros as f64) / (DECIMAL_SCALE as f64)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Literal {
    Int(i64),
    Float(f64),
    Decimal(i64),
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
    Modulo,
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

/// Control flow for expression AST visitors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisitControl {
    /// Continue recursion into child expressions.
    Continue,
    /// Skip visiting the child expressions of the current node.
    SkipChildren,
    /// Stop the traversal entirely.
    Break,
}

/// Recursively traverses and mutates an expression tree with explicit control flow.
pub fn visit_expr_mut_control<F: FnMut(&mut Expr) -> VisitControl>(
    expr: &mut Expr,
    f: &mut F,
) -> VisitControl {
    match f(expr) {
        VisitControl::Break => return VisitControl::Break,
        VisitControl::SkipChildren => return VisitControl::Continue,
        VisitControl::Continue => {}
    }

    match expr {
        Expr::Literal(_) | Expr::Ident(_) | Expr::Hole(_) | Expr::Raw(_) => VisitControl::Continue,
        Expr::BinaryOp { left, right, .. } => {
            if visit_expr_mut_control(left, f) == VisitControl::Break {
                return VisitControl::Break;
            }
            visit_expr_mut_control(right, f)
        }
        Expr::UnaryOp { operand, .. } => visit_expr_mut_control(operand, f),
        Expr::Let { value, .. } => visit_expr_mut_control(value, f),
        Expr::Block { statements, tail } => {
            for stmt in statements {
                if visit_expr_mut_control(stmt, f) == VisitControl::Break {
                    return VisitControl::Break;
                }
            }
            if let Some(tail_expr) = tail {
                visit_expr_mut_control(tail_expr, f)
            } else {
                VisitControl::Continue
            }
        }
        Expr::Call { callee, args } => {
            if visit_expr_mut_control(callee, f) == VisitControl::Break {
                return VisitControl::Break;
            }
            for arg in args {
                if visit_expr_mut_control(arg, f) == VisitControl::Break {
                    return VisitControl::Break;
                }
            }
            VisitControl::Continue
        }
        Expr::FieldAccess { target, .. } => visit_expr_mut_control(target, f),
        Expr::RecordInit { fields, .. } => {
            for (_, val) in fields {
                if visit_expr_mut_control(val, f) == VisitControl::Break {
                    return VisitControl::Break;
                }
            }
            VisitControl::Continue
        }
        Expr::RecordUpdate { base, updates } => {
            if visit_expr_mut_control(base, f) == VisitControl::Break {
                return VisitControl::Break;
            }
            for (_, val) in updates {
                if visit_expr_mut_control(val, f) == VisitControl::Break {
                    return VisitControl::Break;
                }
            }
            VisitControl::Continue
        }
        Expr::Match { target, arms } => {
            if visit_expr_mut_control(target, f) == VisitControl::Break {
                return VisitControl::Break;
            }
            for arm in arms {
                if visit_expr_mut_control(&mut arm.body, f) == VisitControl::Break {
                    return VisitControl::Break;
                }
            }
            VisitControl::Continue
        }
    }
}

/// Recursively traverses and mutates an expression tree.
pub fn visit_expr_mut<F: FnMut(&mut Expr)>(expr: &mut Expr, mut f: F) {
    visit_expr_mut_control(expr, &mut |e| {
        f(e);
        VisitControl::Continue
    });
}

/// Checks whether an expression contains any hole (`?hole_id`).
pub fn expr_contains_hole(expr: &Expr) -> bool {
    let mut contains = false;
    visit_expr(expr, &mut |e| {
        if matches!(e, Expr::Hole(_)) {
            contains = true;
        }
    });
    contains
}

/// Checks whether an expression contains any raw escape hatch (`Expr::Raw`).
pub fn expr_contains_raw(expr: &Expr) -> bool {
    let mut contains = false;
    visit_expr(expr, &mut |e| {
        if matches!(e, Expr::Raw(_)) {
            contains = true;
        }
    });
    contains
}

impl FunctionDecl {
    /// Checks whether the function body or any contract clause contains `Expr::Raw`.
    pub fn contains_raw(&self) -> bool {
        expr_contains_raw(&self.body)
            || self.requires.iter().any(expr_contains_raw)
            || self.ensures.iter().any(expr_contains_raw)
    }

    /// Checks whether the function has formal verification contracts.
    pub fn has_contracts(&self) -> bool {
        !self.requires.is_empty() || !self.ensures.is_empty()
    }
}
