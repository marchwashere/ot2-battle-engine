//! Imported Scholar skill definitions. Learning remains caller-controlled.
use crate::content::{Skill,SkillTier,Effect,HitCheck,TargetKind};
pub fn skills()->Vec<Skill> {
    [("ABI_SET_SCH_070","Advanced Magic",TargetKind::Ally,15),
     ("ABI_SET_SCH_011","Fire Storm",TargetKind::AllOpponents,14)].into_iter().map(|(id,name,target,sp_cost)|Skill {
        id:id.into(),name:name.into(),target,sp_cost,hit:HitCheck::Guaranteed,
        effects:vec![Effect::NoEffect],hits:1,
        boosted:(0..3).map(|_|SkillTier {effects:vec![Effect::NoEffect],hits:1}).collect(),
    }).collect()
}
#[derive(serde::Deserialize)]
struct Voice { #[serde(rename="AbilityID")] ability:String, #[serde(rename="VoiceRate")] rate:u32 }
#[derive(serde::Deserialize)]
struct Data {voices:std::collections::BTreeMap<String,Vec<Voice>>}
/// Import asserts these source rows are unconditional, non-PlayOnce cast voices.
/// Keep them separate from the route's 64-bit mask: they never mutate that mask.
pub(crate) fn cast_voice(rng:&mut crate::CrtRng,caster:&str,ability:&str)->crate::Result<()> {
    static DATA:std::sync::OnceLock<Data>=std::sync::OnceLock::new();
    let data=DATA.get_or_init(||serde_json::from_str(include_str!("../data/source/scholar-magic.json")).expect("Scholar source"));
    let row=data.voices.get(caster).and_then(|rows|rows.iter().find(|r|r.ability==ability))
        .ok_or_else(||crate::SimError(format!("Missing Scholar cast voice for {caster}/{ability}")))?;
    rng.weighted_bool(row.rate as f32*0.01);Ok(())
}
