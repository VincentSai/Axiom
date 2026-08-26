use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use axiom_ast::FunctionDecl;

/// A host implementation of one Axiom effect.
///
/// Effect-specific operation traits can be implemented alongside this trait;
/// the registry only needs the stable effect name in order to assemble the
/// handler set requested by a function's `with` annotation.
pub trait EffectHandler: Send + Sync {
    fn effect_name(&self) -> &str;
}

pub type SharedEffectHandler = Arc<dyn EffectHandler>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandlerError {
    MissingHandler { effect: String },
}

impl fmt::Display for HandlerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingHandler { effect } => {
                write!(formatter, "no handler registered for effect `{effect}`")
            }
        }
    }
}

impl std::error::Error for HandlerError {}

/// Runtime registry used to resolve `with [...]` requirements to trait objects.
#[derive(Default)]
pub struct HandlerRegistry {
    handlers: HashMap<String, SharedEffectHandler>,
}

impl HandlerRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a handler, replacing and returning any handler with the same name.
    pub fn register<H>(&mut self, handler: H) -> Option<SharedEffectHandler>
    where
        H: EffectHandler + 'static,
    {
        self.register_shared(Arc::new(handler))
    }

    pub fn register_shared(&mut self, handler: SharedEffectHandler) -> Option<SharedEffectHandler> {
        self.handlers
            .insert(handler.effect_name().to_owned(), handler)
    }

    pub fn get(&self, effect: &str) -> Option<SharedEffectHandler> {
        self.handlers.get(effect).cloned()
    }

    /// Resolves names in input order, preserving duplicates when present.
    pub fn resolve<I, S>(&self, effects: I) -> Result<Vec<SharedEffectHandler>, HandlerError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        effects
            .into_iter()
            .map(|effect| {
                let effect = effect.as_ref();
                self.get(effect)
                    .ok_or_else(|| HandlerError::MissingHandler {
                        effect: effect.to_owned(),
                    })
            })
            .collect()
    }

    pub fn resolve_for_function(
        &self,
        function: &FunctionDecl,
    ) -> Result<Vec<SharedEffectHandler>, HandlerError> {
        self.resolve(&function.effects)
    }
}
