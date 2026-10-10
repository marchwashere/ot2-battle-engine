//! Bounded first-menu Escape projection for ordinary daytime highroad battles.
//! Entry follows field composition and the party timing draw. No enemy action is
//! exposed: normal openings promote the first player; preemptive starts omit enemies.
use crate::{
    ActorId, Battle, BattleConfig, Command, CrtRng, Enemy as EnemyDefinition, Outcome, Result,
    SimError, StepReport, combat, ensure,
    escape::{Situation, success_percent},
    opening_crt::{self, RngSpan, VoiceCondition},
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::OnceLock};

#[derive(Debug, Deserialize)]
struct Data {
    enemies: BTreeMap<String, Enemy>,
    levels: Vec<Level>,
    osvald_levels: Vec<Level>,
    route_base_exp: BTreeMap<String, u32>,
}
#[derive(Debug, Deserialize)]
struct Enemy {
    speed: u32,
    bp_base: u8,
    initialization: combat::EnemyInitialization,
}
#[derive(Debug, Deserialize)]
struct Level {
    level: u32,
    total_exp: u32,
    max_hp: u32,
    speed: u32,
}
fn data() -> &'static Data {
    static DATA: OnceLock<Data> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../data/field_escape.json"))
            .expect("field escape import")
    })
}
#[derive(Deserialize)]
struct CarriedPlayer {voice:usize,latent_max:u16}
#[derive(Deserialize)]
struct CarriedData {players:BTreeMap<String,CarriedPlayer>,initial_bp:u8}
fn carried_data()->&'static CarriedData {
    static DATA:OnceLock<CarriedData>=OnceLock::new();
    DATA.get_or_init(||serde_json::from_str(include_str!("../data/field_carried.json")).expect("carried field definitions"))
}
pub fn level_for_experience(exp: u32) -> u32 {
    data()
        .levels
        .iter()
        .rev()
        .find(|r| r.total_exp <= exp)
        .unwrap()
        .level
}
pub fn experience_for_level(level:u32)->Result<u32> {
    data().levels.iter().find(|r|r.level==level).map(|r|r.total_exp).ok_or_else(||SimError("Missing player level".into()))
}
pub fn route_battle_base_exp(group: &str) -> Result<u32> {
    data()
        .route_base_exp
        .get(group)
        .copied()
        .ok_or_else(|| SimError(format!("Missing EXP for {group}")))
}

#[derive(Debug,Deserialize)]
struct FieldReward { exp:u32,jp:u32,money:u32,drop_item:String,drop_rate:u32 }
#[derive(Debug,Serialize)]
pub struct FieldRareVictory {
    pub enemy_knowledge: crate::knowledge::EnemyHistory,
    pub at_command_menu:CrtRng,
    pub after:CrtRng,
    pub actor:usize,
    pub report:StepReport,
    pub trace:Vec<RngSpan>,
    pub base_exp:u32,
    pub base_jp:u32,
    pub base_money:u32,
    pub drops:BTreeMap<String,u32>,
    pub party_resources_after:Vec<crate::Resources>,
}

#[cfg(test)]
mod rare_regression {
    use super::*;
    #[test]
    fn repeated_study_matches_every_captured_second_encounter_draw() {
        let mut entry=CrtRng::new(0);entry.advance(6514);
        let rare=opening_shadow_soulstone_m(entry,Situation::Normal,&[5,1],
            &["ENE_NML_FLD_SNW_110","ENE_NML_OTR_140","ENE_NML_FLD_SNW_150"],false).unwrap().unwrap();
        let mut second=CrtRng::new(0);second.advance(6581);
        let history=rare.enemy_knowledge.clone();
        let menu=FieldEscapeMenu::with_history(second,Situation::Normal,&[14,14],
            &["ENE_NML_FLD_SNW_110","ENE_NML_FLD_SNW_110","ENE_NML_FLD_SNW_150"],&history).unwrap();
        assert_eq!(menu.rng.draws,6630);
        assert_eq!(history,rare.enemy_knowledge,"Initialization must not mutate incoming history");
        let attempt=menu.attempt(&[],14).unwrap();
        assert!(!attempt.success);
        let live:serde_json::Value=serde_json::from_str(include_str!("../data/source/live-western-second-escape-6630.json")).unwrap();
        let records=live["records"].as_array().unwrap();
        for span in &attempt.trace {
            for n in span.before.draws..span.after.draws {
                let r=&records[(n-6581) as usize];
                let mut rng=span.before;rng.advance(n-span.before.draws);
                assert_eq!(rng.state,r["before"].as_u64().unwrap() as u32,"{} at {n}",span.reason);
                let node=r["blueprint"]["node"].as_str().unwrap_or("");
                if span.reason=="study_foe_weakness" {assert!(node.ends_with(".OpenWeakIcon"));}
                if span.reason=="escape_check" {assert!(node.ends_with(".CheckEscapeSuccess"));}
            }
        }
        assert_eq!(attempt.trace.iter().filter(|s|s.reason=="study_foe_weakness").count(),4);
    }
    #[test]
    fn fully_known_and_duplicate_types_do_not_repeat_random_reveals() {
        let entry=CrtRng::new(17);
        let ids=["ENE_NML_FLD_SNW_110","ENE_NML_FLD_SNW_110"];
        let mut history=crate::knowledge::EnemyHistory::new();
        let first=FieldEscapeMenu::with_history(entry,Situation::Normal,&[14,14],&ids,&history).unwrap();
        assert_eq!(first.trace.iter().filter(|s|s.reason=="study_foe_weakness").count(),1);
        history=first.enemy_knowledge;
        let second=FieldEscapeMenu::with_history(entry,Situation::Normal,&[14,14],&ids,&history).unwrap();
        assert_eq!(second.trace.iter().filter(|s|s.reason=="study_foe_weakness").count(),2);
        history=second.enemy_knowledge;
        let third=FieldEscapeMenu::with_history(entry,Situation::Normal,&[14,14],&ids,&history).unwrap();
        assert_eq!(third.trace.iter().filter(|s|s.reason=="study_foe_weakness").count(),1);
        history=third.enemy_knowledge;
        let full=FieldEscapeMenu::with_history(entry,Situation::Normal,&[14,14],&ids,&history).unwrap();
        assert!(!full.trace.iter().any(|s|s.reason=="study_foe_weakness"));
    }
    #[test]
    fn cultured_cait_aoe_reaction_is_aggregated_after_damage() {
        let mut entry=CrtRng::new(0);entry.advance(6514);
        let v=opening_shadow_soulstone_m(entry,Situation::Normal,&[5,1],&["ENE_NML_FLD_SNW_110","ENE_NML_OTR_140","ENE_NML_FLD_SNW_150"],false).unwrap().unwrap();
        assert!(v.report.events.iter().any(|e|matches!(e,crate::Event::Damage {target,amount:11,..} if target.0==3)));
        assert_eq!(v.at_command_menu.draws,6563);
        assert_eq!(v.after.draws,6572);
        let reasons:Vec<_>=v.trace.iter().filter(|s|s.before.draws>=6563).map(|s|s.reason).collect();
        assert_eq!(reasons,vec!["item_voice","damage_notification_timer","damage_notification_timer","damage_notification_timer","kill_voice","victory_voice","enemy_drop","enemy_drop","enemy_drop"]);
        let live:serde_json::Value=serde_json::from_str(include_str!("../data/source/live-field-rare-cultured.json")).unwrap();
        let records=live["records"].as_array().unwrap();
        let action:Vec<_>=records.iter().filter(|r|r["position"].as_u64().unwrap()>=6563&&r["position"].as_u64().unwrap()<6572).collect();
        for (span,record) in v.trace.iter().filter(|s|s.before.draws>=6563).zip(action) {
            assert_eq!(span.before.draws,record["position"].as_u64().unwrap());
            assert_eq!(span.before.state,record["before"].as_u64().unwrap() as u32);
            let expected=match span.reason {"damage_notification_timer"=>"0x2C52E57",_=>"0x2C57B26"};
            assert_eq!(record["returnRva"].as_str().unwrap(),expected);
        }
        assert!(records.iter().filter(|r|r["position"].as_u64().unwrap()>=6572).all(|r|r["blueprint"]["node"].as_str().unwrap().ends_with(".CalcEncountStepCount")));
    }
}

/// One Shadow Soulstone (M) at the first player menu. Reject surviving packs;
/// this bounded API never invents an enemy action or a subsequent turn.
pub fn opening_shadow_soulstone_m(entry:CrtRng,situation:Situation,levels:&[u32],enemies:&[&str],night:bool)->Result<Option<FieldRareVictory>> {
    opening_shadow_soulstone_m_with_history(entry,situation,levels,enemies,night,&Default::default())
}
pub fn opening_shadow_soulstone_m_with_history(entry:CrtRng,situation:Situation,levels:&[u32],enemies:&[&str],night:bool,history:&crate::knowledge::EnemyHistory)->Result<Option<FieldRareVictory>> {
    ensure(levels.len()==2,"Rare route requires Throné and Osvald")?;
    if situation==Situation::Surprise {return Ok(None);}
    let mut config=break_data().party_config.clone();
    for (i,(&id,base)) in [(7,5),(6,1)].iter().map(|(id,base)|(id,*base)).enumerate() {
        crate::progression::apply_growth(&mut config.players[i],id,base,levels[i])?;
        config.players[i].resources=Some(crate::Resources {hp:config.players[i].stats.max_hp,sp:config.players[i].stats.max_sp,bp:1,latent:0});
    }
    config.seed=entry.state;config.night_buff_turns=if night {3}else{0};
    config.enemies=enemies.iter().map(|id|break_data().enemies.get(*id).cloned().ok_or_else(||SimError(format!("Missing rare-fight enemy {id}")))).collect::<Result<_>>()?;
    if config.enemies.iter().any(|e| {let c=e.combat.as_ref().unwrap();let damage=combat::elemental_fixed_damage(1100,c.element_weaknesses.contains(&combat::Element::Dark),false,c.use_cat_damage_type);e.stats.max_hp>damage}) {return Ok(None);}
    config.inventory.insert("shadow_soulstone_m".into(),1);
    config.enemy_knowledge=history.clone();
    let mut battle=Battle::new_field_item(config,situation)?;
    let actor=battle.current_actor().unwrap();
    let absolute=|r:CrtRng|CrtRng {state:r.state,draws:entry.draws+r.draws};
    let at_command_menu=absolute(battle.state().rng);
    let mut report=battle.step_player(Command::Item {id:"shadow_soulstone_m".into(),target:None})?;
    ensure(battle.state().outcome==Outcome::Victory,"Shadow Soulstone (M) did not clear the predicted formation")?;
    report.rng_before=absolute(report.rng_before);report.rng_after=absolute(report.rng_after);
    let mut trace:Vec<_>=battle.state().opening_crt.as_ref().unwrap().trace.as_ref().unwrap().iter().map(|s|RngSpan {reason:s.reason,actor:s.actor,before:absolute(s.before),after:absolute(s.after)}).collect();
    let rewards:BTreeMap<String,FieldReward>=serde_json::from_str(include_str!("../data/field_rare_rewards.json"))?;
    let mut after=absolute(battle.state().rng);
    let (mut exp,mut jp,mut money)=(0,0,0);let mut drops=BTreeMap::new();
    for (i,id) in enemies.iter().enumerate() {
        let r=&rewards[*id];exp+=r.exp;jp+=r.jp;money+=r.money;
        if r.drop_item!="None" {
            let before=after;
            if after.weighted_bool(r.drop_rate as f32*0.01) {*drops.entry(r.drop_item.clone()).or_insert(0)+=1;}
            trace.push(RngSpan {reason:"enemy_drop",actor:Some(ActorId(2+i)),before,after});
        }
    }
    let party_resources_after=battle.state().actors[..2].iter().map(|a|a.resources.clone()).collect();
    Ok(Some(FieldRareVictory {enemy_knowledge:battle.enemy_knowledge(),at_command_menu,after,actor:actor.0,report,trace,base_exp:exp,base_jp:jp,base_money:money,drops,party_resources_after}))
}
/// All-party A Step Ahead opening, after field selector party timing.
/// RNG in the returned Battle is relative to entry.state.
pub fn carried_step_ahead(entry:CrtRng,situation:Situation,players:&[crate::PlayerCharacter],
    enemies:&[&str],night:bool,history:&crate::knowledge::EnemyHistory)->Result<Battle> {
    ensure(situation!=Situation::Surprise,"Route prohibits surprise")?;
    let mut config=break_data().party_config.clone();config.players=players.to_vec();
    for p in &mut config.players {p.resources.as_mut().ok_or_else(||SimError("Missing carried resources".into()))?.bp=1;p.skills.clear();}
    config.seed=entry.state;config.night_buff_turns=if night&&players.iter().any(|p|p.id=="throne") {3}else{0};
    config.enemies=enemies.iter().map(|id|break_data().enemies.get(*id).cloned().ok_or_else(||SimError(format!("Missing Step Ahead enemy {id}")))).collect::<Result<_>>()?;
    config.enemy_knowledge=history.clone();
    config.skills.extend(serde_json::from_str::<Vec<crate::Skill>>(include_str!("../data/gravell_skills.json"))?);
    for p in &mut config.players {
        if p.id=="throne" {p.skills.push("springy_boots".into());}
        if p.id=="agnea" {p.skills.extend(["stimulate".into(),"bewildering_grace".into()]);}
        if matches!(p.id.as_str(),"hikari"|"partitio") {p.skills.extend(["hired_help_guards".into(),"donate_bp".into()]);}
    }
    config.inventory.insert("energizing_pomegranate_m".into(),1);
    config.inventory.insert("thunder_soulstone_l".into(),1);
    let voices=players.iter().map(|p|carried_data().players.get(&p.id).map(|d|d.voice).ok_or_else(||SimError("Unsupported carried identity".into()))).collect::<Result<_>>()?;
    Battle::new_carried_step_ahead(config,situation,voices)
}

/// Bounded first-menu item clear with the actual ordered party. Night talent is
/// applied by the shared initializer. No enemy action or surviving pack is allowed.
pub fn carried_shadow_soulstone_m(entry:CrtRng,situation:Situation,players:&[crate::PlayerCharacter],
    levels:&[u32],enemies:&[&str],night:bool,history:&crate::knowledge::EnemyHistory,inputs:&[u8])->Result<Option<FieldRareVictory>> {
    carried_soulstone(entry,situation,players,levels,enemies,night,history,inputs,"shadow_soulstone_m")
}
pub fn carried_soulstone(entry:CrtRng,situation:Situation,players:&[crate::PlayerCharacter],
    levels:&[u32],enemies:&[&str],night:bool,history:&crate::knowledge::EnemyHistory,inputs:&[u8],item:&str)->Result<Option<FieldRareVictory>> {
    carried_soulstone_with_step_ahead(entry,situation,players,levels,enemies,night,history,inputs,item,false)
}
pub fn carried_soulstone_with_step_ahead(entry:CrtRng,situation:Situation,players:&[crate::PlayerCharacter],
    levels:&[u32],enemies:&[&str],night:bool,history:&crate::knowledge::EnemyHistory,inputs:&[u8],item:&str,step_ahead:bool)->Result<Option<FieldRareVictory>> {
    let (amount,element)=match item {"shadow_soulstone_m"=>(1100,combat::Element::Dark),"wind_soulstone_l"=>(2700,combat::Element::Wind),_=>return Err(SimError("Unsupported carried soulstone".into()))};
    if situation==Situation::Surprise {return Ok(None);}
    // Validate identities, resources and the ordinary first-menu assumptions.
    let menu=if step_ahead {FieldEscapeMenu::with_step_ahead(entry,situation,players,levels,enemies,history)?}else{FieldEscapeMenu::with_players(entry,situation,players,levels,enemies,history)?};
    let mut config=break_data().party_config.clone();
    config.players=players.to_vec();
    for p in &mut config.players {p.resources.as_mut().unwrap().bp=1;p.skills.clear();}
    config.seed=entry.state;config.night_buff_turns=if night&&players.iter().any(|p|p.id=="throne") {3}else{0};
    config.enemies=enemies.iter().map(|id|break_data().enemies.get(*id).cloned().ok_or_else(||SimError(format!("Missing carried item enemy {id}")))).collect::<Result<_>>()?;
    if config.enemies.iter().any(|e| {let c=e.combat.as_ref().unwrap();e.stats.max_hp>combat::elemental_fixed_damage(amount,c.element_weaknesses.contains(&element),false,c.use_cat_damage_type)}) {return Ok(None);}
    config.inventory.insert(item.into(),1);config.enemy_knowledge=history.clone();
    let voices=players.iter().map(|p|carried_data().players[&p.id].voice).collect();
    let mut battle=if step_ahead {Battle::new_carried_step_ahead(config,situation,voices)?}else{Battle::new_carried_field_item(config,situation,voices)?};
    let actor=battle.current_actor().unwrap();
    let absolute=|r:CrtRng|CrtRng {state:r.state,draws:entry.draws+r.draws};
    let at_command_menu=absolute(battle.state().rng);
    ensure(at_command_menu==menu.rng&&actor.0==menu.first_actor,"Carried item/Escape initialization disagrees")?;
    let mut report=battle.step_player_with_boost_inputs(Command::Item {id:item.into(),target:None},inputs)?;
    ensure(battle.state().outcome==Outcome::Victory,"Soulstone failed to clear carried formation")?;
    report.rng_before=absolute(report.rng_before);report.rng_after=absolute(report.rng_after);
    let mut trace:Vec<_>=battle.state().opening_crt.as_ref().unwrap().trace.as_ref().unwrap().iter().map(|s|RngSpan {reason:s.reason,actor:s.actor,before:absolute(s.before),after:absolute(s.after)}).collect();
    static REWARDS:OnceLock<BTreeMap<String,FieldReward>>=OnceLock::new();
    let rewards=REWARDS.get_or_init(||serde_json::from_str(include_str!("../data/field_rare_rewards.json")).unwrap());
    let mut after=absolute(battle.state().rng);let (mut exp,mut jp,mut money)=(0,0,0);let mut drops=BTreeMap::new();
    for (i,id) in enemies.iter().enumerate() {
        let r=&rewards[*id];exp+=r.exp;jp+=r.jp;money+=r.money;
        if r.drop_item!="None" {let before=after;if after.weighted_bool(r.drop_rate as f32*0.01) {*drops.entry(r.drop_item.clone()).or_insert(0)+=1;}
            trace.push(RngSpan {reason:"enemy_drop",actor:Some(ActorId(players.len()+i)),before,after});}
    }
    let party_resources_after=battle.state().actors[..players.len()].iter().map(|a|a.resources.clone()).collect();
    Ok(Some(FieldRareVictory {enemy_knowledge:battle.enemy_knowledge(),at_command_menu,after,actor:actor.0,report,trace,base_exp:exp,base_jp:jp,base_money:money,drops,party_resources_after}))
}

#[cfg(test)]
mod carried_item_tests {
    use super::*;
    #[test]
    fn coast_item_uses_actual_roster_and_matches_escape_menu_at_night() {
        let mut players=vec![route_player(7,1591).unwrap(),route_player(1,0).unwrap(),route_player(8,61).unwrap(),route_player(4,61).unwrap()];
        let ids=["ENE_NML_FLD_SEA_130","ENE_NML_OTR_140","ENE_NML_FLD_SEA_130","ENE_NML_FLD_SEA_130"];
        let mut speakers=std::collections::BTreeSet::new();
        for seed in 0..48 {
            // Rotate actual identities as well as covering natural order RNG.
            players.rotate_left(1);
            let levels:Vec<_>=players.iter().map(|p|match p.id.as_str(){"throne"=>15,"hikari"=>1,_=>5}).collect();
            let before=serde_json::to_value(&players).unwrap();
            let mut rng=CrtRng::new(seed);rng.advance(100);
            let v=carried_shadow_soulstone_m(rng,Situation::Normal,&players,&levels,&ids,true,&Default::default(),&[1,0,1,0]).unwrap().unwrap();
            speakers.insert(players[v.actor].id.clone());
            assert_eq!(v.party_resources_after.len(),4);
            assert_eq!(serde_json::to_value(&players).unwrap(),before);
            assert!(v.base_exp>1000);
            assert_eq!(v.trace.iter().filter(|t|t.reason=="damage_notification_timer").count(),4);
            assert_eq!(v.trace.iter().filter(|t|t.reason=="kill_voice").count(),1);
            assert!(!v.trace.iter().any(|t|t.reason=="study_foe_weakness"));
            assert_eq!(v.trace.first().unwrap().before,rng);
            for pair in v.trace.windows(2) {assert_eq!(pair[0].after,pair[1].before);}
            assert_eq!(v.trace.last().unwrap().after,v.after);
        }
        assert!(speakers.len()>=2);
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FieldEscapeMenu {
    pub rng: CrtRng,
    pub enemy_knowledge: crate::knowledge::EnemyHistory,
    pub level: u32,
    pub trace: Vec<RngSpan>,
    situation: Situation,
    used_voices: Vec<u64>,
    #[serde(skip)]
    step_ahead: bool,
    #[serde(skip)]
    step_battle: Option<Battle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    voice_indices: Option<Vec<usize>>,
    pub first_actor: usize,
    pub party_levels: Vec<u32>,
    pub current_order: Vec<usize>,
    pub next_order: Vec<usize>,
}
#[derive(Debug, Clone, Serialize)]
pub struct FieldEscapeAttempt {
    pub party_sp_cost: Vec<u32>,
    /// Additive latent from successful field breaks; no CRT calls.
    pub party_latent_gain: Vec<u16>,
    pub enemy_knowledge: crate::knowledge::EnemyHistory,
    pub success: bool,
    pub chance_percent: u32,
    pub before_check: CrtRng,
    pub after: CrtRng,
    pub trace: Vec<RngSpan>,
}

#[derive(Debug, Deserialize)]
struct BreakData {
    level: u32,
    config: BattleConfig,
    party_config: BattleConfig,
    enemies: BTreeMap<String, EnemyDefinition>,
}
fn break_data() -> &'static BreakData {
    static DATA: OnceLock<BreakData> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../data/field_break.json")).expect("field break import")
    })
}
/// Original equipment plus the actual character-specific level growth.
pub fn player_latent_max(id:&str)->Result<u16> {
    carried_data().players.get(id).map(|p|p.latent_max).ok_or_else(||SimError(format!("Unknown latent maximum: {id}")))
}
pub fn route_player(character:u32,experience:u32)->Result<crate::PlayerCharacter> {
    let (mut player,base)=match character {7=>(break_data().party_config.players[0].clone(),5),6=>(break_data().party_config.players[1].clone(),1),8|4|1|3=>(crate::progression::starter(character)?,1),_=>return Err(SimError("Unsupported route character".into()))};
    crate::progression::apply_growth(&mut player,character,base,crate::progression::growth(character,experience)?.level)?;
    player.resources=Some(crate::Resources {hp:player.stats.max_hp,sp:player.stats.max_sp,bp:1,latent:0});
    Ok(player)
}
#[derive(Debug, Serialize)]
pub struct FieldBreakEscape {
    pub defends: Vec<FieldSafeDefend>,
    pub escape_menu_ordinal: usize,
    pub skill: Option<String>,
    pub enemy_knowledge: crate::knowledge::EnemyHistory,
    pub attack_actor: usize,
    pub escape_actor: usize,
    pub weapon: combat::Weapon,
    pub boost: u8,
    pub report: StepReport,
    pub at_attack_menu: CrtRng,
    pub at_escape_menu: CrtRng,
    pub escape_round: u32,
    pub attack_inputs: Vec<u8>,
    pub escape_inputs: Vec<u8>,
    pub available_escape_bp: u8,
    pub attempt: FieldEscapeAttempt,
}

/// Snapshot at the real second command menu after a successful opening break.
/// Every boost trial clones this state, including voice history and BP recovery.
#[derive(Debug, Clone)]
pub struct FieldBreakMenu {
    defends: Vec<FieldSafeDefend>,
    skill: Option<String>,
    battle: Battle,
    entry: CrtRng,
    weapon: combat::Weapon,
    boost: u8,
    report: StepReport,
    at_attack_menu: CrtRng,
    attack_inputs: Vec<u8>,
    attack_actor: usize,
}
/// A command-menu prefix before committing Escape. Reuse only when the next
/// candidate has these exact inputs; the snapshot owns BP/voice/weakness state.
#[derive(Debug, Clone)]
pub struct FieldBreakBoostCursor {
    menu: FieldBreakMenu,
    at_escape_menu: CrtRng,
    inputs: Vec<u8>,
}
impl FieldBreakBoostCursor {
    pub fn inputs(&self) -> &[u8] { &self.inputs }
    pub fn extend(&mut self, levels:&[u8]) -> Result<()> {
        let maximum=self.menu.available_bp().min(3);
        let mut previous=self.inputs.last().copied().unwrap_or(0);
        for &bp in levels {
            ensure(bp<=maximum && previous.abs_diff(bp)==1,"boost inputs must change one level within available BP")?;
            previous=bp;
        }
        previous=self.inputs.last().copied().unwrap_or(0);
        for &bp in levels {
            self.menu.battle.advance_boost_input(previous,bp)?;
            self.inputs.push(bp);previous=bp;
        }
        Ok(())
    }
    pub fn attempt(self) -> Result<FieldBreakEscape> {
        ensure(self.inputs.last().copied().unwrap_or(0)==0,"Return to zero BP before Escape")?;
        self.menu.finish_attempt(self.at_escape_menu,self.inputs)
    }
}
#[derive(Debug,Clone,Serialize)]
pub struct FieldSafeDefend {pub actor:usize,pub round:u32,pub menu_ordinal:usize,pub before:CrtRng,pub after:CrtRng}
impl FieldBreakMenu {
    fn menu_ordinal(&self)->usize {
        self.battle.state().turn_order().iter().take_while(|id|Some(**id)!=self.battle.current_actor()).filter(|id|self.battle.side(**id)==Some(crate::Side::Player)).count()+1
    }
    /// Keep only Defend transitions which still allow a guaranteed broken-enemy Escape.
    /// step_player executes one player action, never an enemy action.
    pub fn round(&self)->u32 {self.battle.state().round}
    pub fn safe_defend(mut self)->Result<Option<Self>> {
        let safe=|b:&Battle| b.current_actor().is_some_and(|id|b.side(id)==Some(crate::Side::Player))
            && b.state().actors.iter().enumerate().filter(|(i,a)|b.side(ActorId(*i))==Some(crate::Side::Enemy)&&a.alive()).all(|(_,a)|a.broken());
        if !safe(&self.battle) {return Ok(None);}
        let actor=self.battle.current_actor().unwrap().0;let round=self.battle.state().round;
        let menu_ordinal=self.menu_ordinal();let before=self.rng();
        self.battle.step_player(Command::Defend)?;
        if !safe(&self.battle) {return Ok(None);}
        self.defends.push(FieldSafeDefend {actor,round,menu_ordinal,before,after:self.rng()});
        Ok(Some(self))
    }
    pub fn available_bp(&self) -> u8 { self.battle.state().actors[self.battle.current_actor().unwrap().0].resources.bp }
    pub fn rng(&self) -> CrtRng {
        let r=self.battle.state().rng; CrtRng {state:r.state,draws:self.entry.draws+r.draws}
    }
    pub fn boost_cursor(self) -> FieldBreakBoostCursor {
        FieldBreakBoostCursor {at_escape_menu:self.rng(),menu:self,inputs:vec![]}
    }
    pub fn attempt(self, inputs:&[u8]) -> Result<FieldBreakEscape> {
        let mut cursor=self.boost_cursor();cursor.extend(inputs)?;cursor.attempt()
    }
    fn finish_attempt(mut self,at_escape_menu:CrtRng,inputs:Vec<u8>) -> Result<FieldBreakEscape> {
        let escape_menu_ordinal=self.menu_ordinal();
        let escape_actor=self.battle.current_actor().unwrap().0;
        let escape_round=self.battle.state().round;
        let available_escape_bp=self.available_bp();
        let before_check=self.rng();
        ensure(self.battle.escape_check(None),"Broken-enemy Escape must succeed")?;
        let at=|r:CrtRng| CrtRng {state:r.state,draws:self.entry.draws+r.draws};
        let sp_cost=self.report.events.iter().filter_map(|e|match e {
            crate::Event::ResourceChanged {actor,resource:crate::Resource::Sp,before,after} if actor.0==self.attack_actor=>Some(before.saturating_sub(*after)),_=>None
        }).sum();
        let breaks=self.report.events.iter().filter(|e|matches!(e,crate::Event::Broken {..})).count() as u16;
        let mut report=self.report;
        report.rng_before=at(report.rng_before); report.rng_after=at(report.rng_after);
        let trace=self.battle.state().opening_crt.as_ref().unwrap().trace.as_ref().unwrap()
            .iter().map(|r|RngSpan {reason:r.reason,actor:r.actor,before:at(r.before),after:at(r.after)}).collect();
        Ok(FieldBreakEscape {defends:self.defends,escape_menu_ordinal,skill:self.skill,enemy_knowledge:self.battle.enemy_knowledge(),attack_actor:self.attack_actor,escape_actor,weapon:self.weapon,boost:self.boost,report,at_attack_menu:self.at_attack_menu,
            at_escape_menu,escape_round,attack_inputs:self.attack_inputs,escape_inputs:inputs,available_escape_bp,
            attempt:FieldEscapeAttempt {party_sp_cost:{let mut costs=vec![0;self.attack_actor+1];costs[self.attack_actor]=sp_cost;costs},party_latent_gain: {let mut gain=vec![0;self.attack_actor+1];gain[self.attack_actor]=25*breaks;gain},enemy_knowledge:self.battle.enemy_knowledge(),success:true,chance_percent:100,before_check,after:at(self.battle.state().rng),trace}})
    }
}

pub fn opening_break_menu(entry:CrtRng,situation:Situation,level:u32,enemies:&[&str],
    weapon:combat::Weapon,inputs:&[u8]) -> Result<Option<FieldBreakMenu>> {
    opening_break_menu_with_party(entry,situation,&[level],enemies,weapon,inputs,false)
}

/// Original-gear solo Throné or Throné/Osvald opening. Only a successful break
/// with the next menu belonging to a player is supported; enemy AI is never run.
pub fn opening_break_menu_with_party(entry:CrtRng,situation:Situation,levels:&[u32],enemies:&[&str],
    weapon:combat::Weapon,inputs:&[u8],night:bool) -> Result<Option<FieldBreakMenu>> {
    opening_break_menu_with_history(entry,situation,levels,enemies,weapon,inputs,night,&Default::default())
}
pub fn opening_break_menu_with_history(entry:CrtRng,situation:Situation,levels:&[u32],enemies:&[&str],
    weapon:combat::Weapon,inputs:&[u8],night:bool,history:&crate::knowledge::EnemyHistory) -> Result<Option<FieldBreakMenu>> {
    opening_break_menu_with_players(entry,situation,levels,enemies,weapon,inputs,night,history,None)
}
/// Carried original-gear Throné/Osvald weapon break; no enemy action is skipped.
pub fn opening_break_menu_with_players(entry:CrtRng,situation:Situation,levels:&[u32],enemies:&[&str],
    weapon:combat::Weapon,inputs:&[u8],night:bool,history:&crate::knowledge::EnemyHistory,
    players:Option<&[crate::PlayerCharacter]>) -> Result<Option<FieldBreakMenu>> {
    if let Some(p)=players {
        if p.len()!=levels.len() || p.first().is_none_or(|p|p.id!="throne") || p.len()>2
            || (p.len()==2 && p[1].id!="osvald") {return Ok(None);}
    }
    if enemies.len()!=1 || situation==Situation::Surprise || (players.is_none() && (levels.first()!=Some(&break_data().level)
        || !(levels.len()==1 || levels==[break_data().level,1]))) {return Ok(None);}
    let Some(enemy)=break_data().enemies.get(enemies[0]) else {return Ok(None);};
    let boost=inputs.last().copied().unwrap_or(0);
    let profile=enemy.combat.as_ref().unwrap();
    if boost>1 || !profile.weapon_weaknesses.contains(&weapon) || profile.shields>boost+1 {return Ok(None);}
    let mut config=if levels.len()==2 {break_data().party_config.clone()} else {break_data().config.clone()};
    if let Some(players)=players {
        for (target,carried) in config.players.iter_mut().zip(players) {
            let skills=target.skills.clone();*target=carried.clone();target.skills=skills;
            target.resources.as_mut().unwrap().bp=1;
        }
    }
    config.seed=entry.state; config.enemies=vec![enemy.clone()]; config.night_buff_turns=if night {3} else {0};
    config.enemy_knowledge=history.clone();
    let mut battle=Battle::new_field_break(config,situation)?;
    let actor=battle.current_actor().unwrap();
    if (actor.0==0 && !matches!(weapon,combat::Weapon::Sword|combat::Weapon::Dagger))
        || (actor.0==1 && weapon!=combat::Weapon::Staff) {return Ok(None);}
    let rng=battle.state().rng; let at_attack_menu=CrtRng {state:rng.state,draws:entry.draws+rng.draws};
    let skill=match weapon {combat::Weapon::Dagger=>"attack_dagger",combat::Weapon::Sword=>"attack_sword",combat::Weapon::Staff=>"attack_staff",_=>return Ok(None)};
    let target=ActorId(levels.len());
    let report=battle.step_player_with_boost_inputs(Command::Boost {id:skill.into(),target:Some(target),bp:boost},inputs)?;
    if battle.state().outcome!=Outcome::Ongoing || !battle.state().actors[target.0].broken() {return Ok(None);}
    ensure(battle.current_actor().is_some_and(|a|a.0<levels.len()),"Broken single enemy must yield the next player menu")?;
    Ok(Some(FieldBreakMenu {defends:vec![],skill:None,battle,entry,weapon,boost,report,at_attack_menu,attack_inputs:inputs.to_vec(),attack_actor:actor.0}))
}

/// First actor's known spell must break every surviving enemy; replay its actual BP tier.
/// No enemy action or victory is approximated by this Escape adapter.
pub fn opening_spell_break_menu(entry:CrtRng,situation:Situation,levels:&[u32],enemies:&[&str],
    skill:&str,inputs:&[u8],night:bool,history:&crate::knowledge::EnemyHistory,
    players:&[crate::PlayerCharacter]) -> Result<Option<FieldBreakMenu>> {
    if situation==Situation::Surprise || enemies.is_empty() || players.is_empty() || players.len()>2
        || players[0].id!="throne" || (players.len()==2 && players[1].id!="osvald")
        || inputs.last().copied().unwrap_or(0)>1 {return Ok(None);}
    let boost=inputs.last().copied().unwrap_or(0);
    let (owner,element,aoe)=match skill {
        "ABI_SET_SCH_010"=>(1,combat::Element::Fire,true),
        "ABI_SET_SCH_020"=>(1,combat::Element::Ice,true),
        "ABI_SET_THI_020"=>(0,combat::Element::Dark,false),_=>return Ok(None)
    };
    if owner>=players.len() || (!aoe && enemies.len()!=1) {return Ok(None);}
    let mut definitions=vec![];
    for id in enemies {
        let Some(e)=break_data().enemies.get(*id) else {return Ok(None)};
        let c=e.combat.as_ref().unwrap();
        if c.shields!=1 || !c.element_weaknesses.contains(&element) {return Ok(None);}
        definitions.push(e.clone());
    }
    static MUG:OnceLock<BattleConfig>=OnceLock::new();
    let mug=MUG.get_or_init(||serde_json::from_str(include_str!("../data/cape_cold_mug.json")).unwrap());
    let mut config=if players.len()==1 {break_data().config.clone()}else{break_data().party_config.clone()};
    for (p,carried) in config.players.iter_mut().zip(players) {
        let skills=p.skills.clone();*p=carried.clone();p.skills=skills;p.resources.as_mut().unwrap().bp=1;
    }
    for s in mug.skills.iter().filter(|s|matches!(s.id.as_str(),"ABI_SET_SCH_010"|"ABI_SET_SCH_020")) {
        if !config.skills.iter().any(|old|old.id==s.id) {config.skills.push(s.clone());}
        if config.players.len()>1 {config.players[1].skills.push(s.id.clone());}
    }
    let cost=config.skills.iter().find(|s|s.id==skill).unwrap().sp_cost;
    if config.players[owner].resources.as_ref().unwrap().sp<cost {return Ok(None);}
    config.seed=entry.state;config.enemies=definitions;config.enemy_knowledge=history.clone();config.night_buff_turns=if night {3}else{0};
    let mut battle=Battle::new_field_break(config,situation)?;
    let actor=battle.current_actor().unwrap();if actor.0!=owner {return Ok(None);}
    let at_attack_menu=CrtRng {state:battle.state().rng.state,draws:entry.draws+battle.state().rng.draws};
    let report=battle.step_player_with_boost_inputs(Command::Boost {id:skill.into(),target:if aoe {None}else{Some(ActorId(players.len()))},bp:boost},inputs)?;
    if battle.state().outcome!=Outcome::Ongoing || battle.state().actors[players.len()..].iter().any(|a|a.alive()&&!a.broken()) {return Ok(None);}
    ensure(battle.current_actor().is_some_and(|a|a.0<players.len()),"All broken enemies must yield a player menu")?;
    let _=levels;
    Ok(Some(FieldBreakMenu {defends:vec![],skill:Some(skill.into()),battle,entry,weapon:combat::Weapon::Staff,boost,report,at_attack_menu,attack_inputs:inputs.to_vec(),attack_actor:actor.0}))
}

/// Test an opening normal attack against a lone enemy, using the real hit rolls.
/// A failed break is discarded; no enemy AI or failed Escape is simulated.
/// Prefer fewer BP inputs, then the starting dagger over switching to sword.
pub fn first_action_break(
    entry: CrtRng,
    situation: Situation,
    level: u32,
    enemies: &[&str],
) -> Result<Option<FieldBreakEscape>> {
    for boost in 0..=1 {
        for weapon in [combat::Weapon::Dagger,combat::Weapon::Sword] {
            let inputs=if boost==0 {vec![]} else {vec![1]};
            if let Some(menu)=opening_break_menu(entry,situation,level,enemies,weapon,&inputs)? {
                return menu.attempt(&[]).map(Some);
            }
        }
    }
    Ok(None)
}
impl FieldEscapeMenu {
    pub fn new(entry: CrtRng, situation: Situation, level: u32, enemies: &[&str]) -> Result<Self> {
        Self::initialize(entry, situation, &[level], enemies, &Default::default())
    }
    /// Original-gear healthy Throné/Osvald; no discovered regional weaknesses.
    /// Field selection already consumed the party timing draws.
    pub fn new_party(entry: CrtRng, situation: Situation, levels: [u32; 2], enemies: &[&str]) -> Result<Self> {
        Self::initialize(entry, situation, &levels, enemies, &Default::default())
    }
    pub fn with_history(entry:CrtRng,situation:Situation,levels:&[u32],enemies:&[&str],history:&crate::knowledge::EnemyHistory)->Result<Self> {
        Self::initialize(entry,situation,levels,enemies,history)
    }
    /// Actual ordered party at an ordinary daytime first menu. Entry follows
    /// the field selector's player timing draws. HP/SP/latent and stat changes
    /// are carried; initial BP is source DEFAULT_BP_VALUE.Init (one). No BP
    /// start supports or persistent combat statuses are modeled. No action,
    /// enemy AI, or failed-Escape continuation is exposed by this constructor.
    /// Project the first extra-turn menu using the shared A Step Ahead initializer.
    pub fn with_step_ahead(entry:CrtRng,situation:Situation,players:&[crate::PlayerCharacter],
        levels:&[u32],enemies:&[&str],history:&crate::knowledge::EnemyHistory)->Result<Self> {
        let mut menu=Self::with_players(entry,situation,players,levels,enemies,history)?;
        let battle=carried_step_ahead(entry,situation,players,enemies,false,history)?;
        let state=battle.state();let crt=state.opening_crt.as_ref().unwrap();
        menu.rng=CrtRng {state:state.rng.state,draws:entry.draws+state.rng.draws};
        menu.first_actor=battle.current_actor().unwrap().0;
        menu.current_order=state.turn_order().iter().map(|id|id.0).collect();
        menu.next_order=state.next_queue.iter().map(|id|id.0).collect();
        menu.step_ahead=true;menu.used_voices=crt.used_voices.clone();menu.enemy_knowledge=battle.enemy_knowledge();
        menu.trace=crt.trace.clone().unwrap_or_default();
        for span in &mut menu.trace {span.before.draws+=entry.draws;span.after.draws+=entry.draws;}
        menu.step_battle=Some(battle);
        Ok(menu)
    }
    /// Defend only inside the all-player bonus turn. Never run an enemy action.
    pub fn safe_step_ahead_defend(&self)->Result<Option<(Self,crate::StepReport)>> {
        let Some(mut battle)=self.step_battle.clone() else {return Ok(None)};
        if battle.state().round!=1 || battle.state().turn_order().len()<2 {return Ok(None);}
        let origin=self.rng.draws-battle.state().rng.draws;
        let mut report=battle.step_player(Command::Defend)?;
        if battle.state().round!=1 || battle.current_actor().is_none_or(|id|battle.side(id)!=Some(crate::Side::Player)) {return Ok(None);}
        let mut menu=self.clone();let state=battle.state();let crt=state.opening_crt.as_ref().unwrap();
        menu.rng=CrtRng {state:state.rng.state,draws:origin+state.rng.draws};
        menu.first_actor=battle.current_actor().unwrap().0;
        menu.current_order=state.turn_order().iter().map(|id|id.0).collect();
        menu.next_order=state.next_queue.iter().map(|id|id.0).collect();
        menu.used_voices=crt.used_voices.clone();menu.trace=crt.trace.clone().unwrap_or_default();
        for s in &mut menu.trace {s.before.draws+=origin;s.after.draws+=origin;}
        report.rng_before.draws+=origin;report.rng_after.draws+=origin;
        menu.step_battle=Some(battle);Ok(Some((menu,report)))
    }
    pub fn with_players(entry:CrtRng,situation:Situation,players:&[crate::PlayerCharacter],
        levels:&[u32],enemies:&[&str],history:&crate::knowledge::EnemyHistory)->Result<Self> {
        ensure(!players.is_empty()&&players.len()<=4&&players.len()==levels.len(),"Carried field party requires one to four matching levels")?;
        ensure(levels.iter().all(|&l|(1..=99).contains(&l)),"Invalid carried field level")?;
        ensure(situation!=Situation::Surprise,"Field Escape requires a non-surprise opening")?;
        ensure(!enemies.is_empty()&&enemies.len()<=6,"Unsupported carried field formation")?;
        let mut identities=std::collections::BTreeSet::new();
        let mut speeds=Vec::new();let mut voice_indices=Vec::new();let mut pinch=Vec::new();
        for p in players {
            ensure(identities.insert(&p.id),"Duplicate carried field character")?;
            let imported=carried_data().players.get(&p.id).ok_or_else(||SimError(format!("Unsupported carried field character {}",p.id)))?;
            p.stats.validate()?;ensure(p.stats.speed<=999,"Carried speed exceeds source cap")?;
            let r=p.resources.as_ref().ok_or_else(||SimError("Carried field resources are required".into()))?;
            r.validate_with_latent_max(&p.stats,imported.latent_max)?;
            ensure(r.hp>0,"Carried first-menu projection requires living party members")?;
            let c=p.combat.as_ref().ok_or_else(||SimError("Carried field combat stats are required".into()))?;
            c.validate()?;ensure(c.statuses.is_empty(),"Carried combat statuses are outside first-menu field support")?;
            speeds.push(p.stats.speed);voice_indices.push(imported.voice);
            pinch.push(u64::from(r.hp)*4<=u64::from(p.stats.max_hp));
        }
        ensure(carried_data().initial_bp==1,"Unexpected imported initial field BP")?;
        for id in enemies {speeds.push(data().enemies.get(*id).ok_or_else(||SimError(format!("Unsupported first-Escape enemy {id}")))?.speed);}
        let menu=Self {rng:entry,enemy_knowledge:history.clone(),level:levels[0],trace:vec![],situation,
            step_battle:None,step_ahead:false,used_voices:vec![0;players.len()],voice_indices:Some(voice_indices),first_actor:0,party_levels:levels.to_vec(),
            current_order:vec![],next_order:vec![]};
        menu.finish_initialize(speeds,enemies,players.iter().any(|p|p.id=="osvald"),&pinch)
    }
    fn initialize(entry: CrtRng, situation: Situation, levels: &[u32], enemies: &[&str], history:&crate::knowledge::EnemyHistory) -> Result<Self> {
        ensure(!levels.is_empty() && levels.len()<=3,"Field Escape supports Throné, then Osvald, then Agnea")?;
        let level = levels[0];
        ensure(
            situation != Situation::Surprise,
            "Highroad Escape requires the non-surprise opening target",
        )?;
        ensure(
            !enemies.is_empty() && enemies.len() <= 4,
            "Unsupported highroad formation",
        )?;
        let player = data()
            .levels
            .iter()
            .find(|r| r.level == level)
            .ok_or_else(|| SimError("Missing player level".into()))?;
        let mut speeds = vec![player.speed];
        if levels.len() >= 2 {
            let osvald = data().osvald_levels.iter().find(|r| r.level == levels[1])
                .ok_or_else(|| SimError("Missing Osvald level".into()))?;
            speeds.push(osvald.speed);
        }
        if levels.len()==3 {
            speeds.push(route_player(8,experience_for_level(levels[2])?)?.stats.speed);
        }
        let players = levels.len();
        for id in enemies {
            speeds.push(
                data()
                    .enemies
                    .get(*id)
                    .ok_or_else(|| SimError(format!("Unsupported first-Escape enemy {id}")))?
                    .speed,
            );
        }
        let menu = Self {
            rng: entry,
            enemy_knowledge:history.clone(),
            level,
            trace: vec![],
            situation,
            step_battle:None,step_ahead:false,used_voices: vec![0; players],
            voice_indices: None,
            first_actor: 0,
            party_levels: levels.to_vec(),
            current_order: vec![], next_order: vec![],
        };
        // Legacy constructors describe a healthy route restored by level-up.
        ensure(player.max_hp > 0, "Invalid player HP")?;
        menu.finish_initialize(speeds,enemies,players>=2,&vec![false;players])
    }
    fn finish_initialize(mut self,speeds:Vec<u32>,enemies:&[&str],study_foe:bool,pinch:&[bool])->Result<Self> {
        let players=self.party_levels.len();let situation=self.situation;
        let menu=&mut self;
        for (slot, id) in enemies.iter().enumerate() {
            let before = menu.rng;
            let enemy = &data().enemies[*id];
            opening_crt::initialize_enemy_rng(&mut menu.rng, enemy.bp_base, enemy.initialization);
            menu.record("actor_initialization", Some(ActorId(slot + players)), before);
        }
        let before = menu.rng;
        let mut rows: Vec<_> = (0..speeds.len())
            .filter(|&i| situation != Situation::Preemptive || i < players)
            .collect();
        combat::sort_order(&mut rows, |i| speeds[i]);
        combat::shuffle_order(&mut rows, |_| true, &mut menu.rng);
        menu.record("current_order_shuffle", None, before);
        let first = rows.iter().position(|&i| i < players).unwrap();
        rows[..=first].rotate_right(1);
        menu.first_actor = rows[0];
        menu.current_order = rows;
        let before = menu.rng;
        let speaker = menu.rng.index(players as u32) as usize;
        menu.record("battle_start_speaker", Some(ActorId(speaker)), before);
        menu.voice_for(speaker, 1, VoiceCondition::default(), "battle_start_voice");
        if study_foe {
            let mut seen=std::collections::BTreeSet::new();
            for (slot,id) in enemies.iter().enumerate() {
                if !seen.insert(*id) {continue;}
                let profile=break_data().enemies[*id].combat.as_ref().unwrap();
                let spans=crate::knowledge::study(profile,menu.enemy_knowledge.entry((*id).into()).or_default(),&mut menu.rng);
                for (before,after) in spans {
                    menu.trace.push(RngSpan {reason:"study_foe_weakness",actor:Some(ActorId(slot+players)),before,after});
                }
            }
        }
        let before = menu.rng;
        let mut rows: Vec<_> = (0..speeds.len()).collect();
        combat::sort_order(&mut rows, |i| speeds[i]);
        combat::shuffle_order(&mut rows, |_| true, &mut menu.rng);
        menu.record("next_order_shuffle", None, before);
        menu.next_order = rows;
        menu.voice(3, VoiceCondition {pinch:pinch[menu.first_actor],..Default::default()}, "order_voice");
        Ok(self)
    }
    fn record(&mut self, reason: &'static str, actor: Option<ActorId>, before: CrtRng) {
        self.trace.push(RngSpan {
            reason,
            actor,
            before,
            after: self.rng,
        });
    }
    fn voice(&mut self, kind: u8, condition: VoiceCondition, reason: &'static str) {
        self.voice_for(self.first_actor, kind, condition, reason);
    }
    fn voice_for(&mut self, actor: usize, kind: u8, condition: VoiceCondition, reason: &'static str) {
        let before = self.rng;
        let (data,voice_index)=if let Some(indices)=&self.voice_indices {(if self.step_ahead {opening_crt::gravell_data()} else {opening_crt::field_carried_data()},indices[actor])}
            else {(if self.party_levels.len() >= 2 { opening_crt::field_party_data() } else { opening_crt::mansion_data() },actor)};
        opening_crt::select_voice_from(
            data,
            voice_index,
            kind,
            condition,
            &mut self.used_voices[actor],
            &mut self.rng,
        );
        self.record(reason, Some(ActorId(actor)), before);
    }
    /// Advance one complete boost/cancel cycle without attempting Escape.
    /// Search cursors retain both the selected-voice history and the full trace;
    /// clone this menu before checking Escape so a rejected roll cannot leak.
    pub fn boost_cancel_cycle(&mut self) {
        self.apply_boost_inputs(&[1, 0]);
    }
    fn apply_boost_inputs(&mut self, inputs: &[u8]) {
        let mut previous = 0;
        for &bp in inputs {
            if bp > previous {
                self.voice(
                    8,
                    VoiceCondition {
                        border: bp.into(),
                        ..Default::default()
                    },
                    "boost_voice",
                );
            } else {
                self.record("boost_reduce", Some(ActorId(self.first_actor)), self.rng);
            }
            previous = bp;
        }
    }
    pub fn attempt(mut self, inputs: &[u8], danger: u32) -> Result<FieldEscapeAttempt> {
        let mut previous = 0;
        for &bp in inputs {
            ensure(bp <= 1 && bp.abs_diff(previous) == 1, "First highroad menu has 1 BP")?;
            previous = bp;
        }
        ensure(previous == 0, "Return to zero BP before Escape")?;
        self.apply_boost_inputs(inputs);
        let before_check = self.rng;
        let guaranteed = self.situation == Situation::Preemptive;
        let chance_percent = if guaranteed {
            100
        } else {
            success_percent(&self.party_levels, danger, 0, 0)?
        };
        let success = guaranteed || self.rng.weighted_bool(chance_percent as f32 * 0.01);
        self.record("escape_check", Some(ActorId(self.first_actor)), before_check);
        Ok(FieldEscapeAttempt {
            party_sp_cost:vec![],party_latent_gain:vec![],
            enemy_knowledge:self.enemy_knowledge,
            success,
            chance_percent,
            before_check,
            after: self.rng,
            trace: self.trace,
        })
    }
}

#[cfg(test)]
mod three_member_tests {
    use super::*;
    #[test]
    fn agnea_can_take_first_menu_and_boost_with_persistent_study_foe() {
        let enemy=data().enemies.keys().find(|id|id.starts_with("ENE_NML_FLD_FST_")).unwrap();
        let mut covered=[false;3];
        for seed in 0..1024 {
            let menu=FieldEscapeMenu::with_history(CrtRng::new(seed),Situation::Normal,&[15,15,1],&[enemy.as_str()],&Default::default()).unwrap();
            covered[menu.first_actor]=true;
            assert_eq!(menu.current_order[0],menu.first_actor);
            assert_eq!(menu.used_voices.len(),3);
            assert!(!menu.enemy_knowledge.is_empty());
            let actor=menu.first_actor;
            let attempt=menu.attempt(&[1,0],6).unwrap();
            assert!(attempt.trace.iter().any(|s|s.reason=="boost_voice" && s.actor==Some(ActorId(actor))));
            if covered==[true;3] {return;}
        }
        panic!("Three-member corpus must cover every first-menu actor: {covered:?}");
    }
}

#[cfg(test)]
mod boost_cursor_tests {
    use super::*;

    #[test]
    fn incremental_first_menu_matches_fresh_replay_including_failed_checks_and_history() {
        let ids=["ENE_NML_FLD_SNW_010"];
        let mut actors=[false;3];
        let mut failures=0;
        for levels in [&[5][..],&[5,1][..],&[15,15,1][..]] {
            for situation in [Situation::Normal,Situation::Preemptive] {
                for seed in 0..24 {
                    let entry=CrtRng {state:seed,draws:6100};
                    let fresh=FieldEscapeMenu::with_history(entry,situation,levels,&ids,&Default::default()).unwrap();
                    for history in [crate::knowledge::EnemyHistory::default(),fresh.enemy_knowledge] {
                        let menu=FieldEscapeMenu::with_history(entry,situation,levels,&ids,&history).unwrap();
                        actors[menu.first_actor]=true;
                        let mut cursor=menu.clone();
                        let mut path=vec![];
                        for _ in 0..32 {
                            let expected=menu.clone().attempt(&path,20).unwrap();
                            let actual=cursor.clone().attempt(&[],20).unwrap();
                            failures+=usize::from(!actual.success);
                            assert_eq!(serde_json::to_value(actual).unwrap(),serde_json::to_value(expected).unwrap());
                            cursor.boost_cancel_cycle();path.extend([1,0]);
                        }
                    }
                }
            }
        }
        assert_eq!(actors,[true;3]);
        assert!(failures>0,"The cursor must survive discarded failing Escape rolls");
    }

    #[test]
    fn second_menu_incremental_edges_match_whole_sequence_and_are_immutable() {
        let ids=["ENE_NML_FLD_SNW_010"];
        let mut actors=[false;2];
        for levels in [&[5][..],&[5,1][..]] {
            for night in [false,true] {
                for seed in 0..24 {
                    let entry=CrtRng {state:seed,draws:6100};
                    let first=FieldEscapeMenu::with_history(entry,Situation::Normal,levels,&ids,&Default::default()).unwrap();
                    let weapon=if first.first_actor==0 {combat::Weapon::Dagger} else {combat::Weapon::Staff};
                    let Some(menu)=opening_break_menu_with_history(entry,Situation::Normal,levels,&ids,weapon,&[],night,&first.enemy_knowledge).unwrap() else {continue;};
                    let available=menu.available_bp().min(3);
                    actors[menu.attack_actor]=true;
                    let mut cursor=menu.clone().boost_cursor();
                    let mut path=vec![];
                    for _ in 0..16 {
                        let suffix:Vec<_>=(1..=available).chain((0..available).rev()).collect();
                        cursor.extend(&suffix).unwrap();path.extend(suffix);
                        let mut replay=menu.clone();
                        replay.battle.boost_sequence(&path).unwrap();
                        let expected=replay.finish_attempt(menu.rng(),path.clone()).unwrap();
                        let actual=cursor.clone().attempt().unwrap();
                        assert_eq!(serde_json::to_value(actual).unwrap(),serde_json::to_value(expected).unwrap());
                        let before=serde_json::to_value(cursor.clone().attempt().unwrap()).unwrap();
                        assert!(cursor.extend(&[1,0,2]).is_err());
                        assert_eq!(before,serde_json::to_value(cursor.clone().attempt().unwrap()).unwrap(),"Invalid extensions must not partially consume RNG");
                    }
                }
            }
        }
        assert_eq!(actors,[true;2]);
    }
}

#[cfg(test)]
mod carried_break_tests {
    use super::*;
    #[test]
    fn carried_break_matches_first_menu_and_real_bp() {
        let exp=[1530,1467];
        let players=[route_player(7,exp[0]).unwrap(),route_player(6,exp[1]).unwrap()];
        let levels=[crate::progression::growth(7,exp[0]).unwrap().level,crate::progression::growth(6,exp[1]).unwrap().level];
        let enemies=["ENE_NML_FLD_WLD_020"];
        let history=Default::default();
        let mut tested=0;
        for seed in 0..64 {
            let entry=CrtRng::new(seed);
            for weapon in [combat::Weapon::Dagger,combat::Weapon::Sword] {
                let Some(b)=opening_break_menu_with_players(entry,Situation::Normal,&levels,&enemies,weapon,&[1],false,&history,Some(&players)).unwrap() else {continue};
                let menu=FieldEscapeMenu::with_players(entry,Situation::Normal,&players,&levels,&enemies,&history).unwrap();
                assert_eq!(b.at_attack_menu,menu.rng);
                assert_eq!(b.attack_actor,menu.first_actor);
                let result=b.attempt(&[]).unwrap();
                assert_eq!(result.attack_actor,0);
                assert!(result.attempt.success);
                assert_eq!(result.attempt.chance_percent,100);
                assert_eq!(result.attempt.party_latent_gain,vec![25]);
                if result.escape_actor==0 {assert_eq!(result.available_escape_bp,0);}
                assert!(result.report.events.iter().any(|e|matches!(e,crate::Event::Broken {..})));
                tested+=1;
            }
        }
        assert!(tested>0);
    }
}

#[cfg(test)]
mod spell_break_tests {
 use super::*;
 #[test]
 fn first_actor_fireball_breaks_two_moles_and_sp_is_required() {
  let mut players=break_data().party_config.players.clone();
  let ids=["ENE_NML_FLD_SNW_010","ENE_NML_FLD_SNW_010"];
  let history=Default::default();let mut found=0;
  for seed in 0..64 {
   let entry=CrtRng::new(seed);
   let first=FieldEscapeMenu::with_players(entry,Situation::Normal,&players,&[5,1],&ids,&history).unwrap();
   let b=opening_spell_break_menu(entry,Situation::Normal,&[5,1],&ids,"ABI_SET_SCH_010",&[],false,&history,&players).unwrap();
   if first.first_actor!=1 {assert!(b.is_none());continue;}
   let b=b.expect("Osvald Fireball must break both surviving moles");
   assert_eq!(b.at_attack_menu,first.rng);
   let mut safe=b.clone();let mut count=0;
   loop {
    let Some(next)=safe.clone().safe_defend().unwrap() else {break};
    assert!(next.rng().draws>safe.rng().draws);
    assert!(next.battle.current_actor().unwrap().0<2);
    assert!(next.battle.state().actors[2..].iter().all(|a|a.broken()));
    safe=next;count+=1;assert!(count<=4);
   }
   assert!(count>0);
   let defended=safe.attempt(&[]).unwrap();
   assert_eq!(defended.defends.len(),count);
   assert!(defended.attempt.success);
   assert_eq!(defended.attempt.party_sp_cost,vec![0,14]);
   let guard_draws:u64=defended.attempt.trace.iter().filter(|r|matches!(r.reason,"guard_disease"|"guard_voice")).map(|r|r.after.draws-r.before.draws).sum();
   assert_eq!(guard_draws,22*count as u64);
   let b=b.attempt(&[]).unwrap();assert_eq!(b.skill.as_deref(),Some("ABI_SET_SCH_010"));
   assert_eq!(b.attempt.party_sp_cost,vec![0,14]);assert_eq!(b.attempt.party_latent_gain,vec![0,50]);
     assert_eq!(b.attempt.trace.iter().filter(|r|r.reason=="ability_voice").map(|r|r.after.draws-r.before.draws).sum::<u64>(),1,"Fireball must select its ability voice");
   assert!(b.attempt.success);assert_eq!(b.attempt.chance_percent,100);
   assert_eq!(b.report.events.iter().filter(|e|matches!(e,crate::Event::Broken {..})).count(),2);
   players[1].resources.as_mut().unwrap().sp=13;
   assert!(opening_spell_break_menu(entry,Situation::Normal,&[5,1],&ids,"ABI_SET_SCH_010",&[],false,&history,&players).unwrap().is_none());
   found+=1;break;
  }
  assert!(found>0);
 }
}

pub(crate) fn transformed_enemy(id:&str)->Result<EnemyDefinition> {break_data().enemies.get(id).cloned().ok_or_else(||SimError(format!("Missing transformed enemy {id}")))}
