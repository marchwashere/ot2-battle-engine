//! SummonAbilityCommand.SetupSummonData; this is the preparation lottery,
//! separate from summon construction, child hit callbacks and command cleanup.
use crate::{CrtRng, Result, ensure};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
#[derive(Debug, Deserialize)]
struct Variant { id:String, weight:u32 }
#[derive(Debug, Deserialize)]
struct Data { guard_variants:Vec<Variant> }
fn data()->&'static Data {
    static DATA:OnceLock<Data>=OnceLock::new();
    DATA.get_or_init(||serde_json::from_str(include_str!("../data/gravell_battle.json")).expect("Guard summon import"))
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GuardSummons {
    pub variants:Vec<&'static str>,
    pub count:u8,
    pub before:CrtRng,
    pub after:CrtRng,
}
/// Source prepares FOUR weighted variants without replacement regardless of BP;
/// only boost+1 are subsequently summoned. Even the final singleton draws CRT.
pub fn prepare_guards(rng:&mut CrtRng,boost:u8)->Result<GuardSummons> {
    ensure(boost<=3,"Hired Help boost must be 0..3")?;
    let before=*rng;let mut pool:Vec<_>=data().guard_variants.iter().collect();let mut variants=vec![];
    for _ in 0..4 {
        let total=pool.iter().map(|v|v.weight).sum();let roll=rng.index(total);let mut sum=0;
        let index=pool.iter().position(|v|{sum+=v.weight;roll<sum}).expect("positive Guard weights");
        variants.push(pool.remove(index).id.as_str());
    }
    Ok(GuardSummons {variants,count:boost+1,before,after:*rng})
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preparation_is_four_draws_even_for_one_or_three_guards() {
        for boost in 0..4 {
            let mut rng=CrtRng::new(9182);let result=prepare_guards(&mut rng,boost).unwrap();
            assert_eq!(result.count,boost+1);assert_eq!(rng.draws,4);
            let mut rows=result.variants;rows.sort();rows.dedup();assert_eq!(rows.len(),4);
        }
        let mut rng=CrtRng::new(72);let before=rng;assert!(prepare_guards(&mut rng,4).is_err());assert_eq!(rng,before);
    }
}
