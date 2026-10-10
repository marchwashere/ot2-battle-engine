//! Scalar job assignment for carried original-growth characters. The caller
//! owns licenses, learned abilities, JP and support equipment. No RNG is used.
use crate::{PlayerCharacter, Result, SimError, combat::{Weapon,WeaponStats}, ensure, progression};
use serde::{Deserialize,Serialize};
use std::{collections::BTreeMap,sync::OnceLock};

pub const SCHOLAR: u32 = 6;
#[derive(Debug,Clone,Deserialize,Serialize)]
pub struct DefaultWeapon {pub weapon:Weapon,pub source_id:String,pub attack:u32,pub magic:u32}
#[derive(Debug,Clone,Serialize)]
pub struct EquipmentChange {
    pub previous_job:Option<u32>,
    pub secondary_job:Option<u32>,
    pub added:Vec<DefaultWeapon>,
    /// Stats are retained because the carried combat type does not store item IDs.
    pub removed:Vec<WeaponStats>,
    pub stat_delta:BTreeMap<String,i64>,
}
#[derive(Debug,Deserialize,Serialize)]
pub struct ScholarAbility {
    pub id:String,
    pub name:String,
    pub default_on_assignment:bool,
    pub sp_cost:u32,
    pub tier_ids:Vec<String>,
    /// Unmodified source AbilityData rows. These do not imply CRT action support.
    pub tiers:Vec<serde_json::Value>,
}
#[derive(Deserialize)]
struct Job {id:u32,revision:BTreeMap<String,u32>,permitted_weapons:Vec<Weapon>}
#[derive(Deserialize)]
struct Character {id:String,main_job:u32,permitted_weapons:Vec<Weapon>}
#[derive(Deserialize)]
struct Data {scholar:Job,#[serde(default)] additional_jobs:Vec<Job>,characters:BTreeMap<u32,Character>,default_weapons:Vec<DefaultWeapon>,
    scholar_abilities:Vec<ScholarAbility>,maximum_hp_sp:u32,maximum_stat:u32}
fn data()->&'static Data {
    static DATA:OnceLock<Data>=OnceLock::new();
    DATA.get_or_init(||serde_json::from_str(include_str!("../data/jobs.json")).expect("imported job definitions"))
}
pub fn scholar_abilities()->&'static [ScholarAbility] {&data().scholar_abilities}
fn character(id:u32)->Result<&'static Character> {
    data().characters.get(&id).ok_or_else(||SimError(format!("Unsupported job character {id}")))
}
fn job(job:Option<u32>)->Result<Option<&'static Job>> {
    match job {None=>Ok(None),Some(id) if id==data().scholar.id=>Ok(Some(&data().scholar)),
        Some(id)=>data().additional_jobs.iter().find(|j|j.id==id).map(Some).ok_or_else(||SimError(format!("Unsupported secondary job {id}")))}
}
/// Native Multiply_CharacterParam_RevisionParam: integer product, binary32
/// multiplication by .01, then FloorToInt. Never multiply flat gear bonuses.
fn revised(value:u32,percent:u32)->u32 {((value*percent) as f32*0.01_f32).floor() as u32}
fn parameters(id:u32,level:u32,secondary:Option<u32>)->Result<BTreeMap<String,u32>> {
    let growth=progression::growth_at_level(id,level)?;
    let job=job(secondary)?;
    Ok(growth.parameters.iter().map(|(k,&v)|{
        let value=job.map_or(v,|j|revised(v,j.revision[k]));
        let value=match k.as_str() {"HP"|"MP"=>value.min(data().maximum_hp_sp),
            "ATK"|"DEF"|"MATK"|"MDEF"|"ACC"|"EVA"|"CON"|"AGI"=>value.min(data().maximum_stat),_=>value};
        (k.clone(),value)
    }).collect())
}
fn apply_delta(player:&mut PlayerCharacter,before:&BTreeMap<String,u32>,after:&BTreeMap<String,u32>)
    ->Result<BTreeMap<String,i64>> {
    let mut deltas=BTreeMap::new();
    let mut update=|value:&mut u32,key:&str|->Result<()> {
        let delta=i64::from(after[key])-i64::from(before[key]);
        *value=u32::try_from(i64::from(*value)+delta).map_err(|_|SimError(format!("Invalid carried {key} for job revision")))?;
        if delta!=0 {deltas.insert(key.to_owned(),delta);}Ok(())
    };
    update(&mut player.stats.max_hp,"HP")?;update(&mut player.stats.max_sp,"MP")?;update(&mut player.stats.speed,"AGI")?;
    let c=player.combat.as_mut().ok_or_else(||SimError("Missing carried combat stats".into()))?;
    for (value,key) in [(&mut c.attack,"ATK"),(&mut c.defense,"DEF"),(&mut c.magic,"MATK"),
        (&mut c.magic_defense,"MDEF"),(&mut c.accuracy,"ACC"),(&mut c.evasion,"EVA"),(&mut c.critical,"CON")] {update(value,key)?;}
    Ok(deltas)
}
pub fn permitted_weapons(id:u32,secondary:Option<u32>)->Result<Vec<Weapon>> {
    let character=character(id)?;
    let mut permitted=character.permitted_weapons.clone();
    if let Some(j)=job(secondary)? {for &weapon in &j.permitted_weapons {if !permitted.contains(&weapon) {permitted.push(weapon);}}}
    Ok(permitted)
}
fn normal(weapon:Weapon)->&'static str {match weapon {
    Weapon::Sword=>"attack_sword",Weapon::Lance=>"attack_lance",Weapon::Dagger=>"attack_dagger",
    Weapon::Axe=>"attack_axe",Weapon::Bow=>"attack_bow",Weapon::Staff=>"attack_staff",
}}

/// Apply/remove Scholar using the supplied previous job at this exact level.
/// Preserves flat gear/support contributions and all carried resources; HP/SP
/// are clamped to the new maxima, never healed. No stat-boosting nuts are modeled.
/// Learned job abilities remain the caller's responsibility. Newly permitted
/// empty weapon slots receive source default equipment, not inventory awards.
pub fn change_secondary_job(player:&mut PlayerCharacter,id:u32,level:u32,previous:Option<u32>,next:Option<u32>)
    ->Result<EquipmentChange> {
    let ch=character(id)?;ensure(player.id==ch.id,"Carried character/job identity mismatch")?;
    job(previous)?;job(next)?;
    ensure(previous!=Some(ch.main_job),"Main job cannot also be the previous secondary job")?;
    let next=next.filter(|&id|id!=ch.main_job); // ChangePlayerSecondJob chooses -1 for the main job.
    let mut updated=player.clone();
    let before=parameters(id,level,previous)?;let after=parameters(id,level,next)?;
    let stat_delta=apply_delta(&mut updated,&before,&after)?;
    let previous_permitted=permitted_weapons(id,previous)?;
    let permitted=permitted_weapons(id,next)?;
    let c=updated.combat.as_mut().unwrap();
    ensure(c.main_weapon.is_none_or(|w|permitted.contains(&w)),
        "Removing the selected main weapon requires an explicit supported weapon selection")?;
    let removed=c.weapons.iter().filter(|w|!permitted.contains(&w.weapon)).cloned().collect::<Vec<_>>();
    c.weapons.retain(|w|permitted.contains(&w.weapon));
    for removed in &removed {updated.skills.retain(|s|s!=normal(removed.weapon));}
    let mut added=Vec::new();
    for default in &data().default_weapons {
        if permitted.contains(&default.weapon) && !c.weapons.iter().any(|w|w.weapon==default.weapon) {
            c.weapons.push(WeaponStats {weapon:default.weapon,attack:default.attack,magic:default.magic});
            if !updated.skills.iter().any(|s|s==normal(default.weapon)) {updated.skills.push(normal(default.weapon).into());}
            added.push(default.clone());
        }
    }
    for &weapon in &permitted {
        if !previous_permitted.contains(&weapon) && !updated.skills.iter().any(|s|s==normal(weapon)) {
            updated.skills.push(normal(weapon).into());
        }
    }
    if let Some(r)=updated.resources.as_mut() {r.hp=r.hp.min(updated.stats.max_hp);r.sp=r.sp.min(updated.stats.max_sp);}
    *player=updated;
    Ok(EquipmentChange {previous_job:previous,secondary_job:next,added,removed,stat_delta})
}

/// Character level changes with the assigned job's exact stat deltas. Retains
/// equipment and resource values; the caller applies its level-up healing rule.
pub fn apply_growth(player:&mut PlayerCharacter,id:u32,from:u32,to:u32,secondary:Option<u32>)->Result<()> {
    if secondary.is_none() {return progression::apply_growth(player,id,from,to);}
    let ch=character(id)?;ensure(player.id==ch.id,"Carried character/job identity mismatch")?;
    ensure(secondary!=Some(ch.main_job),"Main job cannot also be the secondary job")?;
    let before=parameters(id,from,secondary)?;let after=parameters(id,to,secondary)?;
    let mut updated=player.clone();apply_delta(&mut updated,&before,&after)?;
    let growth=progression::growth_at_level(id,to)?;
    let c=updated.combat.as_mut().unwrap();c.growth_attack=growth.growth_attack;c.growth_magic=growth.growth_magic;
    *player=updated;Ok(())
}
