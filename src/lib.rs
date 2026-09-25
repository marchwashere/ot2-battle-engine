//! Headless simulation scaffold. See README.md for the fidelity boundary.
pub mod ai;
pub mod battle;
pub mod content;
pub mod model;
pub mod rng;

pub use battle::{Battle, BattleConfig, BattleState, Command, Event, Outcome, StepReport};
pub use content::{Catalog, Effect, Item, ItemCatalog, Recovery, Resource, Skill, TargetKind};
pub use model::{ActorId, Combatant, Enemy, PlayerCharacter, Resources, Side, Stats};
pub use rng::CrtRng;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimError(pub String);
impl std::fmt::Display for SimError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl std::error::Error for SimError {}
impl From<serde_json::Error> for SimError {
    fn from(value: serde_json::Error) -> Self {
        Self(value.to_string())
    }
}
pub type Result<T> = std::result::Result<T, SimError>;
pub(crate) fn ensure(condition: bool, message: impl Into<String>) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(SimError(message.into()))
    }
}
