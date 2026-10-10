//! Exact command-menu boost frontier. Search distinct RNG/voice states, retaining
//! the minimum button path to each; do not reinitialize battles from the seed.
use crate::{Battle, Command, CrtRng, Result, Side, StepReport, ensure};
use std::collections::{BTreeSet, VecDeque};

#[derive(Clone)]
struct Node { battle:Battle, bp:u8, inputs:Vec<u8> }
pub struct BoostFrontier {
    base:Battle,
    maximum:u8,
    queue:VecDeque<Node>,
    seen:BTreeSet<(u32,u64,u8,Vec<u64>)>,
}
pub struct Candidate {
    pub inputs:Vec<u8>,
    pub selected_bp:u8,
    pub rng:CrtRng,
    pub after_action:Battle,
    pub report:StepReport,
}
impl BoostFrontier {
    pub fn new(menu:&Battle)->Result<Self> {
        let actor=menu.current_actor().ok_or_else(||crate::SimError("Fight search requires an active command menu".into()))?;
        ensure(menu.side(actor)==Some(Side::Player),"Fight search requires a player command menu")?;
        ensure(menu.state().opening_crt.is_some(),"Fight search requires full CRT callbacks")?;
        let mut initial=Node {battle:menu.clone(),bp:0,inputs:vec![]};
        initial.battle.omit_search_trace();
        let maximum=menu.state().actors[actor.0].resources.bp.min(3);
        let mut result=Self {base:menu.clone(),maximum,queue:VecDeque::new(),seen:BTreeSet::new()};
        result.seen.insert(Self::key(&initial));result.queue.push_back(initial);Ok(result)
    }
    fn key(node:&Node)->(u32,u64,u8,Vec<u64>) {
        let s=node.battle.state();
        (s.rng.state,s.rng.draws,node.bp,s.opening_crt.as_ref().unwrap().used_voices.clone())
    }
    /// Returns the next unique ready-to-execute state, by minimum input count.
    /// Reaching a caller's work limit leaves the frontier intact for resumption.
    /// Unsupported action errors propagate: they are never labelled no solution.
    pub fn next(&mut self,command:&Command,max_inputs:usize)->Result<Option<Candidate>> {
        self.next_matching(command,max_inputs,None)
    }
    /// Skip action execution unless the boost endpoint matches an exact CRT state.
    pub fn next_matching(&mut self,command:&Command,max_inputs:usize,target:Option<u32>)->Result<Option<Candidate>> {
        self.next_bounded(command,max_inputs,target,None,None)
    }
    /// The action cannot contain a target lottery already passed by boosting.
    /// RNG draws are monotone, so the entire over-deadline subtree is discarded.
    pub fn next_before(&mut self,command:&Command,max_inputs:usize,deadline:Option<u64>)->Result<Option<Candidate>> {
        self.next_bounded(command,max_inputs,None,deadline,None)
    }
    /// Joint frontiers may reach the same complete mutable battle state.
    /// The caller visits total costs monotonically, so later duplicates cannot
    /// improve the solution. Compare full state, not RNG alone.
    pub fn next_unique_before(&mut self,command:&Command,max_inputs:usize,deadline:Option<u64>,seen:&mut std::collections::BTreeMap<u32,Vec<crate::BattleState>>)->Result<Option<Candidate>> {
        self.next_bounded(command,max_inputs,None,deadline,Some(seen))
    }
    fn next_bounded(&mut self,command:&Command,max_inputs:usize,target:Option<u32>,deadline:Option<u64>,mut seen:Option<&mut std::collections::BTreeMap<u32,Vec<crate::BattleState>>>)->Result<Option<Candidate>> {
        let final_bp=match command {Command::Boost {bp,..}=>*bp,Command::Latent {..}=>return Err(crate::SimError("Activate latent before constructing this frontier".into())),_=>0};
        ensure(final_bp<=self.maximum,"Requested action BP exceeds available BP")?;
        loop {
            if self.queue.front().is_none_or(|n|n.inputs.len()>max_inputs) {return Ok(None);}
            let node=self.queue.pop_front().unwrap();
            if deadline.is_some_and(|d|node.battle.state().rng.draws>d) {continue;}
            for bp in [node.bp.checked_sub(1),node.bp.checked_add(1).filter(|n|*n<=self.maximum)].into_iter().flatten() {
                let mut next=node.clone();next.battle.advance_boost_input(node.bp,bp)?;
                next.bp=bp;next.inputs.push(bp);
                if self.seen.insert(Self::key(&next)) {self.queue.push_back(next);}
            }
            if node.bp!=final_bp || target.is_some_and(|t|node.battle.state().rng.state!=t) {continue;}
            if let Some(seen)=seen.as_deref_mut() {
                let state=node.battle.state();let bucket=seen.entry(state.rng.state).or_default();
                if bucket.iter().any(|old|old==state) {continue;}
                bucket.push(state.clone());
            }
            let mut after_action=self.base.clone();
            let report=if node.inputs.is_empty() {after_action.step_player(command.clone())?}
                else {after_action.step_player_with_boost_inputs(command.clone(),&node.inputs)?};
            return Ok(Some(Candidate {inputs:node.inputs,selected_bp:node.bp,rng:node.battle.state().rng,after_action,report}));
        }
    }
}
