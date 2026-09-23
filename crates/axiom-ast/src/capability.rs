//! Typed Capability IR and Whole-Module Function Capability Analysis.
//!
//! Provides a single, authoritative source of truth for function purity, effect boundaries,
//! floating-point taint closures, decimal usage, arithmetic overflow potentials,
//! and unknown external call boundaries.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

use crate::{
    BinaryOperator, Expr, FunctionDecl, Literal, Module, Pattern, Type, TypeEnv, UnaryOperator,
};

/// 描述单个函数能力特征与边界属性的统一 IR 摘要
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionCapabilitySummary {
    /// 函数名称
    pub name: String,
    /// 是否包含前置/后置形式化契约 (requires / ensures)
    pub has_contracts: bool,
    /// 是否为纯函数 (Pure):
    /// 1. 自身没有声明 effects
    /// 2. 不包含任何未声明的未知外部调用 (transitive_has_unknown_external_callees == false)
    /// 3. 所调用的全部传递下游函数也均为纯函数 (transitive_effects.is_empty())
    pub is_pure: bool,
    /// 本地直接是否包含未受严格实数理论证明支持的浮点运算或类型:
    /// - 参数或返回值类型为 Float
    /// - 契约或函数体中包含 Float 字面量、Float 算术/比较/否定
    pub local_float_tainted: bool,
    /// 传递闭包中是否被浮点污染:
    /// 自身为 local_float_tainted，或其传递调用的任何下游模块内函数为 local_float_tainted
    pub transitive_float_tainted: bool,
    /// 是否包含 Decimal 运算/类型/字段访问
    pub has_decimal: bool,
    /// 是否包含可能发生溢出的整数算术运算 (+, -, *, /, %)
    pub may_overflow: bool,
    /// 直接调用的模块内已知函数集合
    pub direct_callees: HashSet<String>,
    /// 传递调用的全量模块内已知函数闭包 (Transitive Callee Closure)
    pub transitive_callees: HashSet<String>,
    /// 本地是否直接包含未知的外部函数调用（即调用的符号未在当前 Module 中声明）
    pub has_unknown_external_callees: bool,
    /// 传递闭包中是否包含任何未知外部函数调用（自身直接调用或下游传递调用）
    pub transitive_has_unknown_external_callees: bool,
    /// 自身显式声明的 Effect 列表
    pub declared_effects: Vec<String>,
    /// 传递累积的所有下游 Effect 集合
    pub transitive_effects: HashSet<String>,
}

/// 模块级全量函数能力映射表与查询接口
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ModuleCapabilityMap {
    summaries: HashMap<String, FunctionCapabilitySummary>,
}

impl ModuleCapabilityMap {
    /// 从模块 AST 构建不可变的全模块函数能力快照
    pub fn from_module(module: &Module) -> Self {
        let known_functions: HashSet<String> =
            module.functions.iter().map(|f| f.name.clone()).collect();

        // 1. 本地特征扫描
        let mut local_summaries = HashMap::new();
        for func in &module.functions {
            let mut direct_callees = HashSet::new();
            let mut has_unknown_callees = false;

            // 检查直接调用
            let mut extract_calls = |expr: &Expr| {
                crate::visit_expr(expr, &mut |e| {
                    if let Expr::Call { callee, .. } = e {
                        if let Expr::Ident(ref name) = **callee {
                            if known_functions.contains(name) {
                                direct_callees.insert(name.clone());
                            } else {
                                has_unknown_callees = true;
                            }
                        } else {
                            has_unknown_callees = true;
                        }
                    }
                });
            };

            extract_calls(&func.body);
            for req in &func.requires {
                extract_calls(req);
            }
            for ens in &func.ensures {
                extract_calls(ens);
            }

            let mut type_env = TypeEnv::new(module, Some(func));

            let local_float = function_has_local_unsupported_float(func, module, &mut type_env);
            let has_decimal = function_has_decimal_usage(func, module, &mut type_env);
            let may_overflow = function_may_overflow(func, module, &mut type_env);

            let summary = FunctionCapabilitySummary {
                name: func.name.clone(),
                has_contracts: func.has_contracts(),
                is_pure: false, // 待传递闭包计算后决定
                local_float_tainted: local_float,
                transitive_float_tainted: local_float,
                has_decimal,
                may_overflow,
                direct_callees,
                transitive_callees: HashSet::new(),
                has_unknown_external_callees: has_unknown_callees,
                transitive_has_unknown_external_callees: has_unknown_callees,
                declared_effects: func.effects.clone(),
                transitive_effects: HashSet::new(),
            };
            local_summaries.insert(func.name.clone(), summary);
        }

        // 2. 传递闭包传播 (Transitive Propagation)
        let mut final_summaries = local_summaries.clone();

        for summary in final_summaries.values_mut() {
            let mut visited = HashSet::new();
            let mut queue = VecDeque::new();

            for callee in &summary.direct_callees {
                if visited.insert(callee.clone()) {
                    queue.push_back(callee.clone());
                }
            }

            let mut trans_float = summary.local_float_tainted;
            let mut trans_unknown = summary.has_unknown_external_callees;
            let mut trans_effects = HashSet::new();

            while let Some(current) = queue.pop_front() {
                if let Some(callee_summary) = local_summaries.get(&current) {
                    if callee_summary.local_float_tainted {
                        trans_float = true;
                    }
                    if callee_summary.has_unknown_external_callees {
                        trans_unknown = true;
                    }
                    for eff in &callee_summary.declared_effects {
                        trans_effects.insert(eff.clone());
                    }

                    for next_callee in &callee_summary.direct_callees {
                        if visited.insert(next_callee.clone()) {
                            queue.push_back(next_callee.clone());
                        }
                    }
                }
            }

            summary.transitive_callees = visited;
            summary.transitive_float_tainted = trans_float;
            summary.transitive_has_unknown_external_callees = trans_unknown;
            summary.transitive_effects = trans_effects;

            // 纯函数判定规则：
            // 自身无 effect、下游传递无 effect、且无未知外部黑盒调用
            summary.is_pure = summary.declared_effects.is_empty()
                && summary.transitive_effects.is_empty()
                && !summary.transitive_has_unknown_external_callees;
        }

        Self {
            summaries: final_summaries,
        }
    }

    /// 获取指定函数的能力摘要
    pub fn get(&self, name: &str) -> Option<&FunctionCapabilitySummary> {
        self.summaries.get(name)
    }

    /// 判断函数是否处于传递浮点污染状态
    pub fn is_transitive_float_tainted(&self, name: &str) -> bool {
        self.summaries
            .get(name)
            .map(|s| s.transitive_float_tainted)
            .unwrap_or(false)
    }

    /// 判断函数是否为严格纯函数
    pub fn is_pure(&self, name: &str) -> bool {
        self.summaries.get(name).map(|s| s.is_pure).unwrap_or(false)
    }

    /// 判断函数自身或其传递调用中是否包含未知外部调用
    pub fn has_unknown_external_callees(&self, name: &str) -> bool {
        self.summaries
            .get(name)
            .map(|s| s.transitive_has_unknown_external_callees)
            .unwrap_or(true) // 未知函数自身默认视为包含未知外部调用 (Fail-Closed)
    }

    /// 获取模块所有函数的摘要表引用
    pub fn all_summaries(&self) -> &HashMap<String, FunctionCapabilitySummary> {
        &self.summaries
    }
}

// ---------------------------------------------------------------------------
// 局部特征递归检测辅助函数
// ---------------------------------------------------------------------------

fn expr_contains_float_operation(expr: &Expr, type_env: &mut TypeEnv) -> bool {
    match expr {
        Expr::Ident(name) => type_env.lookup(name) == Some(Type::Float),
        Expr::Call { callee, args } => {
            if let Expr::Ident(fn_name) = callee.as_ref() {
                if let Some(f) = type_env
                    .module()
                    .functions
                    .iter()
                    .find(|f| &f.name == fn_name)
                {
                    if f.return_type == Type::Float {
                        return true;
                    }
                }
            }
            args.iter()
                .any(|a| expr_contains_float_operation(a, type_env))
        }
        Expr::BinaryOp { left, right, .. } => {
            let lt = type_env.infer_type(left);
            let rt = type_env.infer_type(right);
            if (lt == Some(Type::Float) || rt == Some(Type::Float))
                && lt != Some(Type::Decimal)
                && rt != Some(Type::Decimal)
            {
                return true;
            }
            expr_contains_float_operation(left, type_env)
                || expr_contains_float_operation(right, type_env)
        }
        Expr::UnaryOp { op, operand } => {
            if *op == UnaryOperator::Negate && type_env.infer_type(operand) == Some(Type::Float) {
                return true;
            }
            expr_contains_float_operation(operand, type_env)
        }
        Expr::Block { statements, tail } => {
            let mut child_env = type_env.clone();
            child_env.push_scope();
            for stmt in statements {
                if let Expr::Let { name, value } = stmt {
                    if expr_contains_float_operation(value, &mut child_env) {
                        return true;
                    }
                    if let Some(ty) = child_env.infer_type(value) {
                        child_env.insert(name.clone(), ty);
                    }
                } else if expr_contains_float_operation(stmt, &mut child_env) {
                    return true;
                }
            }
            if let Some(t) = tail {
                expr_contains_float_operation(t, &mut child_env)
            } else {
                false
            }
        }
        Expr::Match { target, arms } => {
            if expr_contains_float_operation(target, type_env) {
                return true;
            }
            let target_ty = type_env.infer_type(target);
            for arm in arms {
                let mut child_env = type_env.clone();
                child_env.push_scope();
                if let Pattern::Ident(ref id) = arm.pattern {
                    if let Some(ref ty) = target_ty {
                        child_env.insert(id.clone(), ty.clone());
                    }
                }
                if expr_contains_float_operation(&arm.body, &mut child_env) {
                    return true;
                }
            }
            false
        }
        Expr::Let { name, value } => {
            if expr_contains_float_operation(value, type_env) {
                return true;
            }
            if let Some(ty) = type_env.infer_type(value) {
                type_env.insert(name.clone(), ty);
            }
            false
        }
        Expr::RecordInit { fields, .. } => fields
            .iter()
            .any(|(_, v)| expr_contains_float_operation(v, type_env)),
        Expr::RecordUpdate { base, updates } => {
            expr_contains_float_operation(base, type_env)
                || updates
                    .iter()
                    .any(|(_, v)| expr_contains_float_operation(v, type_env))
        }
        Expr::FieldAccess { target, .. } => expr_contains_float_operation(target, type_env),
        _ => false,
    }
}

fn function_has_local_unsupported_float(
    func: &FunctionDecl,
    _module: &Module,
    type_env: &mut TypeEnv,
) -> bool {
    if func.return_type == Type::Float {
        return true;
    }
    if func.params.iter().any(|p| p.ty == Type::Float) {
        return true;
    }
    if expr_contains_float_operation(&func.body, type_env) {
        return true;
    }
    for req in &func.requires {
        if expr_contains_float_operation(req, type_env) {
            return true;
        }
    }
    for ens in &func.ensures {
        if expr_contains_float_operation(ens, type_env) {
            return true;
        }
    }
    false
}

fn expr_contains_decimal(expr: &Expr, type_env: &mut TypeEnv) -> bool {
    if type_env.infer_type(expr) == Some(Type::Decimal) {
        return true;
    }
    match expr {
        Expr::Literal(Literal::Decimal(_)) => true,
        Expr::Ident(name) => type_env.lookup(name) == Some(Type::Decimal),
        Expr::BinaryOp { left, right, .. } => {
            expr_contains_decimal(left, type_env) || expr_contains_decimal(right, type_env)
        }
        Expr::UnaryOp { operand, .. } => expr_contains_decimal(operand, type_env),
        Expr::Call { callee: _, args } => args.iter().any(|a| expr_contains_decimal(a, type_env)),
        Expr::Block { statements, tail } => {
            let mut child_env = type_env.clone();
            child_env.push_scope();
            for stmt in statements {
                if let Expr::Let { name, value } = stmt {
                    if expr_contains_decimal(value, &mut child_env) {
                        return true;
                    }
                    if let Some(ty) = child_env.infer_type(value) {
                        child_env.insert(name.clone(), ty);
                    }
                } else if expr_contains_decimal(stmt, &mut child_env) {
                    return true;
                }
            }
            if let Some(t) = tail {
                expr_contains_decimal(t, &mut child_env)
            } else {
                false
            }
        }
        Expr::Match { target, arms } => {
            if expr_contains_decimal(target, type_env) {
                return true;
            }
            let target_ty = type_env.infer_type(target);
            for arm in arms {
                let mut child_env = type_env.clone();
                child_env.push_scope();
                if let Pattern::Ident(ref id) = arm.pattern {
                    if let Some(ref ty) = target_ty {
                        child_env.insert(id.clone(), ty.clone());
                    }
                }
                if expr_contains_decimal(&arm.body, &mut child_env) {
                    return true;
                }
            }
            false
        }
        Expr::RecordInit { fields, .. } => fields
            .iter()
            .any(|(_, v)| expr_contains_decimal(v, type_env)),
        Expr::RecordUpdate { base, updates } => {
            expr_contains_decimal(base, type_env)
                || updates
                    .iter()
                    .any(|(_, v)| expr_contains_decimal(v, type_env))
        }
        Expr::FieldAccess { target, .. } => expr_contains_decimal(target, type_env),
        _ => false,
    }
}

fn function_has_decimal_usage(
    func: &FunctionDecl,
    _module: &Module,
    type_env: &mut TypeEnv,
) -> bool {
    if func.return_type == Type::Decimal {
        return true;
    }
    if func.params.iter().any(|p| p.ty == Type::Decimal) {
        return true;
    }
    if expr_contains_decimal(&func.body, type_env) {
        return true;
    }
    for req in &func.requires {
        if expr_contains_decimal(req, type_env) {
            return true;
        }
    }
    for ens in &func.ensures {
        if expr_contains_decimal(ens, type_env) {
            return true;
        }
    }
    false
}

fn expr_may_overflow(expr: &Expr, type_env: &mut TypeEnv) -> bool {
    match expr {
        Expr::BinaryOp { left, op, right } => {
            match op {
                BinaryOperator::Add
                | BinaryOperator::Subtract
                | BinaryOperator::Multiply
                | BinaryOperator::Divide
                | BinaryOperator::Modulo => {
                    let lt = type_env.infer_type(left);
                    let rt = type_env.infer_type(right);
                    if lt == Some(Type::Int) || rt == Some(Type::Int) {
                        return true;
                    }
                }
                _ => {}
            }
            expr_may_overflow(left, type_env) || expr_may_overflow(right, type_env)
        }
        Expr::UnaryOp { operand, .. } => expr_may_overflow(operand, type_env),
        Expr::Call { callee: _, args } => args.iter().any(|a| expr_may_overflow(a, type_env)),
        Expr::Block { statements, tail } => {
            let mut child_env = type_env.clone();
            child_env.push_scope();
            for stmt in statements {
                if let Expr::Let { name, value } = stmt {
                    if expr_may_overflow(value, &mut child_env) {
                        return true;
                    }
                    if let Some(ty) = child_env.infer_type(value) {
                        child_env.insert(name.clone(), ty);
                    }
                } else if expr_may_overflow(stmt, &mut child_env) {
                    return true;
                }
            }
            if let Some(t) = tail {
                expr_may_overflow(t, &mut child_env)
            } else {
                false
            }
        }
        Expr::Match { target, arms } => {
            if expr_may_overflow(target, type_env) {
                return true;
            }
            let target_ty = type_env.infer_type(target);
            for arm in arms {
                let mut child_env = type_env.clone();
                child_env.push_scope();
                if let Pattern::Ident(ref id) = arm.pattern {
                    if let Some(ref ty) = target_ty {
                        child_env.insert(id.clone(), ty.clone());
                    }
                }
                if expr_may_overflow(&arm.body, &mut child_env) {
                    return true;
                }
            }
            false
        }
        Expr::RecordInit { fields, .. } => {
            fields.iter().any(|(_, v)| expr_may_overflow(v, type_env))
        }
        Expr::RecordUpdate { base, updates } => {
            expr_may_overflow(base, type_env)
                || updates.iter().any(|(_, v)| expr_may_overflow(v, type_env))
        }
        Expr::FieldAccess { target, .. } => expr_may_overflow(target, type_env),
        _ => false,
    }
}

fn function_may_overflow(func: &FunctionDecl, _module: &Module, type_env: &mut TypeEnv) -> bool {
    if expr_may_overflow(&func.body, type_env) {
        return true;
    }
    for req in &func.requires {
        if expr_may_overflow(req, type_env) {
            return true;
        }
    }
    for ens in &func.ensures {
        if expr_may_overflow(ens, type_env) {
            return true;
        }
    }
    false
}
