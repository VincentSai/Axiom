//! Lexically scoped type environment and expression type inference.
//!
//! Provides a unified, sound type checker for both SMT verification and code generation,
//! preventing name collisions across functions and accurately tracking local variable types.

use std::collections::HashMap;

use crate::{BinaryOperator, Expr, FunctionDecl, Literal, Module, Type, TypeDef, UnaryOperator};

/// 词法作用域类型环境，支持局部变量、参数、遮蔽 (shadowing) 与跨语句类型推导
#[derive(Debug, Clone)]
pub struct TypeEnv<'a> {
    module: &'a Module,
    current_func: Option<&'a FunctionDecl>,
    scopes: Vec<HashMap<String, Type>>,
}

impl<'a> TypeEnv<'a> {
    /// 为指定模块与当前函数构造初始类型环境
    pub fn new(module: &'a Module, current_func: Option<&'a FunctionDecl>) -> Self {
        let mut root_scope = HashMap::new();
        if let Some(func) = current_func {
            for param in &func.params {
                root_scope.insert(param.name.clone(), param.ty.clone());
            }
            root_scope.insert("result".to_string(), func.return_type.clone());
        }

        Self {
            module,
            current_func,
            scopes: vec![root_scope],
        }
    }

    /// 进入新的词法作用域块
    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// 退出当前词法作用域块
    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// 在当前最顶层作用域中注册局部变量类型（支持同名变量遮蔽）
    pub fn insert(&mut self, name: String, ty: Type) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, ty);
        }
    }

    /// 从当前作用域逐级向外查找符号类型
    pub fn lookup(&self, name: &str) -> Option<Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty.clone());
            }
        }
        None
    }

    /// 获取当前函数
    pub fn current_func(&self) -> Option<&'a FunctionDecl> {
        self.current_func
    }

    /// 获取所属模块
    pub fn module(&self) -> &'a Module {
        self.module
    }

    /// 推导任意表达式的静态类型
    pub fn infer_type(&self, expr: &Expr) -> Option<Type> {
        match expr {
            Expr::Literal(lit) => match lit {
                Literal::Int(_) => Some(Type::Int),
                Literal::Float(_) => Some(Type::Float),
                Literal::Decimal(_) => Some(Type::Decimal),
                Literal::Bool(_) => Some(Type::Bool),
                Literal::String(_) => Some(Type::String),
                Literal::Unit => Some(Type::Unit),
            },
            Expr::Ident(name) => self.lookup(name),
            Expr::Hole(_) => None,
            Expr::BinaryOp { left, op, right } => {
                match op {
                    BinaryOperator::Equal
                    | BinaryOperator::NotEqual
                    | BinaryOperator::LessThan
                    | BinaryOperator::LessThanOrEqual
                    | BinaryOperator::GreaterThan
                    | BinaryOperator::GreaterThanOrEqual
                    | BinaryOperator::Is
                    | BinaryOperator::And
                    | BinaryOperator::Or
                    | BinaryOperator::Implies => Some(Type::Bool),
                    BinaryOperator::Add
                    | BinaryOperator::Subtract
                    | BinaryOperator::Multiply
                    | BinaryOperator::Divide
                    | BinaryOperator::Modulo => {
                        let lt = self.infer_type(left);
                        let rt = self.infer_type(right);

                        // Decimal 规则：
                        // 1. Decimal 与 Decimal 运算得到 Decimal
                        // 2. Decimal 与 Int 运算得到 Decimal
                        // 3. Int 与 Decimal 运算得到 Decimal
                        if lt == Some(Type::Decimal) || rt == Some(Type::Decimal) {
                            return Some(Type::Decimal);
                        }
                        if lt == Some(Type::Float) || rt == Some(Type::Float) {
                            return Some(Type::Float);
                        }
                        if lt == Some(Type::Int) || rt == Some(Type::Int) {
                            return Some(Type::Int);
                        }
                        None
                    }
                }
            }
            Expr::UnaryOp { op, operand } => match op {
                UnaryOperator::Not => Some(Type::Bool),
                UnaryOperator::Negate => self.infer_type(operand),
            },
            Expr::Let { .. } => Some(Type::Unit),
            Expr::Block { statements, tail } => {
                let mut child_env = self.clone();
                child_env.push_scope();
                for stmt in statements {
                    if let Expr::Let { name, value } = stmt {
                        if let Some(v_ty) = child_env.infer_type(value) {
                            child_env.insert(name.clone(), v_ty);
                        }
                    }
                }
                if let Some(tail_expr) = tail {
                    child_env.infer_type(tail_expr)
                } else {
                    Some(Type::Unit)
                }
            }
            Expr::Call { callee, .. } => {
                if let Expr::Ident(func_name) = callee.as_ref() {
                    for f in &self.module.functions {
                        if &f.name == func_name {
                            return Some(f.return_type.clone());
                        }
                    }
                }
                None
            }
            Expr::FieldAccess { target, field } => {
                let target_ty = self.infer_type(target);
                if let Some(Type::Custom(ref record_name)) = target_ty {
                    for ty_def in &self.module.types {
                        if let TypeDef::Record { name, fields } = ty_def {
                            if name == record_name {
                                for f in fields {
                                    if &f.name == field {
                                        return Some(f.ty.clone());
                                    }
                                }
                            }
                        }
                    }
                }

                // Fallback：若无法推断 target 确切类型，但在唯一的 Record 中匹配到该字段
                let mut matched = Vec::new();
                for ty_def in &self.module.types {
                    if let TypeDef::Record { fields, .. } = ty_def {
                        for f in fields {
                            if &f.name == field {
                                matched.push(f.ty.clone());
                            }
                        }
                    }
                }
                if matched.len() == 1 {
                    matched.pop()
                } else {
                    None
                }
            }
            Expr::RecordInit { ty_name, .. } => {
                ty_name.as_ref().map(|name| Type::Custom(name.clone()))
            }
            Expr::RecordUpdate { base, .. } => self.infer_type(base),
            Expr::Match { target, arms } => {
                let target_ty = self.infer_type(target);
                for arm in arms {
                    let mut child_env = self.clone();
                    child_env.push_scope();
                    if let crate::Pattern::Ident(id) = &arm.pattern {
                        child_env.insert(id.clone(), target_ty.clone().unwrap_or(Type::Int));
                    }
                    if let Some(body_ty) = child_env.infer_type(&arm.body) {
                        return Some(body_ty);
                    }
                }
                None
            }
            Expr::Raw(_) => None,
        }
    }

    /// 在当前环境中推断表达式在指定上下文类型中的有效操作数类型（用于 FloatLiteral 到 Decimal 的上下文精化）
    pub fn infer_effective_operand_type(
        &self,
        expr: &Expr,
        context_ty: Option<&Type>,
    ) -> Option<Type> {
        let base_ty = self.infer_type(expr);
        if let Some(Type::Decimal) = context_ty {
            if matches!(expr, Expr::Literal(Literal::Float(_))) {
                return Some(Type::Decimal);
            }
        }
        base_ty
    }

    /// 判断表达式是否为 Decimal 类型
    pub fn is_decimal(&self, expr: &Expr) -> bool {
        self.infer_type(expr) == Some(Type::Decimal)
    }

    /// 判断表达式是否为 Int 类型
    pub fn is_int(&self, expr: &Expr) -> bool {
        self.infer_type(expr) == Some(Type::Int)
    }

    /// 判断表达式是否为 Float 类型
    pub fn is_float(&self, expr: &Expr) -> bool {
        self.infer_type(expr) == Some(Type::Float)
    }
}
