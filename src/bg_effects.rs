//! Source effect descriptions for every Bewildering Grace child.
//! These are execution instructions, not a second lottery or precomputed RNG
//! offsets. Targeting and authored events must be executed between selections.
use crate::{Result, SimError};
use serde::{Deserialize, Serialize};
use std::{collections::{BTreeMap,BTreeSet}, sync::OnceLock};

/// Mutable effects shared by the children of a dance and the remaining battle.
/// Preserve reward modifier requests in source execution order; the reward
/// manager owns resolving them, rather than the lottery declaring a victory.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct BattleModifiers {
    pub exp_requests:Vec<u32>,
    pub jp_requests:Vec<u32>,
    pub money_gained:u32,
    pub items_disabled:bool,
    pub last_act_round:Option<u32>,
    pub guard_hits:Vec<crate::ActorId>,
    /// SetStatusDead saves permanent diseases for ExecRevive.
    pub dead_permanent: BTreeMap<crate::ActorId,Vec<crate::combat::Status>>,
    /// ResetCharacter_PreAction clears this once per parent action, not per BG child.
    pub action_disease_hits: BTreeSet<crate::ActorId>,
}
impl BattleModifiers {
    /// Native SetExpRatioByAbility (140e463f0) and SetJPRatioByAbility
    /// (140e466e0) retain the largest multiplier, never multiply/overwrite it.
    pub fn exp_percent(&self)->u32 {self.exp_requests.iter().copied().max().unwrap_or(100).max(100)}
    pub fn jp_percent(&self)->u32 {self.jp_requests.iter().copied().max().unwrap_or(100).max(100)}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all="snake_case")]
pub enum Target { SelfOnly, PartySingle, PartyAll, EnemySingle, EnemyAll, Everyone }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all="snake_case")]
pub enum EffectKind {
    PhysicalDamage, MagicDamage, RandomTargetMagicDamage, RandomTargetPhysicalDamage, AdditiveShieldDamage,
    ReviveRatio, HpOne, HpFixedRecovery, SpRatioRecovery, SpFixedRecovery,
    SpRatioDamage, BpRecovery, BpDamage, ShieldDamage,
    InstantDeathExceptCaster, RemovePartyDeconditions, AddStatus,
    DisableItems, OrderDelay, LatentRecovery, LatentDamage,
    ReverseBuffs, AddMoney, ExpMultiplier, JpMultiplier,
    TransformEnemy, Summon,
}
#[derive(Debug, Clone, Serialize)]
pub struct Operation {
    pub kind: EffectKind,
    pub source: String,
    pub value: u32,
    pub turns: u8,
    pub disease_rate: u32,
    /// LibAilment target override, independent of AbilityData.TargetType.
    pub target_override: u8,
    pub status: Option<String>,
    /// All slots draw, including None; Silence has range 0..2 rather than 0..0.
    pub status_slots: usize,
    pub status_random_min: u32,
    pub status_random_max: u32,
}
#[derive(Debug, Clone, Serialize)]
pub struct Child {
    pub id: String,
    /// BP_ActionCommandBase's camera flag, retained from the source ability.
    pub use_ability_camera: bool,
    pub target: Target,
    pub attribute: String,
    pub ability_ratio: u32,
    pub hit_ratio: i32,
    pub critical_ratio: i32,
    pub critical_power: u32,
    pub command: String,
    pub revive_recovery: bool,
    pub operations: Vec<Operation>,
    pub events: Vec<AuthoredEvent>,
    pub steps: Vec<Step>,
}
#[derive(Debug,Clone,Serialize,Deserialize)]
#[serde(tag="kind",rename_all="snake_case")]
pub enum Step {
    CounterBranch {tick:f64,destination:f64},
    PatternBranch {tick:f64,ice:f64,thunder:f64},
    Jump {tick:f64,destination:f64},
    RandomTarget {tick:f64},
    Effects {tick:f64,index:Option<usize>,last:bool,#[serde(default)] on_hit:bool},
    Voice {tick:f64,voice_type:u8},
    DiseaseMissUi {tick:f64},
}
impl Step {pub fn tick(&self)->f64 {match *self {Self::RandomTarget{tick}|Self::Effects{tick,..}|Self::Voice{tick,..}|Self::CounterBranch{tick,..}|Self::PatternBranch{tick,..}|Self::Jump{tick,..}|Self::DiseaseMissUi{tick}=>tick}}}
#[derive(Debug,Deserialize)]
pub struct SummonProfile {
    pub id:String,
    parameter:BTreeMap<String,u32>,
    weapon:String,
    pub ability_voice:bool,
    pub initial_idle_frames:u32,
}
impl SummonProfile {
    pub fn combat(&self)->crate::combat::CombatProfile {
        let p=&self.parameter;
        crate::combat::CombatProfile {
            nuts:Default::default(),fire_damage_bonus:0,
            attack:p["ATK"],defense:p["DEF"],magic:p["MATK"],magic_defense:p["MDEF"],
            accuracy:p["ACC"],evasion:p["EVA"],critical:p["CON"],growth_attack:p["ATK"],growth_magic:p["MATK"],
            main_weapon:match self.weapon.as_str() {"EWEAPON_CATEGORY::eNONE"=>None,"EWEAPON_CATEGORY::eROD"=>Some(crate::combat::Weapon::Staff),_=>unreachable!("validated summon weapon")},
            use_cat_damage_type:false,enemy_initialization:None,weapons:vec![],shields:0,
            weapon_weaknesses:vec![],element_weaknesses:vec![],statuses:vec![],steal_item:None,enemy_bp_base:0,
        }
    }
}
pub fn summon(id:&str)->Result<&'static SummonProfile> {
    #[derive(Deserialize)] struct Data {summon_profiles:Vec<SummonProfile>}
    static DATA:OnceLock<Data>=OnceLock::new();
    DATA.get_or_init(||serde_json::from_str(include_str!("../data/source/bg-effects.json")).expect("summon import"))
        .summon_profiles.iter().find(|s|s.id==id).ok_or_else(||SimError(format!("Unknown summon {id}")))
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthoredEvent { pub time: f64, pub body: String }
#[derive(Deserialize)]
#[serde(rename_all="PascalCase")]
struct Ability {
    #[serde(rename="id")] id:String,
    use_ability_camera:bool,
    target_type:String, attribute:String, ability_ratio:u32,
    ability_type:String,
    hit_ratio:i32, critical_ratio:i32, critical_power:u32,
    command_actor:SoftObject, ailment:Vec<AilmentOption>,
}
#[derive(Deserialize)]
struct SoftObject { asset:Asset }
#[derive(Deserialize)]
struct Asset { text:String }
#[derive(Deserialize)]
#[serde(rename_all="PascalCase")]
struct AilmentOption { ailment_name:String, invocation_value:u32, invocation_turn:u8, disease_rate:u32 }
#[derive(Deserialize)]
#[serde(rename_all="PascalCase")]
struct Ailment {
    #[serde(rename="id")] id:String,
    target:String, add_disease:Vec<Disease>, adv_value1:u32, adv_value2:u32,
}
#[derive(Deserialize)]
#[serde(rename_all="PascalCase")]
struct Disease { disease_name:String }
#[derive(Deserialize)]
struct Imported { abilities:Vec<Ability>, ailments:Vec<Ailment>, sequences:BTreeMap<String,Vec<AuthoredEvent>>,steps:BTreeMap<String,Vec<Step>> }

fn classify(name:&str)->Result<EffectKind> {
    use EffectKind::*;
    Ok(match name {
        "RANDOM_TARGET_PHYSICS_DAMAGE"=>RandomTargetPhysicalDamage,"SP_DAMAGE"=>AdditiveShieldDamage,
        "PHYSICS_DAMAGE"=>PhysicalDamage,"MAGIC_DAMAGE"=>MagicDamage,
        "RANDOM_TARGET_MAGIC_DAMAGE"|"RANDOM_TARGET_MAGIC_DAMAGE_FIRE"|"RANDOM_TARGET_MAGIC_DAMAGE_ICE"|"RANDOM_TARGET_MAGIC_DAMAGE_THUNDER"=>RandomTargetMagicDamage,
        "REVIVE_RATIO"=>ReviveRatio,"HP1"=>HpOne,"HP_FIXED_RECOVERY"=>HpFixedRecovery,
        "MP_RATIO_RECOVERY"=>SpRatioRecovery,"MP_FIXED_RECOVERY"=>SpFixedRecovery,
        "MP_RATIO_DAMAGE"=>SpRatioDamage,"BP_RECOVERY"=>BpRecovery,"BP_DAMAGE"=>BpDamage,
        "SP_DAMAGE_DIRECT"=>ShieldDamage,"INSTANT_DEATH_EXCEPT_ENFORCER"=>InstantDeathExceptCaster,
        "REMOVE_DECONDITION_PARTY_ALL"=>RemovePartyDeconditions,
        "ADD_SUPER_MAGIC"|"ADD_FORCE_CRITICAL"|"ADD_SILENCE"|"ADD_POISON_NORMAL"|"ADD_MAGIC_REFLECTION"|"ADD_ATK_UP"|
        "ADD_MATK_UP"|"ADD_DEF_UP"|"ADD_MDEF_UP"|"ADD_AGI_UP"|
        "ADD_DEF_DOWN"|"ADD_MDEF_DOWN"|"ADD_LAST_ACT_SPECIAL"=>AddStatus,
        "ADD_DISABLE_ITEM_MENU"=>DisableItems,"ORDER_COUNT_DOWN"=>OrderDelay,
        "RECOVERY_POTENTIALITY_ACTION"=>LatentRecovery,"DAMAGE_POTENTIALITY_ACTION"=>LatentDamage,
        "REVERSE_BUFF"=>ReverseBuffs,"GET_LEAF"=>AddMoney,"EXPUP"=>ExpMultiplier,"JPUP"=>JpMultiplier,
        "ENE_NML_OTR_130"|"ENE_NML_OTR_140"|"ENE_NML_OTR_150"|
        "ENE_NML_WTR_510"|"ENE_NML_WTR_590"|"ENE_NML_WTR_600"=>TransformEnemy,
        "PC_ABI_MAKA_001_INVADE"|"PC_ABI_MAKA_002_INVADE"=>Summon,
        _=>return Err(SimError(format!("Unimplemented BG effect definition {name}"))),
    })
}
fn import()->Result<BTreeMap<String,Child>> {
    let mut d:Imported=serde_json::from_str(include_str!("../data/source/bg-effects.json"))?;
    let scholar:Imported=serde_json::from_str(include_str!("../data/source/scholar-magic.json"))?;
    d.abilities.extend(scholar.abilities);
    for row in scholar.ailments {if !d.ailments.iter().any(|r|r.id==row.id) {d.ailments.push(row);}}
    d.sequences.extend(scholar.sequences);d.steps.extend(scholar.steps);
    let mut children=BTreeMap::new();
    for a in d.abilities {
        let target=match a.target_type.rsplit("::").next().unwrap() {
            "eSELF"=>Target::SelfOnly,"ePARTY_SINGLE"=>Target::PartySingle,
            "ePARTY_ALL"=>Target::PartyAll,"eENEMY_SINGLE"=>Target::EnemySingle,
            "eENEMY_ALL"=>Target::EnemyAll,"eALL"=>Target::Everyone,
            x=>return Err(SimError(format!("Unknown BG target {x}"))),
        };
        let mut operations=Vec::new();
        for o in a.ailment.into_iter().filter(|o|o.ailment_name!="None") {
            let kind=classify(&o.ailment_name)?;
            let row=d.ailments.iter().find(|r|r.id==o.ailment_name);
            if !matches!(kind,EffectKind::TransformEnemy|EffectKind::Summon) && row.is_none() {
                return Err(SimError(format!("Missing BG ailment {}",o.ailment_name)));
            }
            let target_override=row.map(|r|r.target.rsplit("NewEnumerator").next().unwrap().parse::<u8>())
                .transpose().map_err(|e|SimError(e.to_string()))?.unwrap_or(0);
            let status=row.and_then(|r|r.add_disease.iter().find(|s|s.disease_name!="None")).map(|s|s.disease_name.clone());
            if kind==EffectKind::AddStatus && status.is_none() {return Err(SimError("Missing BG disease".into()));}
            operations.push(Operation {kind,source:o.ailment_name,value:o.invocation_value,turns:o.invocation_turn,
                disease_rate:o.disease_rate,target_override,status,
                status_slots:row.map_or(0,|r|r.add_disease.len()),
                status_random_min:row.map_or(0,|r|r.adv_value1),status_random_max:row.map_or(0,|r|r.adv_value2)});
        }
        let events=d.sequences.get(&a.id).ok_or_else(||SimError(format!("Missing BG sequence {}",a.id)))?.clone();
        let steps=d.steps.get(&a.id).ok_or_else(||SimError(format!("Missing BG executable sequence {}",a.id)))?.clone();
        children.insert(a.id.clone(),Child {id:a.id,use_ability_camera:a.use_ability_camera,target,attribute:a.attribute,ability_ratio:a.ability_ratio,
            hit_ratio:a.hit_ratio,critical_ratio:a.critical_ratio,critical_power:a.critical_power,
            command:a.command_actor.asset.text,revive_recovery:a.ability_type.ends_with("::eREVIVE_RECOVERY"),operations,events,steps});
    }
    for n in 1..=39 {crate::ensure(children.contains_key(&format!("ABI_DAN_MAKA_{n:03}")),"Missing BG child")?;}
    Ok(children)
}
pub fn children()-> &'static BTreeMap<String,Child> {
    static DATA:OnceLock<BTreeMap<String,Child>>=OnceLock::new();
    DATA.get_or_init(||import().expect("source BG effects must be complete"))
}
pub fn child(id:&str)->Result<&'static Child> {
    children().get(id).ok_or_else(||SimError(format!("Unknown BG child {id}")))
}

#[derive(Debug, Clone, Copy)]
pub struct TargetActor { pub id:crate::ActorId, pub side:crate::Side, pub alive:bool }
impl Child {
    /// GetAbilityTarget's execution-flow stack runs the dead/stealth filter
    /// before the random-single selection. In the enemy branch, Push(1502),
    /// Push(1746) means filter block 1746 runs before selection block 1502.
    /// Revive+recovery retains dead party members; the remaining handlers decide
    /// which resources to apply to them.
    pub fn select_targets(&self,caster:crate::ActorId,actors:&[TargetActor],rng:&mut crate::CrtRng)->Result<Vec<crate::ActorId>> {
        crate::ensure(actors.iter().any(|a|a.id==caster&&a.side==crate::Side::Player),"BG caster is not in the player party")?;
        crate::ensure(actors.iter().enumerate().all(|(i,a)|!actors[..i].iter().any(|b|b.id==a.id)),"Duplicate BG actor ID")?;
        let mut selected:Vec<_>=actors.iter().filter(|a|match self.target {
            Target::SelfOnly=>a.id==caster,
            Target::PartySingle|Target::PartyAll=>a.side==crate::Side::Player,
            Target::EnemySingle|Target::EnemyAll=>a.side==crate::Side::Enemy,
            Target::Everyone=>true,
        }).filter(|a|a.alive||self.revive_recovery).collect();
        if matches!(self.target,Target::PartySingle|Target::EnemySingle) && !selected.is_empty() {
            let chosen=selected[rng.index(selected.len() as u32) as usize];
            selected=vec![chosen];
        }
        Ok(selected.into_iter().map(|a|a.id).collect())
    }
}

impl Operation {
    /// LibAilment.GetAilmentTarget overrides the child target list. In particular,
    /// instant death removes the caster AFTER the random party-single selection:
    /// selecting Agnea does not reroll another party member.
    pub fn select_targets(&self,caster:crate::ActorId,default:&[crate::ActorId],party:&[crate::ActorId],rng:&mut crate::CrtRng)->Result<Vec<crate::ActorId>> {
        Ok(match self.target_override {
            0=>default.to_vec(),
            1=>vec![caster],
            2=>if default.is_empty() {vec![]} else {vec![default[rng.index(default.len() as u32) as usize]]},
            3=>party.to_vec(),
            9=>default.iter().copied().filter(|id|*id!=caster).collect(),
            x=>return Err(SimError(format!("Unsupported BG target override {x}"))),
        })
    }
    /// CheckAddDisease + the non-random disease-array loop. Return each slot's
    /// additive duration, including None slots, so execution retains CRT order.
    pub fn roll_status_slots(&self,rng:&mut crate::CrtRng)->Result<Option<Vec<u32>>> {
        crate::ensure(self.kind==EffectKind::AddStatus,"Not a BG status operation")?;
        crate::ensure(self.status_random_max>=self.status_random_min,"Invalid BG status random range")?;
        let rate=if self.disease_rate==0 {100} else {self.disease_rate};
        if !rng.weighted_bool(rate as f32*0.01_f32) {return Ok(None);}
        Ok(Some((0..self.status_slots).map(|_|self.status_random_min+rng.index(self.status_random_max-self.status_random_min+1)).collect()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_authored_disease_miss_display_is_imported() {
        let mut count=0;
        for child in children().values() {
            let authored:Vec<_>=child.events.iter().filter(|e|e.body.contains("LocalVirtualFunction ExecPlayDiseaseMissUI")).map(|e|e.time).collect();
            let imported:Vec<_>=child.steps.iter().filter_map(|s|if let Step::DiseaseMissUi {tick}=s {Some(*tick)}else{None}).collect();
            assert_eq!(authored,imported,"{}",child.id);
            count+=authored.len();
        }
        assert_eq!(count,5); // death, poison, mist, reflection, and Critical Scope
    }
    #[test]
    fn every_command_retains_its_required_source_camera_flag() {
        for json in [include_str!("../data/source/bg-effects.json"),include_str!("../data/source/scholar-magic.json")] {
            let source:serde_json::Value=serde_json::from_str(json).unwrap();
            for row in source["abilities"].as_array().unwrap() {
                let id=row["id"].as_str().unwrap();
                assert_eq!(Some(child(id).unwrap().use_ability_camera),row["UseAbilityCamera"].as_bool(),"{id}");
            }
            let mut missing=source["abilities"][0].clone();
            missing.as_object_mut().unwrap().remove("UseAbilityCamera");
            assert!(serde_json::from_value::<Ability>(missing).is_err(),"Missing camera data must not silently default to false");
        }
    }
    #[test]
    fn all_39_children_and_both_nested_summons_are_present() {
        assert_eq!(children().len(),56);
        for n in 1..=4 {for skill in ["011","070"] {assert!(child(&format!("ABI_SCH_{skill}_0{n}")).is_ok());}}
        for id in ["ABI_WAR_010_03","ABI_WAR_010_04","ABI_SCH_060_04","ABI_CLE_020_01","ABI_INV_030_01","ABI_MER_YOHEI_05_01"] {assert!(child(id).is_ok());}
        for n in 1..=39 {assert!(child(&format!("ABI_DAN_MAKA_{n:03}")).is_ok());}
        assert!(child("ABI_DAN_MAKA_006_INV").is_ok());
        assert!(child("ABI_DAN_MAKA_017_INV").is_ok());
        assert!(child("made_up").is_err());assert!(classify("made_up").is_err());
    }
    #[test]
    fn harmful_effects_keep_their_actual_source_parameters() {
        let silence=&child("ABI_DAN_MAKA_024").unwrap().operations[0];
        assert_eq!((silence.turns,silence.status_random_min,silence.status_random_max,silence.status_slots),(2,0,2,20));
        assert_eq!(child("ABI_DAN_MAKA_014").unwrap().operations[0].turns,9);
        assert_eq!(child("ABI_DAN_MAKA_004").unwrap().ability_ratio,900);
        assert_eq!(child("ABI_DAN_MAKA_023").unwrap().ability_ratio,50);
        assert_eq!(child("ABI_DAN_MAKA_039").unwrap().target,Target::Everyone);
        let mushroom=child("ABI_DAN_MAKA_017_INV").unwrap();
        assert_eq!(mushroom.target,Target::EnemyAll);
        assert_eq!(mushroom.operations.len(),5);
        assert!(mushroom.operations.iter().all(|e|e.kind==EffectKind::AddStatus&&e.turns==5));
        assert_eq!(child("ABI_DAN_MAKA_003").unwrap().operations.iter().map(|o|o.value).sum::<u32>(),100);
        let death=&child("ABI_DAN_MAKA_008").unwrap().operations[0];
        assert_eq!((death.disease_rate,death.target_override),(50,9));
    }
    #[test]
    fn instant_death_can_select_caster_and_is_not_rerolled() {
        use crate::{ActorId,Side,CrtRng};
        let actors=[TargetActor{id:ActorId(0),side:Side::Player,alive:true}];
        let death=child("ABI_DAN_MAKA_008").unwrap();let mut rng=CrtRng::new(0);
        let targets=death.select_targets(ActorId(0),&actors,&mut rng).unwrap();
        assert_eq!(rng.draws,1);
        assert!(death.operations[0].select_targets(ActorId(0),&targets,&[ActorId(0)],&mut rng).unwrap().is_empty());
        assert_eq!(rng.draws,1);
    }
    #[test]
    fn random_single_filters_dead_targets_before_drawing() {
        use crate::{ActorId,Side,CrtRng};
        let actors=[
            TargetActor{id:ActorId(0),side:Side::Player,alive:true},
            TargetActor{id:ActorId(1),side:Side::Player,alive:false},
            TargetActor{id:ActorId(2),side:Side::Enemy,alive:false},
            TargetActor{id:ActorId(3),side:Side::Enemy,alive:true},
            TargetActor{id:ActorId(4),side:Side::Enemy,alive:false},
        ];
        for seed in [0,1,0x3899b25a,u32::MAX] {
            let mut rng=CrtRng::new(seed);
            assert_eq!(child("ABI_DAN_MAKA_022").unwrap().select_targets(ActorId(0),&actors,&mut rng).unwrap(),vec![ActorId(3)]);
            assert_eq!(rng.draws,1); // Even the singleton target draws.
            assert_eq!(child("ABI_DAN_MAKA_008").unwrap().select_targets(ActorId(0),&actors,&mut rng).unwrap(),vec![ActorId(0)]);
            assert_eq!(rng.draws,2);
        }
        let mut dead=actors;
        dead[3].alive=false;
        let mut rng=CrtRng::new(0);
        assert!(child("ABI_DAN_MAKA_022").unwrap().select_targets(ActorId(0),&dead,&mut rng).unwrap().is_empty());
        assert_eq!(rng.draws,0);
    }
    #[test]
    fn silence_draws_each_slot_and_revive_keeps_dead_targets() {
        use crate::{ActorId,Side,CrtRng};
        let silence=&child("ABI_DAN_MAKA_024").unwrap().operations[0];
        let mut rng=CrtRng::new(0);let slots=silence.roll_status_slots(&mut rng).unwrap().unwrap();
        assert_eq!(rng.draws,21);assert_eq!(slots.len(),20);assert!(slots.iter().all(|&v|v<=2));
        let actors=[TargetActor{id:ActorId(0),side:Side::Player,alive:true},TargetActor{id:ActorId(1),side:Side::Player,alive:false}];
        let before=rng;
        assert_eq!(child("ABI_DAN_MAKA_005").unwrap().select_targets(ActorId(0),&actors,&mut rng).unwrap().len(),2);
        assert_eq!(child("ABI_DAN_MAKA_024").unwrap().select_targets(ActorId(0),&actors,&mut rng).unwrap().len(),1);
        assert_eq!(rng,before);
    }
}

pub fn transform_immune(id:&str)->bool {
    #[derive(Deserialize)] struct Data {transform_resists:BTreeMap<String,bool>}
    static DATA:OnceLock<Data>=OnceLock::new();
    DATA.get_or_init(||serde_json::from_str(include_str!("../data/source/bg-effects.json")).expect("transform resistance import")).transform_resists.get(id).copied().unwrap_or(true)
}
