//! Coordination of independently produced implementations for holes in an Axiom module.

use std::collections::HashSet;
use std::fmt;

use axiom_ast::{Expr, Module};
use axiom_hole_extractor::extract_holes;
pub use axiom_hole_extractor::HoleTask;
use axiom_parser::{parse_expr, parse_module, ParseError};
use axiom_smt::verify_function;
pub use axiom_smt::SmtResult;

/// A pool of hole tasks backed by a module whose accepted solutions are SMT-verified.
#[derive(Debug, Clone)]
pub struct AgentTaskPool {
    module: Module,
    tasks: Vec<HoleTask>,
    pending_holes: HashSet<String>,
}

impl AgentTaskPool {
    /// Create a task pool from a parsed skeleton module.
    pub fn new(skeleton_module: Module) -> Self {
        let mut seen_holes = HashSet::new();
        let tasks = extract_holes(&skeleton_module)
            .into_iter()
            .filter(|task| seen_holes.insert(task.hole_id.clone()))
            .collect::<Vec<_>>();
        let pending_holes = tasks.iter().map(|task| task.hole_id.clone()).collect();

        Self {
            module: skeleton_module,
            tasks,
            pending_holes,
        }
    }

    /// Parse an Axiom skeleton module and create its task pool.
    pub fn from_source(source: &str) -> Result<Self, PoolError> {
        Ok(Self::new(parse_module(source)?))
    }

    /// Return the tasks that still need verified implementations, in source order.
    pub fn dispatch_tasks(&self) -> Vec<HoleTask> {
        self.tasks
            .iter()
            .filter(|task| self.pending_holes.contains(&task.hole_id))
            .cloned()
            .collect()
    }

    /// Return the pending hole identifiers in source order.
    pub fn pending_hole_ids(&self) -> Vec<String> {
        self.dispatch_tasks()
            .into_iter()
            .map(|task| task.hole_id)
            .collect()
    }

    /// Parse, verify, and conditionally commit a candidate hole implementation.
    ///
    /// Counter-examples and skipped verification results are returned to the worker without
    /// mutating the pool. Only a candidate for which every affected function is verified is
    /// committed to the module.
    pub fn submit_solution(
        &mut self,
        hole_id: &str,
        solution_source: &str,
    ) -> Result<SmtResult, PoolError> {
        if !self.tasks.iter().any(|task| task.hole_id == hole_id) {
            return Err(PoolError::UnknownHole(hole_id.to_owned()));
        }
        if !self.pending_holes.contains(hole_id) {
            return Err(PoolError::AlreadyResolved(hole_id.to_owned()));
        }

        let solution = parse_expr(solution_source)?;
        let mut candidate_module = self.module.clone();
        let mut affected_functions = Vec::new();

        for (index, function) in candidate_module.functions.iter_mut().enumerate() {
            if replace_hole(&mut function.body, hole_id, &solution) {
                affected_functions.push(index);
            }
        }

        if affected_functions.is_empty() {
            return Err(PoolError::MissingHole(hole_id.to_owned()));
        }

        let mut final_result = SmtResult::Verified;
        for function_index in affected_functions {
            let func = &candidate_module.functions[function_index];
            let result = verify_function(func);
            match &result {
                SmtResult::CounterExample { .. } => return Ok(result),
                SmtResult::Unknown(_) => {
                    // Contracted function failed to verify with SMT -> reject solution
                    return Ok(result);
                }
                SmtResult::Skipped(_) => {
                    if !func.ensures.is_empty() {
                        // Declared formal contracts but verification was skipped -> reject solution
                        return Ok(result);
                    }
                    final_result = result;
                }
                SmtResult::Verified => {}
            }
        }

        self.module = candidate_module;
        self.pending_holes.remove(hole_id);
        Ok(final_result)
    }

    /// Return whether every dispatched hole has an accepted implementation.
    pub fn is_complete(&self) -> bool {
        self.pending_holes.is_empty()
    }

    /// Assemble a clone of the verified, hole-free module.
    pub fn assemble_verified_module(&self) -> Result<Module, PoolError> {
        if self.is_complete() {
            Ok(self.module.clone())
        } else {
            Err(PoolError::Incomplete(self.pending_hole_ids()))
        }
    }
}

/// Errors that prevent a worker submission or final module assembly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PoolError {
    Parse(ParseError),
    UnknownHole(String),
    AlreadyResolved(String),
    MissingHole(String),
    Incomplete(Vec<String>),
}

impl fmt::Display for PoolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => write!(formatter, "could not parse Axiom source: {error}"),
            Self::UnknownHole(hole_id) => write!(formatter, "unknown hole `{hole_id}`"),
            Self::AlreadyResolved(hole_id) => {
                write!(
                    formatter,
                    "hole `{hole_id}` already has a verified solution"
                )
            }
            Self::MissingHole(hole_id) => write!(
                formatter,
                "pending hole `{hole_id}` is missing from the working module"
            ),
            Self::Incomplete(hole_ids) => write!(
                formatter,
                "module still has pending holes: {}",
                hole_ids.join(", ")
            ),
        }
    }
}

impl std::error::Error for PoolError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Parse(error) => Some(error),
            Self::UnknownHole(_)
            | Self::AlreadyResolved(_)
            | Self::MissingHole(_)
            | Self::Incomplete(_) => None,
        }
    }
}

impl From<ParseError> for PoolError {
    fn from(error: ParseError) -> Self {
        Self::Parse(error)
    }
}

fn replace_hole(expression: &mut Expr, hole_id: &str, solution: &Expr) -> bool {
    match expression {
        Expr::Hole(current_hole_id) if current_hole_id == hole_id => {
            *expression = solution.clone();
            true
        }
        Expr::BinaryOp { left, right, .. } => {
            replace_hole(left, hole_id, solution) | replace_hole(right, hole_id, solution)
        }
        Expr::UnaryOp { operand, .. } | Expr::Let { value: operand, .. } => {
            replace_hole(operand, hole_id, solution)
        }
        Expr::Block { statements, tail } => {
            let statements_changed = statements.iter_mut().fold(false, |changed, statement| {
                replace_hole(statement, hole_id, solution) | changed
            });
            let tail_changed = tail
                .as_mut()
                .is_some_and(|tail| replace_hole(tail, hole_id, solution));
            statements_changed | tail_changed
        }
        Expr::Call { callee, args } => {
            let callee_changed = replace_hole(callee, hole_id, solution);
            let args_changed = args.iter_mut().fold(false, |changed, argument| {
                replace_hole(argument, hole_id, solution) | changed
            });
            callee_changed | args_changed
        }
        Expr::FieldAccess { target, .. } => replace_hole(target, hole_id, solution),
        Expr::RecordInit { fields, .. } => fields.iter_mut().fold(false, |changed, (_, value)| {
            replace_hole(value, hole_id, solution) | changed
        }),
        Expr::RecordUpdate { base, updates } => {
            let base_changed = replace_hole(base, hole_id, solution);
            let updates_changed = updates.iter_mut().fold(false, |changed, (_, value)| {
                replace_hole(value, hole_id, solution) | changed
            });
            base_changed | updates_changed
        }
        Expr::Match { target, arms } => {
            let target_changed = replace_hole(target, hole_id, solution);
            let arms_changed = arms.iter_mut().fold(false, |changed, arm| {
                replace_hole(&mut arm.body, hole_id, solution) | changed
            });
            target_changed | arms_changed
        }
        Expr::Literal(_) | Expr::Ident(_) | Expr::Hole(_) | Expr::Raw(_) => false,
    }
}
