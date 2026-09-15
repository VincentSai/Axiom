//! AST-level structural 3-way merge algorithm and conflict diagnostic models.
//!
//! Provides deterministic, syntax-aware merging of Axiom modules across concurrent branches,
//! avoiding text-level diff3 pitfalls (such as broken brackets or misplaced indentation).

use std::collections::{BTreeMap, HashSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{EffectDecl, Expr, FunctionDecl, MatchArm, Module, TypeDef};

/// 结构化冲突诊断报告，当 AST 3-way 合并遇到不可调和的语义冲突时产生
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AstConflictReport {
    pub conflicts: Vec<AstConflict>,
}

impl fmt::Display for AstConflictReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "AST 3-way merge failed with {} conflicts:",
            self.conflicts.len()
        )?;
        for (i, c) in self.conflicts.iter().enumerate() {
            writeln!(f, "  [{}] {}", i + 1, c)?;
        }
        Ok(())
    }
}

impl std::error::Error for AstConflictReport {}

/// 单个 AST 节点的冲突分类与诊断
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AstConflict {
    /// 表达式或子节点冲突（双方修改了同一个节点且值不兼容）
    NodeConflict {
        path: String,
        base: String,
        ours: String,
        theirs: String,
        reason: String,
    },
    /// 符号签名冲突（双方修改了同一个符号的签名且互不相同）
    SignatureConflict { symbol: String, reason: String },
    /// 一方删除、另一方修改引发的冲突
    DeleteModifyConflict {
        symbol: String,
        deleted_by: String,
        modified_by: String,
    },
}

impl fmt::Display for AstConflict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NodeConflict {
                path,
                base,
                ours,
                theirs,
                reason,
            } => {
                write!(
                    f,
                    "Node conflict at `{path}`: {reason} (base: `{base}`, ours: `{ours}`, theirs: `{theirs}`)"
                )
            }
            Self::SignatureConflict { symbol, reason } => {
                write!(f, "Signature conflict for symbol `{symbol}`: {reason}")
            }
            Self::DeleteModifyConflict {
                symbol,
                deleted_by,
                modified_by,
            } => {
                write!(
                    f,
                    "Delete/Modify conflict for symbol `{symbol}`: deleted by {deleted_by}, but modified by {modified_by}"
                )
            }
        }
    }
}

/// 执行模块级 AST 三向合并 (3-Way Merge)
///
/// 输入基线模块 `base`，当前分支修改 `ours`，远端/主干修改 `theirs`。
/// 若不存在不可调和冲突，返回干净合并后的新 `Module`；
/// 否则返回包含全部具体冲突路径的 `AstConflictReport`。
pub fn merge_modules_3way(
    base: &Module,
    ours: &Module,
    theirs: &Module,
) -> Result<Module, AstConflictReport> {
    let mut conflicts = Vec::new();

    let merged_types = merge_types_3way(&base.types, &ours.types, &theirs.types, &mut conflicts);
    let merged_effects = merge_effects_3way(
        &base.effects,
        &ours.effects,
        &theirs.effects,
        &mut conflicts,
    );
    let merged_functions = merge_functions_3way(
        &base.functions,
        &ours.functions,
        &theirs.functions,
        &mut conflicts,
    );

    if !conflicts.is_empty() {
        return Err(AstConflictReport { conflicts });
    }

    Ok(Module {
        types: merged_types,
        effects: merged_effects,
        functions: merged_functions,
    })
}

fn type_def_name(ty: &TypeDef) -> &str {
    match ty {
        TypeDef::Record { name, .. } | TypeDef::Enum { name, .. } => name,
    }
}

fn merge_types_3way(
    base: &[TypeDef],
    ours: &[TypeDef],
    theirs: &[TypeDef],
    conflicts: &mut Vec<AstConflict>,
) -> Vec<TypeDef> {
    let base_map: BTreeMap<&str, &TypeDef> = base.iter().map(|t| (type_def_name(t), t)).collect();
    let ours_map: BTreeMap<&str, &TypeDef> = ours.iter().map(|t| (type_def_name(t), t)).collect();
    let theirs_map: BTreeMap<&str, &TypeDef> =
        theirs.iter().map(|t| (type_def_name(t), t)).collect();

    let mut all_names: Vec<&str> = Vec::new();
    let mut seen = HashSet::new();
    for name in ours
        .iter()
        .map(type_def_name)
        .chain(theirs.iter().map(type_def_name))
        .chain(base.iter().map(type_def_name))
    {
        if seen.insert(name) {
            all_names.push(name);
        }
    }

    let mut merged = Vec::new();

    for name in all_names {
        let b = base_map.get(name).copied();
        let o = ours_map.get(name).copied();
        let t = theirs_map.get(name).copied();

        match (b, o, t) {
            (Some(b_val), Some(o_val), Some(t_val)) => {
                if o_val == b_val && t_val == b_val {
                    merged.push((*b_val).clone());
                } else if o_val != b_val && t_val == b_val {
                    merged.push((*o_val).clone());
                } else if o_val == b_val && t_val != b_val {
                    merged.push((*t_val).clone());
                } else if o_val == t_val {
                    merged.push((*o_val).clone());
                } else {
                    conflicts.push(AstConflict::NodeConflict {
                        path: format!("types[{name}]"),
                        base: format!("{b_val:?}"),
                        ours: format!("{o_val:?}"),
                        theirs: format!("{t_val:?}"),
                        reason: "conflicting type definitions".to_string(),
                    });
                }
            }
            (None, Some(o_val), None) => merged.push((*o_val).clone()),
            (None, None, Some(t_val)) => merged.push((*t_val).clone()),
            (None, Some(o_val), Some(t_val)) => {
                if o_val == t_val {
                    merged.push((*o_val).clone());
                } else {
                    conflicts.push(AstConflict::NodeConflict {
                        path: format!("types[{name}]"),
                        base: "none".to_string(),
                        ours: format!("{o_val:?}"),
                        theirs: format!("{t_val:?}"),
                        reason: "both added conflicting type definitions".to_string(),
                    });
                }
            }
            (Some(b_val), None, Some(t_val)) => {
                if t_val == b_val {
                    // Ours deleted, theirs unchanged => accept deletion
                } else {
                    conflicts.push(AstConflict::DeleteModifyConflict {
                        symbol: name.to_string(),
                        deleted_by: "ours".to_string(),
                        modified_by: "theirs".to_string(),
                    });
                }
            }
            (Some(b_val), Some(o_val), None) => {
                if o_val == b_val {
                    // Theirs deleted, ours unchanged => accept deletion
                } else {
                    conflicts.push(AstConflict::DeleteModifyConflict {
                        symbol: name.to_string(),
                        deleted_by: "theirs".to_string(),
                        modified_by: "ours".to_string(),
                    });
                }
            }
            (Some(_), None, None) => {
                // Both deleted
            }
            (None, None, None) => unreachable!(),
        }
    }

    merged
}

fn merge_effects_3way(
    base: &[EffectDecl],
    ours: &[EffectDecl],
    theirs: &[EffectDecl],
    conflicts: &mut Vec<AstConflict>,
) -> Vec<EffectDecl> {
    let base_map: BTreeMap<&str, &EffectDecl> = base.iter().map(|e| (e.name.as_str(), e)).collect();
    let ours_map: BTreeMap<&str, &EffectDecl> = ours.iter().map(|e| (e.name.as_str(), e)).collect();
    let theirs_map: BTreeMap<&str, &EffectDecl> =
        theirs.iter().map(|e| (e.name.as_str(), e)).collect();

    let mut all_names: Vec<&str> = Vec::new();
    let mut seen = HashSet::new();
    for name in ours
        .iter()
        .map(|e| e.name.as_str())
        .chain(theirs.iter().map(|e| e.name.as_str()))
        .chain(base.iter().map(|e| e.name.as_str()))
    {
        if seen.insert(name) {
            all_names.push(name);
        }
    }

    let mut merged = Vec::new();

    for name in all_names {
        let b = base_map.get(name).copied();
        let o = ours_map.get(name).copied();
        let t = theirs_map.get(name).copied();

        match (b, o, t) {
            (Some(b_val), Some(o_val), Some(t_val)) => {
                if o_val == b_val && t_val == b_val {
                    merged.push((*b_val).clone());
                } else if o_val != b_val && t_val == b_val {
                    merged.push((*o_val).clone());
                } else if o_val == b_val && t_val != b_val {
                    merged.push((*t_val).clone());
                } else if o_val == t_val {
                    merged.push((*o_val).clone());
                } else {
                    conflicts.push(AstConflict::NodeConflict {
                        path: format!("effects[{name}]"),
                        base: format!("{b_val:?}"),
                        ours: format!("{o_val:?}"),
                        theirs: format!("{t_val:?}"),
                        reason: "conflicting effect declarations".to_string(),
                    });
                }
            }
            (None, Some(o_val), None) => merged.push((*o_val).clone()),
            (None, None, Some(t_val)) => merged.push((*t_val).clone()),
            (None, Some(o_val), Some(t_val)) => {
                if o_val == t_val {
                    merged.push((*o_val).clone());
                } else {
                    conflicts.push(AstConflict::NodeConflict {
                        path: format!("effects[{name}]"),
                        base: "none".to_string(),
                        ours: format!("{o_val:?}"),
                        theirs: format!("{t_val:?}"),
                        reason: "both added conflicting effect declarations".to_string(),
                    });
                }
            }
            (Some(b_val), None, Some(t_val)) => {
                if t_val == b_val {
                    // Accepted deletion
                } else {
                    conflicts.push(AstConflict::DeleteModifyConflict {
                        symbol: name.to_string(),
                        deleted_by: "ours".to_string(),
                        modified_by: "theirs".to_string(),
                    });
                }
            }
            (Some(b_val), Some(o_val), None) => {
                if o_val == b_val {
                    // Accepted deletion
                } else {
                    conflicts.push(AstConflict::DeleteModifyConflict {
                        symbol: name.to_string(),
                        deleted_by: "theirs".to_string(),
                        modified_by: "ours".to_string(),
                    });
                }
            }
            (Some(_), None, None) => {}
            (None, None, None) => unreachable!(),
        }
    }

    merged
}

fn merge_functions_3way(
    base: &[FunctionDecl],
    ours: &[FunctionDecl],
    theirs: &[FunctionDecl],
    conflicts: &mut Vec<AstConflict>,
) -> Vec<FunctionDecl> {
    let base_map: BTreeMap<&str, &FunctionDecl> =
        base.iter().map(|f| (f.name.as_str(), f)).collect();
    let ours_map: BTreeMap<&str, &FunctionDecl> =
        ours.iter().map(|f| (f.name.as_str(), f)).collect();
    let theirs_map: BTreeMap<&str, &FunctionDecl> =
        theirs.iter().map(|f| (f.name.as_str(), f)).collect();

    let mut all_names: Vec<&str> = Vec::new();
    let mut seen = HashSet::new();
    for name in ours
        .iter()
        .map(|f| f.name.as_str())
        .chain(theirs.iter().map(|f| f.name.as_str()))
        .chain(base.iter().map(|f| f.name.as_str()))
    {
        if seen.insert(name) {
            all_names.push(name);
        }
    }

    let mut merged = Vec::new();

    for name in all_names {
        let b = base_map.get(name).copied();
        let o = ours_map.get(name).copied();
        let t = theirs_map.get(name).copied();

        match (b, o, t) {
            (Some(b_val), Some(o_val), Some(t_val)) => {
                if let Some(f) = merge_single_function_3way(b_val, o_val, t_val, conflicts) {
                    merged.push(f);
                }
            }
            (None, Some(o_val), None) => merged.push((*o_val).clone()),
            (None, None, Some(t_val)) => merged.push((*t_val).clone()),
            (None, Some(o_val), Some(t_val)) => {
                if o_val == t_val {
                    merged.push((*o_val).clone());
                } else {
                    conflicts.push(AstConflict::NodeConflict {
                        path: format!("functions[{name}]"),
                        base: "none".to_string(),
                        ours: format!("{o_val:?}"),
                        theirs: format!("{t_val:?}"),
                        reason: "both added conflicting function implementations".to_string(),
                    });
                }
            }
            (Some(b_val), None, Some(t_val)) => {
                if t_val == b_val {
                    // Accepted deletion
                } else {
                    conflicts.push(AstConflict::DeleteModifyConflict {
                        symbol: name.to_string(),
                        deleted_by: "ours".to_string(),
                        modified_by: "theirs".to_string(),
                    });
                }
            }
            (Some(b_val), Some(o_val), None) => {
                if o_val == b_val {
                    // Accepted deletion
                } else {
                    conflicts.push(AstConflict::DeleteModifyConflict {
                        symbol: name.to_string(),
                        deleted_by: "theirs".to_string(),
                        modified_by: "ours".to_string(),
                    });
                }
            }
            (Some(_), None, None) => {}
            (None, None, None) => unreachable!(),
        }
    }

    merged
}

fn merge_single_function_3way(
    b: &FunctionDecl,
    o: &FunctionDecl,
    t: &FunctionDecl,
    conflicts: &mut Vec<AstConflict>,
) -> Option<FunctionDecl> {
    if o == b && t == b {
        return Some(b.clone());
    }
    if o != b && t == b {
        return Some(o.clone());
    }
    if o == b && t != b {
        return Some(t.clone());
    }
    if o == t {
        return Some(o.clone());
    }

    // 1. 签名三向合并 (Signature)
    let sig_b = (&b.params, &b.return_type, &b.effects);
    let sig_o = (&o.params, &o.return_type, &o.effects);
    let sig_t = (&t.params, &t.return_type, &t.effects);

    let (params, return_type, effects) = if sig_o == sig_b && sig_t == sig_b {
        (b.params.clone(), b.return_type.clone(), b.effects.clone())
    } else if sig_o != sig_b && sig_t == sig_b {
        (o.params.clone(), o.return_type.clone(), o.effects.clone())
    } else if sig_o == sig_b && sig_t != sig_b {
        (t.params.clone(), t.return_type.clone(), t.effects.clone())
    } else if sig_o == sig_t {
        (o.params.clone(), o.return_type.clone(), o.effects.clone())
    } else {
        conflicts.push(AstConflict::SignatureConflict {
            symbol: b.name.clone(),
            reason: "incompatible signature modifications between ours and theirs".to_string(),
        });
        (o.params.clone(), o.return_type.clone(), o.effects.clone())
    };

    // 2. 契约三向合并 (Contracts: requires & ensures)
    let merged_requires = merge_expr_list_3way(
        &format!("functions[{}].requires", b.name),
        &b.requires,
        &o.requires,
        &t.requires,
        conflicts,
    );

    let merged_ensures = merge_expr_list_3way(
        &format!("functions[{}].ensures", b.name),
        &b.ensures,
        &o.ensures,
        &t.ensures,
        conflicts,
    );

    // 3. 函数体三向合并 (Body Expression)
    let merged_body = merge_expr_3way(
        &format!("functions[{}].body", b.name),
        &b.body,
        &o.body,
        &t.body,
        conflicts,
    );

    Some(FunctionDecl {
        name: b.name.clone(),
        params,
        return_type,
        effects,
        requires: merged_requires,
        ensures: merged_ensures,
        body: merged_body,
    })
}

fn merge_expr_list_3way(
    path: &str,
    b: &[Expr],
    o: &[Expr],
    t: &[Expr],
    conflicts: &mut Vec<AstConflict>,
) -> Vec<Expr> {
    if o == b && t == b {
        return b.to_vec();
    }
    if o != b && t == b {
        return o.to_vec();
    }
    if o == b && t != b {
        return t.to_vec();
    }
    if o == t {
        return o.to_vec();
    }

    let o_deleted = b.iter().any(|item| !o.contains(item));
    let t_deleted = b.iter().any(|item| !t.contains(item));

    if !o_deleted && !t_deleted {
        // 两方都没有删除 base 项，各自只追加了谓词，做无冲突并集融合
        let mut result = b.to_vec();
        for item in o {
            if !b.contains(item) && !result.contains(item) {
                result.push(item.clone());
            }
        }
        for item in t {
            if !b.contains(item) && !result.contains(item) {
                result.push(item.clone());
            }
        }
        return result;
    }

    // 若有删除且双方变更不同，记录冲突
    conflicts.push(AstConflict::NodeConflict {
        path: path.to_string(),
        base: format!("{b:?}"),
        ours: format!("{o:?}"),
        theirs: format!("{t:?}"),
        reason: "conflicting contract predicate modifications".to_string(),
    });
    o.to_vec()
}

/// 递归合并表达式
pub fn merge_expr_3way(
    path: &str,
    b: &Expr,
    o: &Expr,
    t: &Expr,
    conflicts: &mut Vec<AstConflict>,
) -> Expr {
    if o == b && t == b {
        return b.clone();
    }
    if o != b && t == b {
        return o.clone();
    }
    if o == b && t != b {
        return t.clone();
    }
    if o == t {
        return o.clone();
    }

    // 关键融合 1: 打洞与填充 (Hole Resolution Fusion)
    if let Expr::Hole(h_b) = b {
        let o_is_same_hole = matches!(o, Expr::Hole(h_o) if h_o == h_b);
        let t_is_same_hole = matches!(t, Expr::Hole(h_t) if h_t == h_b);

        if o_is_same_hole && !t_is_same_hole {
            // Ours 仍是未填的洞，Theirs 填充了实现 => 采纳 Theirs
            return t.clone();
        }
        if !o_is_same_hole && t_is_same_hole {
            // Ours 填充了实现，Theirs 仍是洞 => 采纳 Ours
            return o.clone();
        }
    }

    // 关键融合 2: 同构复合表达式的结构性递归拆解合并
    match (b, o, t) {
        (
            Expr::BinaryOp {
                op: op_b,
                left: l_b,
                right: r_b,
            },
            Expr::BinaryOp {
                op: op_o,
                left: l_o,
                right: r_o,
            },
            Expr::BinaryOp {
                op: op_t,
                left: l_t,
                right: r_t,
            },
        ) if op_b == op_o && op_o == op_t => {
            let left_merged = merge_expr_3way(&format!("{path}.left"), l_b, l_o, l_t, conflicts);
            let right_merged = merge_expr_3way(&format!("{path}.right"), r_b, r_o, r_t, conflicts);
            return Expr::BinaryOp {
                op: op_b.clone(),
                left: Box::new(left_merged),
                right: Box::new(right_merged),
            };
        }
        (
            Expr::UnaryOp {
                op: op_b,
                operand: opnd_b,
            },
            Expr::UnaryOp {
                op: op_o,
                operand: opnd_o,
            },
            Expr::UnaryOp {
                op: op_t,
                operand: opnd_t,
            },
        ) if op_b == op_o && op_o == op_t => {
            let operand_merged = merge_expr_3way(
                &format!("{path}.operand"),
                opnd_b,
                opnd_o,
                opnd_t,
                conflicts,
            );
            return Expr::UnaryOp {
                op: op_b.clone(),
                operand: Box::new(operand_merged),
            };
        }
        (
            Expr::Let {
                name: n_b,
                value: v_b,
            },
            Expr::Let {
                name: n_o,
                value: v_o,
            },
            Expr::Let {
                name: n_t,
                value: v_t,
            },
        ) if n_b == n_o && n_o == n_t => {
            let val_merged = merge_expr_3way(
                &format!("{path}.let[{n_b}].value"),
                v_b,
                v_o,
                v_t,
                conflicts,
            );
            return Expr::Let {
                name: n_b.clone(),
                value: Box::new(val_merged),
            };
        }
        (
            Expr::Block {
                statements: s_b,
                tail: tail_b,
            },
            Expr::Block {
                statements: s_o,
                tail: tail_o,
            },
            Expr::Block {
                statements: s_t,
                tail: tail_t,
            },
        ) => {
            let tail_merged = match (tail_b, tail_o, tail_t) {
                (Some(tb), Some(to), Some(tt)) => Some(Box::new(merge_expr_3way(
                    &format!("{path}.tail"),
                    tb,
                    to,
                    tt,
                    conflicts,
                ))),
                (None, None, None) => None,
                (None, Some(to), None) => Some(to.clone()),
                (None, None, Some(tt)) => Some(tt.clone()),
                _ => {
                    conflicts.push(AstConflict::NodeConflict {
                        path: format!("{path}.tail"),
                        base: format!("{tail_b:?}"),
                        ours: format!("{tail_o:?}"),
                        theirs: format!("{tail_t:?}"),
                        reason: "conflicting block tail expression modifications".to_string(),
                    });
                    tail_o.clone()
                }
            };

            let statements_merged = merge_statements_3way(path, s_b, s_o, s_t, conflicts);

            return Expr::Block {
                statements: statements_merged,
                tail: tail_merged,
            };
        }
        (
            Expr::Call {
                callee: c_b,
                args: a_b,
            },
            Expr::Call {
                callee: c_o,
                args: a_o,
            },
            Expr::Call {
                callee: c_t,
                args: a_t,
            },
        ) => {
            let callee_merged =
                merge_expr_3way(&format!("{path}.callee"), c_b, c_o, c_t, conflicts);
            let args_merged = if a_b.len() == a_o.len() && a_o.len() == a_t.len() {
                let mut merged = Vec::with_capacity(a_b.len());
                for (idx, ((arg_b, arg_o), arg_t)) in
                    a_b.iter().zip(a_o.iter()).zip(a_t.iter()).enumerate()
                {
                    merged.push(merge_expr_3way(
                        &format!("{path}.args[{idx}]"),
                        arg_b,
                        arg_o,
                        arg_t,
                        conflicts,
                    ));
                }
                merged
            } else {
                conflicts.push(AstConflict::NodeConflict {
                    path: format!("{path}.args"),
                    base: format!("{a_b:?}"),
                    ours: format!("{a_o:?}"),
                    theirs: format!("{a_t:?}"),
                    reason: "call argument list length mismatch".to_string(),
                });
                a_o.clone()
            };

            return Expr::Call {
                callee: Box::new(callee_merged),
                args: args_merged,
            };
        }
        (
            Expr::Match {
                target: tg_b,
                arms: ar_b,
            },
            Expr::Match {
                target: tg_o,
                arms: ar_o,
            },
            Expr::Match {
                target: tg_t,
                arms: ar_t,
            },
        ) => {
            let target_merged =
                merge_expr_3way(&format!("{path}.target"), tg_b, tg_o, tg_t, conflicts);
            let arms_merged = if ar_b.len() == ar_o.len() && ar_o.len() == ar_t.len() {
                let mut merged = Vec::with_capacity(ar_b.len());
                for (idx, ((arm_b, arm_o), arm_t)) in
                    ar_b.iter().zip(ar_o.iter()).zip(ar_t.iter()).enumerate()
                {
                    if arm_b.pattern != arm_o.pattern || arm_o.pattern != arm_t.pattern {
                        conflicts.push(AstConflict::NodeConflict {
                            path: format!("{path}.arms[{idx}].pattern"),
                            base: format!("{:?}", arm_b.pattern),
                            ours: format!("{:?}", arm_o.pattern),
                            theirs: format!("{:?}", arm_t.pattern),
                            reason: "conflicting match patterns in arm".to_string(),
                        });
                        merged.push(arm_o.clone());
                    } else {
                        let body_merged = merge_expr_3way(
                            &format!("{path}.arms[{idx}].body"),
                            &arm_b.body,
                            &arm_o.body,
                            &arm_t.body,
                            conflicts,
                        );
                        merged.push(MatchArm {
                            pattern: arm_b.pattern.clone(),
                            body: body_merged,
                        });
                    }
                }
                merged
            } else {
                conflicts.push(AstConflict::NodeConflict {
                    path: format!("{path}.arms"),
                    base: format!("{ar_b:?}"),
                    ours: format!("{ar_o:?}"),
                    theirs: format!("{ar_t:?}"),
                    reason: "conflicting match arms count mismatch".to_string(),
                });
                ar_o.clone()
            };

            return Expr::Match {
                target: Box::new(target_merged),
                arms: arms_merged,
            };
        }
        _ => {}
    }

    // 无法同构拆解，且双方修改互不全等 => 记录结构冲突
    conflicts.push(AstConflict::NodeConflict {
        path: path.to_string(),
        base: format!("{b:?}"),
        ours: format!("{o:?}"),
        theirs: format!("{t:?}"),
        reason: "concurrent conflicting modifications on same AST expression node".to_string(),
    });
    o.clone()
}

fn merge_statements_3way(
    path: &str,
    b: &[Expr],
    o: &[Expr],
    t: &[Expr],
    conflicts: &mut Vec<AstConflict>,
) -> Vec<Expr> {
    if o == b && t == b {
        return b.to_vec();
    }
    if o != b && t == b {
        return o.to_vec();
    }
    if o == b && t != b {
        return t.to_vec();
    }
    if o == t {
        return o.to_vec();
    }

    // 若长度全等，逐语句递归三向合并
    if b.len() == o.len() && o.len() == t.len() {
        let mut merged = Vec::with_capacity(b.len());
        for (idx, ((stmt_b, stmt_o), stmt_t)) in b.iter().zip(o.iter()).zip(t.iter()).enumerate() {
            let stmt_merged = merge_expr_3way(
                &format!("{path}.stmt[{idx}]"),
                stmt_b,
                stmt_o,
                stmt_t,
                conflicts,
            );
            merged.push(stmt_merged);
        }
        return merged;
    }

    conflicts.push(AstConflict::NodeConflict {
        path: format!("{path}.statements"),
        base: format!("{b:?}"),
        ours: format!("{o:?}"),
        theirs: format!("{t:?}"),
        reason: "conflicting block statements with different lengths".to_string(),
    });
    o.to_vec()
}
