use ot2_battle_engine::{Battle, BattleConfig, CrtRng, ItemCatalog, Outcome, pirro::PirroRoutine};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let seed = args.get(1).map_or(Ok(0), |v| v.parse::<u32>())?;
    let advances = args.get(2).map_or(Ok(0), |v| v.parse::<u64>())?;
    let mut rng = CrtRng::new(seed);
    rng.advance(advances);
    let mut battle = Battle::new_pirro(
        BattleConfig::throne_pirro(rng.state)?,
        ItemCatalog::builtin()?,
        true,
    )?;
    let mut routine = PirroRoutine::default();
    let mut steps = Vec::new();
    let mut error = None;
    while battle.state().outcome == Outcome::Ongoing && steps.len() < 80 {
        let round = battle.state().round;
        match routine.step(&mut battle) {Ok(report)=>steps.push(serde_json::json!({"round":round,"report":report,"hp":battle.state().actors.iter().map(|a|a.resources.hp).collect::<Vec<_>>(),"ai":battle.state().pirro,"next":battle.state().next_queue})),Err(e)=>{error=Some(e.to_string());break;}}
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"entry":rng,"routine":routine,"steps":steps,"state":battle.state(),"error":error})
        )?
    );
    Ok(())
}
