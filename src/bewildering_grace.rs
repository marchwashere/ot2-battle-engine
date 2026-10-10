//! Resumable source lottery. Selection and child execution MUST alternate.
//! This module deliberately does not pretend to execute child commands: the
//! battle executor must complete their targeting/effects/CRT callbacks first.
use crate::{CrtRng, Result, ensure};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Debug, Deserialize)]
struct Entry { ability: String, weight: u32 }
#[derive(Debug, Deserialize)]
struct Tier { ability: String, count: u8, total: u32, entries: Vec<Entry> }
#[derive(Debug, Deserialize)]
struct Data { tiers: Vec<Tier> }
fn data() -> &'static Data {
    static DATA: OnceLock<Data> = OnceLock::new();
    DATA.get_or_init(|| {
        let d: Data = serde_json::from_str(include_str!("../data/gravell_battle.json")).expect("BG import");
        assert_eq!(d.tiers.len(), 4);
        for (i,t) in d.tiers.iter().enumerate() {
            assert_eq!(t.count as usize,i+1);
            assert_eq!(t.entries.iter().map(|e|e.weight).sum::<u32>(),t.total);
            assert!(t.total>0);
        }
        d
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Selection {
    pub index: u8,
    pub ability: &'static str,
    pub roll: u32,
    pub before: CrtRng,
    pub after: CrtRng,
}

/// A pending child prevents accidentally drawing all four effects upfront.
/// Clone this together with the battle state when evaluating a search candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Dance {
    boost: u8,
    completed: u8,
    pending: Option<Selection>,
    stopped: bool,
}
impl Dance {
    pub fn new(boost:u8)->Result<Self> {
        ensure(boost<=3,"Bewildering Grace boost must be 0..3")?;
        Ok(Self {boost,completed:0,pending:None,stopped:false})
    }
    pub fn parent_ability(&self)->&'static str { &data().tiers[self.boost as usize].ability }
    pub fn finished(&self)->bool { self.stopped || self.completed==data().tiers[self.boost as usize].count }
    pub fn select(&mut self,rng:&mut CrtRng)->Result<Option<Selection>> {
        ensure(self.pending.is_none(),"Complete the selected BG child before the next lottery draw")?;
        if self.finished() { return Ok(None); }
        let tier=&data().tiers[self.boost as usize];
        let before=*rng;
        let roll=1+rng.index(tier.total);
        let mut sum=0;
        let selected=tier.entries.iter().find(|e|{sum+=e.weight;roll<=sum}).expect("validated BG weights");
        let result=Selection {index:self.completed,ability:&selected.ability,roll,before,after:*rng};
        self.pending=Some(result.clone());
        Ok(Some(result))
    }
    /// Called only after the complete selected command and UpdateCharacterList.
    /// Dead enforcer or no living enemies ends the parent without another draw.
    pub fn complete_child(&mut self,ability:&str,enforcer_alive:bool,enemies_alive:bool)->Result<()> {
        ensure(self.pending.as_ref().is_some_and(|s|s.ability==ability),"BG child completion does not match pending command")?;
        self.pending=None;
        self.completed+=1;
        self.stopped=!enforcer_alive||!enemies_alive;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_verified_x100_positions_and_zero_weight_rows() {
        for (position,id) in [(3285,"ABI_DAN_MAKA_002"),(305,"ABI_DAN_MAKA_001")] {
            let mut rng=CrtRng::new(0);rng.advance(position);
            let s=Dance::new(3).unwrap().select(&mut rng).unwrap().unwrap();
            assert_eq!(s.ability,id);assert_eq!(s.after.draws,position+1);
        }
        for boost in 0..3 {
            for seed in 0..1000 {
                let s=Dance::new(boost).unwrap().select(&mut CrtRng::new(seed)).unwrap().unwrap();
                assert!(!matches!(s.ability,"ABI_DAN_MAKA_001"|"ABI_DAN_MAKA_002"));
            }
        }
    }
    #[test]
    fn child_rng_must_run_between_selections_and_failures_are_atomic() {
        let mut dance=Dance::new(3).unwrap();let mut rng=CrtRng::new(0);
        let first=dance.select(&mut rng).unwrap().unwrap();
        let snapshot=(dance.clone(),rng);
        assert!(dance.select(&mut rng).is_err());assert_eq!((dance.clone(),rng),snapshot);
        assert!(dance.complete_child("wrong",true,true).is_err());assert_eq!(dance,snapshot.0);
        rng.advance(7); // a child executor, not the lottery, owns these calls
        dance.complete_child(first.ability,true,true).unwrap();
        let second=dance.select(&mut rng).unwrap().unwrap();assert_eq!(second.before.draws,8);
        dance.complete_child(second.ability,false,true).unwrap();
        let before=rng;assert!(dance.select(&mut rng).unwrap().is_none());assert_eq!(rng,before);
    }
    #[test]
    fn no_extra_selection_after_four_children_or_early_victory() {
        let mut dance=Dance::new(3).unwrap();let mut rng=CrtRng::new(71);
        for i in 0..4 {let s=dance.select(&mut rng).unwrap().unwrap();assert_eq!(s.index,i);dance.complete_child(s.ability,true,true).unwrap();}
        assert!(dance.finished());assert!(dance.select(&mut rng).unwrap().is_none());assert_eq!(rng.draws,4);
        let mut dance=Dance::new(3).unwrap();let s=dance.select(&mut rng).unwrap().unwrap();
        dance.complete_child(s.ability,true,false).unwrap();assert!(dance.finished());
    }
}
