//! Headless OT2 combat simulation. See README.md for supported encounters and fidelity.
pub mod ai;
pub mod battle;
pub mod bewildering_grace;
pub mod bg_effects;
pub mod gravell_routine;
pub mod roque_routine;
pub mod fight_search;
pub mod hired_help;
pub mod combat;
pub mod content;
pub mod escape;
pub mod field_escape;
pub mod model;
pub mod mug;
pub mod hikari_recruitment;
pub mod jobs;
pub mod opening_crt;
pub mod pirro;
pub mod progression;
pub mod knowledge;
pub mod rng;
pub mod routine;

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
pub mod temenos_recruitment;

pub mod lord_sands;
