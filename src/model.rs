use crate::{Result, ai::BossAi, ensure};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ActorId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Player,
    Enemy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stats {
    pub max_hp: u32,
    pub max_sp: u32,
    pub speed: u32,
}
impl Stats {
    pub(crate) fn validate(&self) -> Result<()> {
        ensure(self.max_hp > 0, "max_hp must be positive")?;
        ensure(
            self.max_hp <= i32::MAX as u32 && self.max_sp <= i32::MAX as u32,
            "resource maxima exceed supported range",
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Resources {
    pub hp: u32,
    pub sp: u32,
    pub bp: u8,
    pub latent: u16,
}
impl Resources {
    pub fn full(stats: &Stats) -> Self {
        Self {
            hp: stats.max_hp,
            sp: stats.max_sp,
            bp: 0,
            latent: 0,
        }
    }
    pub(crate) fn validate(&self, stats: &Stats) -> Result<()> {
        ensure(
            self.hp <= stats.max_hp
                && self.sp <= stats.max_sp
                && self.bp <= 5
                && self.latent <= 100,
            "initial resources exceed caps (BP 5, latent 100)",
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerCharacter {
    pub id: String,
    pub name: String,
    pub stats: Stats,
    #[serde(default)]
    pub resources: Option<Resources>,
    pub skills: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Enemy {
    pub id: String,
    pub name: String,
    pub stats: Stats,
    #[serde(default)]
    pub resources: Option<Resources>,
    pub skills: Vec<String>,
    pub ai: BossAi,
}

/// Mutable actor state. Identity, stats, skills and AI programs live separately.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Combatant {
    pub resources: Resources,
}
impl Combatant {
    pub fn alive(&self) -> bool {
        self.resources.hp != 0
    }
}
