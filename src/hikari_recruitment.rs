//! Scripted Hikari recruitment battle, before Hikari joins: regular Ice
//! Soulstone at the first player menu, then an unboosted default-weapon attack.
//! The caller owns event/map loading, the item ledger, EXP growth and recruitment.
//! Grunts necessarily die to the first command; their later AI is unsupported.
use crate::{ActorId, Battle, BattleConfig, Command, CrtRng, Event, Outcome, PlayerCharacter,
    Resource, Resources, Result, Side, StepReport, ensure, knowledge::EnemyHistory,
    opening_crt::RngSpan, progression::Reward};
use serde::Serialize;
use std::{collections::BTreeMap, sync::OnceLock};

pub const GROUP: &str = "ENG_EVE_WAR_C00_010";
#[derive(Debug, Serialize)]
pub struct Menu {
    pub actor: usize, pub round: u32, pub rng: CrtRng,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub boost_inputs: Vec<u8>,
}
#[derive(Debug, Serialize)]
pub struct Prediction {
    pub before: CrtRng,
    pub after: CrtRng,
    /// Original carried characters, including learned skills outside this routine.
    pub players_before: Vec<PlayerCharacter>,
    /// Executable bounded profile; its player skill list contains normal attacks only.
    pub battle_config: BattleConfig,
    pub menus: Vec<Menu>,
    pub steps: Vec<StepReport>,
    pub trace: Vec<RngSpan>,
    pub outcome: Outcome,
    pub party_resources_after: Vec<Resources>,
    pub enemy_knowledge: EnemyHistory,
    pub base_exp: u32,
    pub base_jp: u32,
    pub base_money: u32,
    pub broke_enemy: bool,
    pub overkill: bool,
    pub party_took_damage: bool,
    pub rewards: Reward,
    /// ItemDB source labels; the caller merges these into the world inventory.
    pub drops: BTreeMap<String, u32>,
    pub ice_soulstones_used: u32,
}

fn template() -> &'static BattleConfig {
    static DATA: OnceLock<BattleConfig> = OnceLock::new();
    DATA.get_or_init(|| BattleConfig::from_json(include_str!("../data/hikari_recruitment.json"))
        .expect("imported recruitment battle"))
}

/// `before` is immediately before battle character initialization, after the
/// event's fixed encounter-volume lookup. One regular Ice Soulstone is required.
/// Only [Throné, Osvald, Agnea, Partitio] in that order is currently imported.
/// All four Chapter 1 latent powers must be released, with no latent-gain support.
pub fn predict(before: CrtRng, players: &[PlayerCharacter], history: &EnemyHistory) -> Result<Prediction> {
    predict_with_boost_inputs(before, players, history, &[], &[])
}

/// Replay zero-ending boost inputs at either player menu, before its fixed
/// command. Normal battle initialization sets BP to one; BP-boosting supports
/// are outside this profile. Each trial owns its battle and cannot leak state.
pub fn predict_with_boost_inputs(before: CrtRng, players: &[PlayerCharacter], history: &EnemyHistory,
    first_inputs: &[u8], second_inputs: &[u8]) -> Result<Prediction> {
    ensure(players.iter().map(|p|p.id.as_str()).eq(["throne","osvald","agnea","partitio"]),
        "Recruitment requires Throné, Osvald, Agnea and Partitio in carried order")?;
    let mut config = template().clone();
    config.seed = before.state;
    config.players = players.to_vec();
    config.enemy_knowledge = history.clone();
    for (player, expected) in config.players.iter_mut().zip([crate::combat::Weapon::Dagger,
        crate::combat::Weapon::Staff, crate::combat::Weapon::Dagger, crate::combat::Weapon::Lance]) {
        ensure(player.resources.as_ref().is_some_and(|r|r.hp>0), "Recruitment requires explicit living carried resources")?;
        // BattlePlayerBP.Initialize -> InitBoostPoint(DEFAULT_BP_VALUE.Init).
        player.resources.as_mut().unwrap().bp=1;
        ensure(player.combat.as_ref().is_some_and(|c|c.main_weapon==Some(expected)),
            "Recruitment requires each imported original default weapon")?;
        player.skills = vec![match expected {
            crate::combat::Weapon::Dagger=>"attack_dagger",
            crate::combat::Weapon::Staff=>"attack_staff",
            crate::combat::Weapon::Lance=>"attack_lance", _=>unreachable!()
        }.into()];
    }
    let mut battle = Battle::new_hikari_recruitment(config.clone())?;
    let absolute = |r: CrtRng| CrtRng {state:r.state, draws:before.draws+r.draws};
    let mut menus = Vec::new();
    let mut steps = Vec::new();
    while let Some(actor) = battle.current_actor() {
        ensure(battle.state().round==1 && menus.len()<2, "Recruitment routine did not win within two player commands")?;
        let mut report = if battle.side(actor)==Some(Side::Enemy) {
            ensure(!menus.is_empty() && actor==ActorId(4), "Unexpected enemy turn before Ice Soulstone")?;
            battle.step_enemy()?
        } else {
            let inputs=if menus.is_empty() {first_inputs} else {second_inputs};
            menus.push(Menu {actor:actor.0,round:battle.state().round,rng:absolute(battle.state().rng),boost_inputs:inputs.to_vec()});
            let command = if menus.len()==1 {
                Command::Item {id:"ice_soulstone".into(),target:None}
            } else {
                Command::Skill {id:battle.default_attack(actor)?.into(),target:Some(ActorId(4))}
            };
            let report = battle.step_player_with_boost_inputs(command,inputs)?;
            if menus.len()==1 {
                ensure(battle.state().actors[4].resources.hp==20
                    && battle.state().actors[5..7].iter().all(|a|!a.alive()),
                    "Ice Soulstone did not leave only the 20-HP leader")?;
            }
            report
        };
        report.rng_before=absolute(report.rng_before);report.rng_after=absolute(report.rng_after);
        steps.push(report);
    }
    ensure(battle.state().outcome==Outcome::Victory && menus.len()==2,
        "Recruitment routine failed to defeat the soldiers")?;
    let crt=battle.state().opening_crt.as_ref().unwrap();
    let trace=crt.trace.as_ref().unwrap().iter().map(|s|RngSpan {reason:s.reason,actor:s.actor,
        before:absolute(s.before),after:absolute(s.after)}).collect();
    let drops=crt.rewards.as_ref().unwrap().items.clone();
    let (mut broke_enemy,mut overkill,mut party_took_damage)=(false,false,false);
    let mut damage=BTreeMap::new();
    for event in steps.iter().flat_map(|s|&s.events) {
        match *event {
            Event::Broken {actor,..} if actor.0>=4=>broke_enemy=true,
            Event::Damage {target,amount,..}=>{damage.insert(target,amount);},
            Event::ResourceChanged {actor,resource:Resource::Hp,before,after}=>{
                if actor.0<4 && after<before {party_took_damage=true;}
                if actor.0>=4 && after==0 && before>0 {
                    overkill|=damage.get(&actor).is_some_and(|&n|u64::from(n)>=2*u64::from(before));
                }
            },
            _=>{}
        }
    }
    let base=crate::progression::formation_reward(GROUP)?;
    let rewards=Reward {exp:base.exp+(u32::from(broke_enemy)+u32::from(overkill))*(base.exp/10).max(1),
        jp:base.jp+(base.jp/10).max(1), // completed within the first round
        money:base.money+u32::from(!party_took_damage)*(base.money/10).max(1)};
    Ok(Prediction {before,after:absolute(battle.state().rng),players_before:players.to_vec(),battle_config:config,
        menus,steps,trace,outcome:battle.state().outcome,
        party_resources_after:battle.state().actors[..4].iter().map(|a|a.resources.clone()).collect(),
        enemy_knowledge:battle.enemy_knowledge(),base_exp:base.exp,base_jp:base.jp,base_money:base.money,
        broke_enemy,overkill,party_took_damage,rewards,drops,ice_soulstones_used:1})
}
