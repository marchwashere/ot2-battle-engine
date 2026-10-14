//! The requested A Step Ahead opening, expressed independently of RNG search.
//! Actor IDs come from the actual carried party/order, never assumed slots.
use crate::{ActorId, Battle, Command, Result, ensure};
use serde::Serialize;

/// A route candidate is accepted only after the prescribed routine has finished.
/// An EXP lottery result is not evidence of a winning fight.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag="result", rename_all="snake_case")]
pub enum Evaluation {
    Accepted,
    Loss { reasons: Vec<LossReason> },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag="reason", rename_all="snake_case")]
pub enum LossReason {
    RoutineExhausted,
    Defeat,
    Escaped,
    PartyMemberDead { actor: ActorId },
    MissingExp100,
}

/// `exp_multiplier_percent` is the final battle reward modifier, not whether
/// an EXP100 child was selected earlier. Call only at the routine endpoint.
/// A temporary knockout followed by revival is permissible: rewards depend
/// on the actual living party at victory.
pub fn evaluate_finished_routine(battle:&Battle, exp_multiplier_percent:u32)->Evaluation {
    evaluate_endpoint(battle.state().outcome, battle.state().actors.iter().enumerate()
        .filter(|(i,_)|battle.side(ActorId(*i))==Some(crate::Side::Player))
        .map(|(i,a)|(ActorId(i),a.alive())), exp_multiplier_percent)
}
fn evaluate_endpoint(outcome:crate::Outcome, party:impl IntoIterator<Item=(ActorId,bool)>, exp:u32)->Evaluation {
    let mut reasons=Vec::new();
    match outcome {
        crate::Outcome::Ongoing=>reasons.push(LossReason::RoutineExhausted),
        crate::Outcome::Defeat=>reasons.push(LossReason::Defeat),
        crate::Outcome::Escaped=>reasons.push(LossReason::Escaped),
        crate::Outcome::Victory=>(),
    }
    for (actor,alive) in party {if !alive {reasons.push(LossReason::PartyMemberDead {actor});}}
    if exp!=10_000 {reasons.push(LossReason::MissingExp100);}
    if reasons.is_empty() {Evaluation::Accepted} else {Evaluation::Loss {reasons}}
}

#[derive(Debug, Clone, Serialize)]
pub struct PlannedAction { pub actor: ActorId, pub action: Action }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all="snake_case")]
pub enum Action {
    SpringyBoots { target: ActorId },
    Fireball,
    FireballMax,
    Dagger { target: ActorId },
    Staff { target: ActorId },
    SmallPomegranate { target: ActorId },
    Defend,
    DonateBp { target: ActorId },
    Pomegranate { target: ActorId },
    BewilderingGrace,
    ThunderSoulstone,
    Guards,
}
impl Action {
    pub fn command(self)->Command {
        match self {
            Self::SpringyBoots {target}=>Command::Skill {id:"springy_boots".into(),target:Some(target)},
            Self::Fireball=>Command::Boost {id:"ABI_SET_SCH_010".into(),target:None,bp:1},
            Self::FireballMax=>Command::Boost {id:"ABI_SET_SCH_010".into(),target:None,bp:3},
            Self::Dagger {target}=>Command::Skill {id:"attack_dagger".into(),target:Some(target)},
            Self::Staff {target}=>Command::Skill {id:"attack_staff".into(),target:Some(target)},
            Self::SmallPomegranate {target}=>Command::Item {id:"energizing_pomegranate".into(),target:Some(target)},
            Self::Defend=>Command::Defend,
            Self::DonateBp {target}=>Command::Boost {id:"donate_bp".into(),target:Some(target),bp:1},
            Self::Pomegranate {target}=>Command::Item {id:"energizing_pomegranate_m".into(),target:Some(target)},
            Self::ThunderSoulstone=>Command::Item {id:"thunder_soulstone_l".into(),target:None},
            Self::BewilderingGrace=>Command::Boost {id:"bewildering_grace".into(),target:None,bp:3},
            Self::Guards=>Command::Boost {id:"hired_help_guards".into(),target:None,bp:3},
        }
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct OpeningPlan {
    pub agnea_first: bool,
    pub bonus_turn: Vec<PlannedAction>,
}
impl OpeningPlan {
    pub fn from_battle(battle:&Battle)->Result<Self> {
        ensure(battle.state().round==1&&battle.state().opening_crt.as_ref().is_some_and(|c|c.step_ahead),"Gravell plan requires the A Step Ahead opening")?;
        ensure(battle.current_actor()==battle.state().turn_order().first().copied(),"Build the opening plan before the first action")?;
        let find=|name:&str| battle.actor_id(name).ok_or_else(||crate::SimError(format!("Missing {name}")));
        let throne=find("throne")?;let agnea=find("agnea")?;let partitio=find("partitio")?;
        let merchant=match (battle.actor_id("hikari"),battle.actor_id("osvald")) {
            (Some(id),None)|(None,Some(id))=>id,
            _=>return Err(crate::SimError("Gravell routine needs exactly one of Hikari or Osvald as the second Merchant".into())),
        };
        Self::new(battle.state().turn_order(),throne,agnea,[merchant,partitio])
    }
    pub fn new(order:&[ActorId],throne:ActorId,agnea:ActorId,merchants:[ActorId;2])->Result<Self> {
        let party=[throne,agnea,merchants[0],merchants[1]];
        ensure(order.len()==4&&(0..4).all(|i|!party[..i].contains(&party[i]))&&party.iter().all(|id|order.iter().filter(|x|*x==id).count()==1),"Opening requires each of the four travelers exactly once")?;
        let agnea_first=order[0]==agnea;
        let ordered_merchants:Vec<_>=order.iter().copied().filter(|id|merchants.contains(id)).collect();
        let actions=order.iter().copied().map(|actor|PlannedAction {actor,action:
            if actor==agnea {Action::Defend}
            else if actor==throne {Action::Pomegranate {target:agnea}}
            else if actor==ordered_merchants[0] {Action::DonateBp {target:ordered_merchants[1]}}
            else {Action::Guards}
        }).collect();
        Ok(Self {agnea_first,bonus_turn:actions})
    }
}


#[derive(Debug, Clone, Serialize)]
pub struct MenuAction {
    pub round:u32,pub actor:ActorId,pub action:Action,pub inputs:Vec<u8>,pub before:crate::CrtRng,pub report:crate::StepReport,
}
#[derive(Debug, Clone, Serialize)]
pub struct Fight {
    pub actions:Vec<MenuAction>,pub evaluation:Evaluation,
    pub after:crate::CrtRng,pub state:crate::BattleState,
    pub enemy_ids:Vec<String>,pub enemy_knowledge:crate::knowledge::EnemyHistory,
    pub trace:Vec<crate::opening_crt::RngSpan>,
}
#[derive(Clone)]
struct Runner {battle:Battle,remaining:Vec<PlannedAction>,actions:Vec<MenuAction>,bg:bool,guards:bool,stone:bool}
impl Runner {
    fn new(battle:Battle)->Result<Self> {
        let plan=OpeningPlan::from_battle(&battle)?;
        Ok(Self {remaining:plan.bonus_turn.clone(),battle,actions:vec![],bg:false,guards:false,stone:false})
    }
    fn next(&self)->Option<Action> {
        if self.battle.state().outcome!=crate::Outcome::Ongoing {return None;}
        let actor=self.battle.current_actor()?;
        if self.battle.side(actor)!=Some(crate::Side::Player) {return None;}
        if self.battle.state().round==1 {return self.remaining.iter().find(|s|s.actor==actor).map(|s|s.action);}
        if self.battle.state().round!=2 {return None;}
        if !self.bg {return (self.battle.actor_id("agnea")==Some(actor)).then_some(Action::BewilderingGrace);}
        (!self.stone).then_some(Action::ThunderSoulstone)
    }
    fn record(&mut self,action:Action,inputs:Vec<u8>,report:crate::StepReport,round:u32) {
        self.remaining.retain(|p|p.actor!=report.actor);
        self.bg|=action==Action::BewilderingGrace;self.guards|=action==Action::Guards;self.stone|=action==Action::ThunderSoulstone;
        self.actions.push(MenuAction {round,actor:report.actor,action,inputs,before:report.rng_before,report});
    }
    fn execute(&mut self,action:Action,inputs:&[u8])->Result<bool> {
        let round=self.battle.state().round;
        match if inputs.is_empty() {self.battle.step_player(action.command())}else{self.battle.step_player_with_boost_inputs(action.command(),inputs)} {
            Ok(report)=>{self.record(action,inputs.to_vec(),report,round);Ok(true)},
            Err(e) if ["not enough BP","Not enough BP","not enough SP","Not enough SP","Silence prevents","Items are scattered","item is out of stock"].iter().any(|s|e.0.contains(s))=>Ok(false),
            Err(e)=>Err(e),
        }
    }
    fn advance_to_search(&mut self)->Result<Option<Action>> {
        loop {
            let Some(action)=self.next() else {return Ok(None)};
            if matches!(action,Action::BewilderingGrace|Action::Guards) {return Ok(Some(action));}
            if !self.execute(action,if matches!(action,Action::DonateBp {..}) {&[1]}else{&[]})? {return Ok(None);}
        }
    }
    fn evaluation(&self)->Evaluation {
        evaluate_finished_routine(&self.battle,self.battle.state().grace.as_ref().map_or(100,|g|g.exp_percent()))
    }
    fn finish(self)->Fight {
        let evaluation=self.evaluation();
        let enemy_ids=(0..self.battle.state().actors.len()).map(ActorId).filter(|id|self.battle.side(*id)==Some(crate::Side::Enemy)).map(|id|self.battle.actor_name_id(id).unwrap().to_owned()).collect();
        Fight {actions:self.actions,evaluation,after:self.battle.state().rng,state:self.battle.state().clone(),enemy_ids,
            enemy_knowledge:self.battle.enemy_knowledge(),trace:self.battle.state().opening_crt.as_ref().and_then(|c|c.trace.clone()).unwrap_or_default()}
    }
}
/// Replay the complete prescribed routine. A stopped/blocked command or any
/// surviving enemy is a failed route candidate, never an assumed victory.
pub fn replay(battle:Battle,bg_inputs:&[u8],guard_inputs:&[u8])->Result<Fight> {
    let mut runner=Runner::new(battle)?;
    while let Some(action)=runner.next() {
        let inputs=match action {Action::BewilderingGrace=>bg_inputs,Action::Guards=>guard_inputs,Action::DonateBp {..}=>&[1],_=>&[]};
        if !runner.execute(action,inputs)? {break;}
    }
    Ok(runner.finish())
}
// Reconstruct the selected routine without changing any upstream decision.
fn before_finisher(battle:Battle,selected:&Fight)->Result<Runner> {
    ensure(selected.evaluation==Evaluation::Accepted,"Finisher adjustment requires an accepted EXP100 routine")?;
    ensure(selected.actions.last().is_some_and(|a|a.action==Action::ThunderSoulstone),"Selected routine must end with Thunder Soulstone L")?;
    let mut runner=Runner::new(battle)?;
    for action in &selected.actions[..selected.actions.len()-1] {
        ensure(runner.next()==Some(action.action)&&runner.battle.current_actor()==Some(action.actor),"Selected finisher prefix diverged")?;
        ensure(runner.execute(action.action,&action.inputs)?,"Selected finisher prefix became unavailable")?;
    }
    ensure(runner.next()==Some(Action::ThunderSoulstone),"Selected routine did not reach the finisher menu")?;
    Ok(runner)
}
/// Actual last command menu, with Guards/BG and all earlier inputs frozen.
pub fn finisher_menu(battle:Battle,selected:&Fight)->Result<Battle> {
    Ok(before_finisher(battle,selected)?.battle)
}
/// Replay the frozen prefix and a zero-ending boost path before the item.
pub fn replay_finisher(battle:Battle,selected:&Fight,inputs:&[u8])->Result<Fight> {
    let mut runner=before_finisher(battle,selected)?;
    ensure(runner.execute(Action::ThunderSoulstone,inputs)?,"Finisher item became unavailable")?;
    let fight=runner.finish();
    ensure(fight.evaluation==Evaluation::Accepted,"Adjusted finisher lost EXP100/victory/all-alive acceptance")?;
    Ok(fight)
}
/// Untargeted compatibility search: fix the first finisher-ready Guards roll,
/// then search BG. Targeted route refinement uses the joint search below.
pub fn search(battle:Battle,max_inputs:usize)->Result<Option<Fight>> {
    // Untargeted/unlimited compatibility callers retain strict propagation;
    // enumerating an unbounded Cartesian product is unsuitable here.
    search_target_impl(battle,max_inputs,None,Some(0),None)
}
/// Search Guards/BG combinations in total input-cost order. Target an actually
/// executed BG lottery at any of the four effect positions; child execution
/// determines later lottery positions and early termination.
pub fn search_target(battle:Battle,max_inputs:usize,target:Option<u32>)->Result<Option<Fight>> {
    search_target_impl(battle,max_inputs,target,None,None)
}
/// Fast route refinement: retain a bounded band of Guards alternatives beyond
/// the cheapest finisher-ready roll. Acceptance still replays the entire fight.
/// This is a search-quality bound, never an approximation of battle execution.
pub fn search_target_windowed(battle:Battle,max_inputs:usize,target:u32,guard_extra_inputs:usize,work:Option<&std::cell::Cell<usize>>)->Result<Option<Fight>> {
    search_target_impl(battle,max_inputs,Some(target),Some(guard_extra_inputs),work)
}
fn search_target_impl(battle:Battle,max_inputs:usize,target:Option<u32>,guard_extra_inputs:Option<usize>,work:Option<&std::cell::Cell<usize>>)->Result<Option<Fight>> {
    use crate::fight_search::BoostFrontier;
    let mut probe=battle.clone();probe.lottery_search_trace();
    let mut prefix=Runner::new(probe)?;
    ensure(prefix.advance_to_search()?==Some(Action::Guards),"Routine cannot reach Guards")?;
    let deadline=target.map(|t|battle.state().rng.draws+battle.state().rng.distance_to(t));
    let mut guards=BoostFrontier::new(&prefix.battle)?;
    let mut continuations=Vec::new();
    let mut seen_bg=std::collections::BTreeMap::new();
    let mut first_guard_cost=None;
    // Both actions end at 3 BP, so their combined input cost is even and >=6.
    for cost in (6..=max_inputs).step_by(2) {
        if work.is_some_and(|w|w.get()==0) {return Ok(None);}
        while let Some(candidate)=guards.next_before(&Action::Guards.command(),first_guard_cost.zip(guard_extra_inputs).map_or(cost-3,|(first,extra): (usize,usize)| (first+extra).min(cost-3)),deadline)? {
            if let Some(w)=work {if w.get()==0 {return Ok(None);}w.set(w.get()-1);}
            let mut runner=prefix.clone();let round=runner.battle.state().round;runner.battle=candidate.after_action;
            runner.record(Action::Guards,candidate.inputs.clone(),candidate.report,round);
            if runner.advance_to_search()?!=Some(Action::BewilderingGrace) {continue;}
            if deadline.is_some_and(|d|runner.battle.state().rng.draws>d) {continue;}
            let actor=runner.battle.current_actor().unwrap();
            if runner.battle.state().actors[actor.0].resources.bp<3 {continue;}
            // Preserve the routine's requirement that Guards leaves the normal
            // Thunder finisher sufficient. Full BG validation still follows.
            let mut finisher=runner.battle.clone();
            finisher.step_player(Action::ThunderSoulstone.command())?;
            if finisher.state().outcome!=crate::Outcome::Victory {continue;}
            first_guard_cost.get_or_insert(candidate.inputs.len());
            let dance=BoostFrontier::new(&runner.battle)?;
            continuations.push((runner,candidate.inputs,dance));
        }
        for (runner,guard_inputs,dance) in &mut continuations {
            while let Some(candidate)=if guard_extra_inputs==Some(0) {dance.next_before(&Action::BewilderingGrace.command(),cost-guard_inputs.len(),deadline)?}else{dance.next_unique_before(&Action::BewilderingGrace.command(),cost-guard_inputs.len(),deadline,&mut seen_bg)?} {
                if let Some(w)=work {if w.get()==0 {return Ok(None);}w.set(w.get()-1);}
                let mut after=runner.clone();let round=after.battle.state().round;after.battle=candidate.after_action;
                after.record(Action::BewilderingGrace,candidate.inputs.clone(),candidate.report,round);
                if target.is_some_and(|t|!after.battle.state().opening_crt.as_ref().and_then(|c|c.trace.as_ref()).is_some_and(|trace|trace.iter().any(|s|s.reason=="bewildering_grace_lottery"&&s.before.state==t))) {continue;}
                if after.battle.state().grace.as_ref().is_none_or(|g|g.exp_percent()!=10_000)||after.battle.state().actors[..4].iter().any(|a|!a.alive()) {continue;}
                after.advance_to_search()?;
                if after.evaluation()==Evaluation::Accepted {
                    let replay=replay(battle,&candidate.inputs,guard_inputs)?;
                    ensure(replay.evaluation==Evaluation::Accepted,"Fight search replay diverged")?;
                    return Ok(Some(replay));
                }
            }
        }
    }

    Ok(None)
}


#[derive(Debug,Clone,Serialize)]
pub struct Rewards {
    pub base:crate::progression::Reward,pub awarded:crate::progression::Reward,
    pub drops:std::collections::BTreeMap<String,u32>,pub after:crate::CrtRng,
    pub trace:Vec<crate::opening_crt::RngSpan>,pub hired_help_cost:u32,
}
/// CalcGetReward applies the battle multiplier before CalcBonusReward.
/// A Step Ahead is turn zero in the retail reward counter.
pub fn rewards(fight:&Fight,grows_on_trees:bool)->Result<Rewards> {
    ensure(fight.evaluation==Evaluation::Accepted,"Cannot award an unaccepted fight")?;
    #[derive(serde::Deserialize)] struct Row {exp:u32,jp:u32,money:u32,drop_item:String,drop_rate:u32}
    static DATA:std::sync::OnceLock<std::collections::BTreeMap<String,Row>>=std::sync::OnceLock::new();
    let data=DATA.get_or_init(||serde_json::from_str(include_str!("../data/field_rare_rewards.json")).unwrap());
    let mut base=crate::progression::Reward::default();let mut after=fight.after;let mut drops=std::collections::BTreeMap::new();let mut trace=vec![];
    for (i,id) in fight.enemy_ids.iter().enumerate() {
        let row=data.get(id).ok_or_else(||crate::SimError(format!("Missing Gravell reward {id}")))?;
        base.exp+=row.exp;base.jp+=row.jp;base.money+=row.money;
        if row.drop_item!="None" {let before=after;if after.weighted_bool(row.drop_rate as f32*0.01_f32) {*drops.entry(row.drop_item.clone()).or_insert(0)+=1;}
            trace.push(crate::opening_crt::RngSpan {reason:"enemy_drop",actor:Some(ActorId(4+i)),before,after});}
    }
    let modifiers=fight.state.grace.as_ref().unwrap();
    let multiply=|n:u32,ratio:f32|(n as f32*ratio+0.01_f32) as u32;
    let mut awarded=crate::progression::Reward {exp:multiply(base.exp,modifiers.exp_percent() as f32*0.01_f32),jp:multiply(base.jp,modifiers.jp_percent() as f32*0.01_f32),money:multiply(base.money,if grows_on_trees {1.1_f32}else{1.0})};
    let mut broken=false;let mut overkill=false;let mut damaged=false;let mut last_damage=std::collections::BTreeMap::new();
    for action in &fight.actions {for event in &action.report.events {
        match *event {
            crate::Event::Broken {actor,..} if actor.0>=4=>broken=true,
            crate::Event::Damage {target,amount,..}=>{last_damage.insert(target,amount);if target.0<4&&amount>0 {damaged=true;}},
            crate::Event::ResourceChanged {actor,resource:crate::Resource::Hp,before,after:0} if actor.0>=4&&before>0=>{overkill|=last_damage.get(&actor).is_some_and(|&n|n as u64>=before as u64*2);},
            _=>(),
        }
    }}
    awarded.exp+=(u32::from(broken)+u32::from(overkill))*(awarded.exp/10).max(1);
    if fight.state.round<=2 {awarded.jp+=(awarded.jp/10).max(1);}
    if !damaged {awarded.money+=(awarded.money/10).max(1);}
    awarded.money+=modifiers.money_gained;
    let hired_help_cost=crate::hired_help::guards_cost()*fight.actions.iter().filter(|a|a.action==Action::Guards).count() as u32;
    Ok(Rewards {base,awarded,drops,after,trace,hired_help_cost})
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exp100_never_masks_unfinished_combat_or_a_dead_reward_recipient() {
        use crate::Outcome::*;
        let party=[(ActorId(0),true),(ActorId(1),true),(ActorId(2),true),(ActorId(3),true)];
        assert_eq!(evaluate_endpoint(Victory,party,10_000),Evaluation::Accepted);
        for outcome in [Ongoing,Defeat,Escaped] {
            assert!(matches!(evaluate_endpoint(outcome,party,10_000),Evaluation::Loss {..}));
        }
        for dead in 0..4 {
            let mut p=party;p[dead].1=false;
            assert_eq!(evaluate_endpoint(Victory,p,10_000),Evaluation::Loss {
                reasons:vec![LossReason::PartyMemberDead {actor:ActorId(dead)}]});
        }
        assert!(matches!(evaluate_endpoint(Victory,party,500),Evaluation::Loss {..}));
    }
    #[test]
    fn scattered_items_rejects_the_routine_without_executing_thunder() {
        let party:Vec<_>=[7,1,8,4].map(|id|crate::field_escape::route_player(id,5000).unwrap()).into();
        // Use real BG lotteries, not an injected item-lock flag.
        for seed in 0..256 {
            let battle=crate::field_escape::carried_step_ahead(crate::CrtRng::new(seed),
                crate::escape::Situation::Normal,&party,
                &["ENE_NML_FLD_WLD_250","ENE_NML_FLD_WLD_160","ENE_NML_OTR_150"],
                true,&crate::knowledge::EnemyHistory::new()).unwrap();
            let mut runner=Runner::new(battle).unwrap();
            assert_eq!(runner.advance_to_search().unwrap(),Some(Action::Guards));
            assert!(runner.execute(Action::Guards,&[1,2,3]).unwrap());
            if runner.advance_to_search().unwrap()!=Some(Action::BewilderingGrace) {continue;}
            assert!(runner.execute(Action::BewilderingGrace,&[1,2,3]).unwrap());
            if !runner.battle.state().grace.as_ref().unwrap().items_disabled
                || runner.next()!=Some(Action::ThunderSoulstone) {continue;}
            let snapshot=runner.battle.state().clone();
            assert_eq!(runner.advance_to_search().unwrap(),None);
            assert_eq!(runner.battle.state(),&snapshot);
            assert!(!runner.actions.iter().any(|a|a.action==Action::ThunderSoulstone));
            assert!(matches!(runner.evaluation(),Evaluation::Loss {reasons}
                if reasons.contains(&LossReason::RoutineExhausted)));
            return;
        }
        panic!("No scattered-items BG case exercised");
    }
    #[test]
    fn galdera_osvald_takes_the_second_merchant_role_in_actual_battle_order() {
        let party:Vec<_>=[7,6,8,4].map(|id|crate::field_escape::route_player(id,5000).unwrap()).into();
        let battle=crate::field_escape::carried_step_ahead(crate::CrtRng::new(123),
            crate::escape::Situation::Normal,&party,
            &["ENE_NML_FLD_WLD_250","ENE_NML_FLD_WLD_160","ENE_NML_OTR_150"],
            true,&crate::knowledge::EnemyHistory::new()).unwrap();
        let plan=OpeningPlan::from_battle(&battle).unwrap();
        let osvald=battle.actor_id("osvald").unwrap();
        let partitio=battle.actor_id("partitio").unwrap();
        let merchants:Vec<_>=battle.state().turn_order().iter().copied().filter(|id|*id==osvald||*id==partitio).collect();
        assert_eq!(plan.bonus_turn.iter().find(|a|a.actor==merchants[0]).unwrap().action,Action::DonateBp {target:merchants[1]});
        assert_eq!(plan.bonus_turn.iter().find(|a|a.actor==merchants[1]).unwrap().action,Action::Guards);
        let fight=replay(battle,&[1,2,3],&[1,2,3]).unwrap();
        assert!(fight.actions.iter().any(|a|a.actor==osvald && matches!(a.action,Action::DonateBp {..}|Action::Guards)));
    }
    #[test]
    fn every_opening_order_uses_fixed_roles_and_donation_precedes_guards() {
        let mut count=0;
        for a in 0..4 {for b in 0..4 {for c in 0..4 {for d in 0..4 {
            let order=[ActorId(a),ActorId(b),ActorId(c),ActorId(d)];
            if (0..4).any(|i|order[..i].contains(&order[i])) {continue;}
            let p=OpeningPlan::new(&order,ActorId(0),ActorId(2),[ActorId(1),ActorId(3)]).unwrap();count+=1;
            let merchant_order:Vec<_>=order.iter().filter(|id|matches!(id.0,1|3)).copied().collect();
            for row in p.bonus_turn {
                assert_eq!(row.action,if row.actor==ActorId(0) {Action::Pomegranate {target:ActorId(2)}} else if row.actor==ActorId(2) {Action::Defend} else if row.actor==merchant_order[0] {Action::DonateBp {target:merchant_order[1]}} else {Action::Guards});
            }
        }}}}
        assert_eq!(count,24);
    }
}
