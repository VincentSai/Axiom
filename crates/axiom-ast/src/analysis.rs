//! Symbol call graph, type dependency analysis, and transitive invalidation closure.
//!
//! Tracks caller/callee relationships and type references across an Axiom module,
//! allowing precise determination of downstream impact when symbols are modified concurrently.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::{visit_expr, Expr, Module, Type};

/// 纯函数调用图：仅追踪函数与函数之间的直接与间接调用边
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FunctionCallGraph {
    /// 被调用函数 -> 调用它的函数集合 (callee -> callers)
    callers: HashMap<String, HashSet<String>>,
    /// 调用函数 -> 它直接调用的函数集合 (caller -> callees)
    callees: HashMap<String, HashSet<String>>,
}

impl FunctionCallGraph {
    /// 添加一条纯函数调用边
    pub fn add_call(&mut self, caller: &str, callee: &str) {
        self.callees
            .entry(caller.to_string())
            .or_default()
            .insert(callee.to_string());
        self.callers
            .entry(callee.to_string())
            .or_default()
            .insert(caller.to_string());
    }

    /// 获取直接调用的函数集合
    pub fn direct_callees(&self, func: &str) -> Option<&HashSet<String>> {
        self.callees.get(func)
    }

    /// 获取直接调用者集合
    pub fn direct_callers(&self, func: &str) -> Option<&HashSet<String>> {
        self.callers.get(func)
    }

    /// 计算传递被调用闭包 (向下遍历)
    pub fn transitive_callees(&self, roots: &[&str]) -> HashSet<String> {
        let mut deps = HashSet::new();
        let mut queue = VecDeque::new();
        for &sym in roots {
            let sym_str = sym.to_string();
            if deps.insert(sym_str.clone()) {
                queue.push_back(sym_str);
            }
        }
        while let Some(current) = queue.pop_front() {
            if let Some(upstream) = self.callees.get(&current) {
                for callee in upstream {
                    if deps.insert(callee.clone()) {
                        queue.push_back(callee.clone());
                    }
                }
            }
        }
        deps
    }

    /// 计算传递调用者闭包 (向上遍历)
    pub fn transitive_callers(&self, seeds: &[&str]) -> HashSet<String> {
        let mut affected = HashSet::new();
        let mut queue = VecDeque::new();
        for &sym in seeds {
            let sym_str = sym.to_string();
            if affected.insert(sym_str.clone()) {
                queue.push_back(sym_str);
            }
        }
        while let Some(current) = queue.pop_front() {
            if let Some(downstream) = self.callers.get(&current) {
                for caller in downstream {
                    if affected.insert(caller.clone()) {
                        queue.push_back(caller.clone());
                    }
                }
            }
        }
        affected
    }
}

/// 类型依赖图：追踪函数与类型对自定义类型的引用依赖
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TypeDependencyGraph {
    /// 自定义类型 -> 依赖它的符号（函数或类型）集合 (type -> users)
    type_users: HashMap<String, HashSet<String>>,
    /// 符号 -> 它所直接引用的自定义类型集合 (user -> types)
    type_deps: HashMap<String, HashSet<String>>,
}

impl TypeDependencyGraph {
    /// 添加一条类型引用边
    pub fn add_dependency(&mut self, user: &str, type_name: &str) {
        self.type_deps
            .entry(user.to_string())
            .or_default()
            .insert(type_name.to_string());
        self.type_users
            .entry(type_name.to_string())
            .or_default()
            .insert(user.to_string());
    }

    /// 获取直接引用该自定义类型的用户符号
    pub fn direct_users(&self, type_name: &str) -> Option<&HashSet<String>> {
        self.type_users.get(type_name)
    }

    /// 获取符号直接引用的自定义类型
    pub fn direct_type_dependencies(&self, user: &str) -> Option<&HashSet<String>> {
        self.type_deps.get(user)
    }
}

/// 符号调用与依赖图（集成纯函数调用子图与类型依赖子图）
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CallGraph {
    /// 被依赖符号 -> 依赖它的符号集合 (callee / dep -> callers / users)
    callers: HashMap<String, HashSet<String>>,
    /// 符号 -> 它所直接依赖的符号集合 (caller -> callees / deps)
    callees: HashMap<String, HashSet<String>>,
    /// 纯函数调用子图
    function_calls: FunctionCallGraph,
    /// 类型依赖子图
    type_dependencies: TypeDependencyGraph,
}

impl CallGraph {
    /// 从模块 AST 构建全局符号调用与依赖图
    pub fn from_module(module: &Module) -> Self {
        let mut graph = Self::default();
        let known_functions: HashSet<&str> =
            module.functions.iter().map(|f| f.name.as_str()).collect();

        for func in &module.functions {
            let caller = &func.name;

            // 1. 函数签名对自定义类型的依赖
            for param in &func.params {
                if let Type::Custom(ref type_name) = param.ty {
                    graph.type_dependencies.add_dependency(caller, type_name);
                    graph.add_edge_internal(caller, type_name);
                }
            }
            if let Type::Custom(ref type_name) = func.return_type {
                graph.type_dependencies.add_dependency(caller, type_name);
                graph.add_edge_internal(caller, type_name);
            }

            // 2. 契约与函数体内的调用及符号引用
            let mut extract_deps = |expr: &Expr| {
                visit_expr(expr, &mut |e| match e {
                    Expr::Call { callee, .. } => {
                        if let Expr::Ident(ref callee_name) = **callee {
                            if known_functions.contains(callee_name.as_str()) {
                                graph.function_calls.add_call(caller, callee_name);
                            }
                            graph.add_edge_internal(caller, callee_name);
                        }
                    }
                    Expr::RecordInit {
                        ty_name: Some(ref ty_name),
                        ..
                    } => {
                        graph.type_dependencies.add_dependency(caller, ty_name);
                        graph.add_edge_internal(caller, ty_name);
                    }
                    _ => {}
                });
            };

            extract_deps(&func.body);
            for req in &func.requires {
                extract_deps(req);
            }
            for ens in &func.ensures {
                extract_deps(ens);
            }
        }

        graph
    }

    fn add_edge_internal(&mut self, caller: &str, callee: &str) {
        self.callees
            .entry(caller.to_string())
            .or_default()
            .insert(callee.to_string());
        self.callers
            .entry(callee.to_string())
            .or_default()
            .insert(caller.to_string());
    }

    /// 添加一条依赖边：`caller` 依赖/调用了 `callee`
    pub fn add_edge(&mut self, caller: &str, callee: &str) {
        self.add_edge_internal(caller, callee);
        self.function_calls.add_call(caller, callee);
    }

    /// 获取底层纯函数调用图
    pub fn function_calls(&self) -> &FunctionCallGraph {
        &self.function_calls
    }

    /// 获取底层类型依赖图
    pub fn type_dependencies_graph(&self) -> &TypeDependencyGraph {
        &self.type_dependencies
    }

    /// 获取符号的直接下游调用者/依赖者
    pub fn direct_dependents(&self, symbol: &str) -> Option<&HashSet<String>> {
        self.callers.get(symbol)
    }

    /// 计算给定变更符号集合的全部下游传递影响闭包（Transitive Invalidation Closure）
    ///
    /// 无论依赖链多深，使用 BFS 沿 `callers` 向上遍历，自动处理循环调用并返回完整受影响符号集合。
    /// 结果始终包含传入的 `changed_symbols` 本身。
    pub fn transitive_dependents(&self, changed_symbols: &[&str]) -> HashSet<String> {
        let mut affected = HashSet::new();
        let mut queue = VecDeque::new();

        for &sym in changed_symbols {
            let sym_str = sym.to_string();
            if affected.insert(sym_str.clone()) {
                queue.push_back(sym_str);
            }
        }

        while let Some(current) = queue.pop_front() {
            if let Some(downstream) = self.callers.get(&current) {
                for caller in downstream {
                    if affected.insert(caller.clone()) {
                        queue.push_back(caller.clone());
                    }
                }
            }
        }

        affected
    }

    /// 获取符号的直接被调用依赖者/被引用的符号集合
    pub fn direct_dependencies(&self, symbol: &str) -> Option<&HashSet<String>> {
        self.callees.get(symbol)
    }

    /// 计算给定根符号集合的全部直接或间接被调用依赖闭包（Transitive Dependency Closure）
    ///
    /// 无论调用链多深，使用 BFS 沿 `callees` 向下遍历，自动处理循环调用并返回完整被依赖符号集合。
    /// 结果始终包含传入的 `root_symbols` 本身。
    pub fn transitive_dependencies(&self, root_symbols: &[&str]) -> HashSet<String> {
        let mut deps = HashSet::new();
        let mut queue = VecDeque::new();

        for &sym in root_symbols {
            let sym_str = sym.to_string();
            if deps.insert(sym_str.clone()) {
                queue.push_back(sym_str);
            }
        }

        while let Some(current) = queue.pop_front() {
            if let Some(upstream) = self.callees.get(&current) {
                for callee in upstream {
                    if deps.insert(callee.clone()) {
                        queue.push_back(callee.clone());
                    }
                }
            }
        }

        deps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_callgraph_bidirectional_transitive_closures() {
        let mut graph = CallGraph::default();
        // A -> B -> C
        graph.add_edge("A", "B");
        graph.add_edge("B", "C");

        // A 的被调用依赖闭包应包含 A, B, C
        let deps_a = graph.transitive_dependencies(&["A"]);
        assert!(deps_a.contains("A"));
        assert!(deps_a.contains("B"));
        assert!(deps_a.contains("C"));

        // C 的调用者影响闭包应包含 C, B, A
        let callers_c = graph.transitive_dependents(&["C"]);
        assert!(callers_c.contains("C"));
        assert!(callers_c.contains("B"));
        assert!(callers_c.contains("A"));
    }
}
