//! Symbol call graph, type dependency analysis, and transitive invalidation closure.
//!
//! Tracks caller/callee relationships and type references across an Axiom module,
//! allowing precise determination of downstream impact when symbols are modified concurrently.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::{visit_expr, Expr, Module, Type};

/// 符号调用图与类型依赖图
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CallGraph {
    /// 被依赖符号 -> 依赖它的符号集合 (callee / dep -> callers / users)
    callers: HashMap<String, HashSet<String>>,
    /// 符号 -> 它所直接依赖的符号集合 (caller -> callees / deps)
    callees: HashMap<String, HashSet<String>>,
}

impl CallGraph {
    /// 从模块 AST 构建全局符号调用与依赖图
    pub fn from_module(module: &Module) -> Self {
        let mut graph = Self::default();

        for func in &module.functions {
            let caller = &func.name;

            // 1. 函数签名对自定义类型的依赖
            for param in &func.params {
                if let Type::Custom(ref type_name) = param.ty {
                    graph.add_edge(caller, type_name);
                }
            }
            if let Type::Custom(ref type_name) = func.return_type {
                graph.add_edge(caller, type_name);
            }

            // 2. 契约与函数体内的调用及符号引用
            let mut extract_deps = |expr: &Expr| {
                visit_expr(expr, &mut |e| match e {
                    Expr::Call { callee, .. } => {
                        if let Expr::Ident(ref callee_name) = **callee {
                            graph.add_edge(caller, callee_name);
                        }
                    }
                    Expr::RecordInit {
                        ty_name: Some(ref ty_name),
                        ..
                    } => {
                        graph.add_edge(caller, ty_name);
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

    /// 添加一条依赖边：`caller` 依赖/调用了 `callee`
    pub fn add_edge(&mut self, caller: &str, callee: &str) {
        self.callees
            .entry(caller.to_string())
            .or_default()
            .insert(callee.to_string());
        self.callers
            .entry(callee.to_string())
            .or_default()
            .insert(caller.to_string());
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
