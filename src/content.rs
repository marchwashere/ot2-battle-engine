use crate::{Result, SimError, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Resource {
    Hp,
    Sp,
    Bp,
    Latent,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Recovery {
    Fixed { amount: u32 },
    Percent { percent: u8 },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Effect {
    Restore {
        resource: Resource,
        recovery: Recovery,
    },
    /// Explicit fixed damage for the scaffold; this is NOT an OT2 attack formula.
    FixedDamage { amount: u32 },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    SelfOnly,
    Ally,
    AllAllies,
    Opponent,
    AllOpponents,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum HitCheck {
    Guaranteed,
    Weighted { probability: f32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub target: TargetKind,
    pub sp_cost: u32,
    pub hit: HitCheck,
    pub effects: Vec<Effect>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub target: TargetKind,
    pub effects: Vec<Effect>,
    /// Original ItemDB label, retained for data provenance and later importers.
    pub source_label: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemCatalog {
    pub schema_version: u32,
    pub source: String,
    pub source_sha256: String,
    pub items: Vec<Item>,
}
impl ItemCatalog {
    pub fn builtin() -> Result<Self> {
        Self::from_json(include_str!("../data/items.json"))
    }
    pub fn from_json(json: &str) -> Result<Self> {
        let value: Self = serde_json::from_str(json)?;
        ensure(value.schema_version == 1, "unsupported item schema_version")?;
        Ok(value)
    }
}

/// Immutable content shared by battle clones. Strings resolve to dense indices
/// at setup/command boundaries, not by scanning records during each effect.
#[derive(Debug)]
pub struct Catalog {
    pub(crate) skills: Vec<Skill>,
    pub(crate) items: Vec<Item>,
    skill_ids: BTreeMap<String, usize>,
    item_ids: BTreeMap<String, usize>,
}
impl Catalog {
    pub fn new(skills: Vec<Skill>, items: ItemCatalog) -> Result<Self> {
        ensure(items.schema_version == 1, "unsupported item schema_version")?;
        let mut skill_ids = BTreeMap::new();
        let mut item_ids = BTreeMap::new();
        for (i, skill) in skills.iter().enumerate() {
            ensure(
                !skill.id.is_empty() && skill_ids.insert(skill.id.clone(), i).is_none(),
                "empty or duplicate skill id",
            )?;
            if let HitCheck::Weighted { probability } = skill.hit {
                ensure(
                    probability.is_finite() && (0.0..=1.0).contains(&probability),
                    "hit probability must be finite and within 0..1",
                )?;
            }
            validate_effects(&skill.effects)?;
        }
        for (i, item) in items.items.iter().enumerate() {
            ensure(
                !item.id.is_empty() && item_ids.insert(item.id.clone(), i).is_none(),
                "empty or duplicate item id",
            )?;
            ensure(
                matches!(
                    item.target,
                    TargetKind::Ally | TargetKind::AllAllies | TargetKind::SelfOnly
                ),
                "healing items must target allies",
            )?;
            validate_effects(&item.effects)?;
            ensure(
                item.effects
                    .iter()
                    .all(|e| matches!(e, Effect::Restore { .. })),
                "only recovery items are supported",
            )?;
        }
        Ok(Self {
            skills,
            items: items.items,
            skill_ids,
            item_ids,
        })
    }
    pub fn skill(&self, id: &str) -> Result<&Skill> {
        Ok(&self.skills[self.skill_id(id)?])
    }
    pub fn item(&self, id: &str) -> Result<&Item> {
        Ok(&self.items[self.item_id(id)?])
    }
    pub(crate) fn skill_id(&self, id: &str) -> Result<usize> {
        self.skill_ids
            .get(id)
            .copied()
            .ok_or_else(|| SimError(format!("unknown skill: {id}")))
    }
    pub(crate) fn item_id(&self, id: &str) -> Result<usize> {
        self.item_ids
            .get(id)
            .copied()
            .ok_or_else(|| SimError(format!("unknown item: {id}")))
    }
}
fn validate_effects(effects: &[Effect]) -> Result<()> {
    ensure(!effects.is_empty(), "an action needs at least one effect")?;
    for effect in effects {
        match effect {
            Effect::Restore {
                recovery: Recovery::Percent { percent },
                ..
            } => ensure(*percent <= 100, "recovery percent exceeds 100")?,
            Effect::Restore {
                recovery: Recovery::Fixed { amount },
                ..
            }
            | Effect::FixedDamage { amount } => ensure(
                *amount <= i32::MAX as u32,
                "effect amount exceeds supported range",
            )?,
        }
    }
    Ok(())
}
