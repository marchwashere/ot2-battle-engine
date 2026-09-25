use ot2_battle_engine::{
    ActorId, Battle, BattleConfig, Command, ItemCatalog, Outcome, Side, SimError, StepReport,
};
use std::{env, fs};

fn print_report(report: &StepReport) -> Result<(), serde_json::Error> {
    println!("{}", serde_json::to_string(report)?);
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().skip(1).collect();
    match args.first().map(String::as_str).unwrap_or("demo") {
        "throne" => {
            if args.len()>2 {return Err(SimError("usage: ot2-sim throne [seed]".into()).into());}
            let mut config=BattleConfig::from_json(include_str!("../data/throne_opening.json"))?;
            if let Some(seed)=args.get(1) {config.seed=seed.parse()?;}
            let mut battle=Battle::new(config,ItemCatalog::builtin()?)?;
            eprintln!("Throne opening: ENG_EVE_THI_C01_010; source-derived combat, full-game CRT callback coverage incomplete");
            for _ in 0..500 {
                if battle.state().outcome!=Outcome::Ongoing {break;}
                let actor=battle.current_actor().ok_or_else(||SimError("missing current actor".into()))?;
                let report=if battle.side(actor)==Some(Side::Enemy) {battle.step_enemy()?} else {
                    let target=battle.state().actors.iter().enumerate().find(|(i,a)|a.alive()&&battle.side(ActorId(*i))==Some(Side::Enemy)).map(|(i,_)|ActorId(i)).expect("ongoing enemy");
                    let weapon=if battle.actor_id("scaracci")==Some(actor) {"attack_axe"} else {"attack_dagger"};
                    let boost=if battle.state().actors[target.0].broken() {battle.state().actors[actor.0].resources.bp.min(3)} else {0};
                    battle.step_player(Command::Boost {id:weapon.into(),target:Some(target),bp:boost})?
                };
                print_report(&report)?;
            }
            if battle.state().outcome==Outcome::Ongoing {return Err(SimError("opening battle action budget exhausted".into()).into());}
            println!("{}",serde_json::to_string(battle.state())?);
        }
        "demo" => {
            if args.len() > 2 { return Err(SimError("usage: ot2-sim demo [seed]".into()).into()); }
            let mut config = BattleConfig::from_json(include_str!("../data/demo_battle.json"))?;
            if let Some(seed) = args.get(1) { config.seed = seed.parse()?; }
            let mut battle = Battle::new(config, ItemCatalog::builtin()?)?;
            eprintln!("scaffold_v1 demo: synthetic encounter, not OT2-verified damage/turn/AI rules");
            for _ in 0..100 {
                if battle.state().outcome != Outcome::Ongoing { break; }
                let actor = battle.current_actor().ok_or_else(|| SimError("missing current actor".into()))?;
                let report = if battle.side(actor) == Some(Side::Enemy) { battle.step_enemy()? } else {
                    let resources = &battle.state().actors[actor.0].resources;
                    let target = battle.state().actors.iter().enumerate().find(|(i, a)| a.alive() && battle.side(ActorId(*i)) == Some(Side::Enemy)).map(|(i, _)| ActorId(i));
                    let command = if resources.hp < 300 && battle.item_count("healing_grape")? > 0 {
                        Command::Item { id: "healing_grape".into(), target: Some(actor) }
                    } else if resources.sp < 20 && battle.item_count("inspiriting_plum")? > 0 {
                        Command::Item { id: "inspiriting_plum".into(), target: Some(actor) }
                    } else if resources.sp >= 20 { Command::Skill { id: "training_strike".into(), target } } else { Command::Wait };
                    battle.step_player(command)?
                };
                print_report(&report)?;
            }
            if battle.state().outcome == Outcome::Ongoing { return Err(SimError("demo action budget exhausted".into()).into()); }
            println!("{}", serde_json::to_string(battle.state())?);
        }
        "run" => {
            if args.len() != 3 { return Err(SimError("usage: ot2-sim run <battle.json> <commands.json>".into()).into()); }
            let config = BattleConfig::from_json(&fs::read_to_string(&args[1])?)?;
            let commands: Vec<Command> = serde_json::from_str(&fs::read_to_string(&args[2])?)?;
            let mut battle = Battle::new(config, ItemCatalog::builtin()?)?;
            for command in commands {
                for report in battle.advance_enemies(10_000)? { print_report(&report)?; }
                if battle.state().outcome != Outcome::Ongoing { return Err(SimError("command script has actions after battle end".into()).into()); }
                print_report(&battle.step_player(command)?)?;
            }
            for report in battle.advance_enemies(10_000)? { print_report(&report)?; }
            println!("{}", serde_json::to_string(battle.state())?);
        }
        "validate-ai" => {
            if args.len() != 2 { return Err(SimError("usage: ot2-sim validate-ai <ai.json>".into()).into()); }
            let ai = ot2_battle_engine::ai::BossAi::from_json(&fs::read_to_string(&args[1])?)?;
            println!("Valid AI structure: {} states. Skill references are checked when loading a battle.", ai.states.len());
        }
        _ => return Err(SimError("usage: ot2-sim [throne [seed] | demo [seed] | run <battle.json> <commands.json> | validate-ai <ai.json>]".into()).into()),
    }
    Ok(())
}
