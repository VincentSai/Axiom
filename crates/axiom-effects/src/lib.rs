//! Static effect checking and runtime handler resolution for Axiom.

pub mod checker;
pub mod handler;

pub use checker::{check_module_effects, EffectError};
pub use handler::{EffectHandler, HandlerError, HandlerRegistry, SharedEffectHandler};
