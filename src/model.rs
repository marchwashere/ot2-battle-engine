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
        self.validate_with_latent_max(stats, 100)
    }
    pub(crate) fn validate_with_latent_max(&self, stats: &Stats, latent_max: u16) -> Result<()> {
        ensure(
            self.hp <= stats.max_hp
                && self.sp <= stats.max_sp
                && self.bp <= 5
                && self.latent <= latent_max,
            "initial resources exceed character caps",
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerCharacter {
    #[serde(default,skip_serializing_if="is_false")]
    pub boost_start: bool,
    pub id: String,
    pub name: String,
    pub stats: Stats,
    #[serde(default)]
    pub resources: Option<Resources>,
    pub skills: Vec<String>,
    #[serde(default)]
    pub combat: Option<crate::combat::CombatProfile>,
}
fn is_false(value:&bool)->bool { !*value }

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
    #[serde(default)]
    pub combat: Option<crate::combat::CombatProfile>,
}

/// Mutable actor state. Identity, stats, skills and AI programs live separately.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Combatant {
    pub resources: Resources,
    pub shields: u8,
    pub break_until_round: u32,
    pub statuses: Vec<crate::combat::Status>,
    pub boosted_this_round: bool,
    pub guard: bool,
    pub main_weapon: Option<crate::combat::Weapon>,
    pub cover_by: Option<ActorId>,
    pub cover_hits: u8,
    pub item_stolen: bool,
    pub enemy_charge_max: u8,
    /// CRT sample used by the next-turn speed revision (players still draw).
    pub order_sample: u16,
    /// Discovered weaknesses affect reaction voice CRT draws, even headlessly.
    pub revealed_weapon_weaknesses: u8,
    pub revealed_element_weaknesses: u8,
}
impl Combatant {
    pub fn alive(&self) -> bool {
        self.resources.hp != 0
    }
    pub fn broken(&self) -> bool {
        self.break_until_round != 0
    }
    pub fn has_status(&self, kind: crate::combat::StatusKind) -> bool {
        self.statuses.iter().any(|s| s.kind == kind && s.turns > 0)
    }
}
