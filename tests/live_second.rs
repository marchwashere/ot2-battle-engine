use ot2_battle_engine::{routine::ThroneSecondRoutine, *};

#[test]
fn captured_second_fight_matches_menus_enemy_hit_and_every_crt_caller() {
    let capture: serde_json::Value =
        serde_json::from_str(include_str!("../data/source/live-second-5120.json")).unwrap();
    let mut battle = Battle::new_throne_second(
        BattleConfig::throne_second(capture["initial_state"].as_u64().unwrap() as u32).unwrap(),
        ItemCatalog::builtin().unwrap(),
        true,
    )
    .unwrap();
    let mut menus = Vec::new();
    let mut enemy_actions = 0;
    let mut enemy_hits = Vec::new();
    while battle.state().outcome == Outcome::Ongoing {
        let actor = battle.current_actor().unwrap();
        if battle.side(actor) == Some(Side::Player) {
            menus.push(
                serde_json::json!({"round":battle.state().round,"actor":actor.0,
                "advances":5120+battle.state().rng.draws}),
            );
        } else {
            enemy_actions += 1;
            assert_eq!(actor, battle.actor_id("leader").unwrap());
        }
        let report = ThroneSecondRoutine.step(&mut battle).unwrap();
        for event in report.events {
            if let Event::Damage { target, .. } = event {
                if battle.side(target) == Some(Side::Player) {
                    enemy_hits.push(target);
                }
            }
        }
    }
    assert_eq!(menus, *capture["command_menus"].as_array().unwrap());
    assert_eq!(enemy_actions, 1);
    assert_eq!(enemy_hits, [battle.actor_id("pirro").unwrap()]);
    assert_eq!(battle.state().outcome, Outcome::Victory);
    assert_eq!(battle.state().rng.draws, 144);
    assert_eq!(capture["final_state"], battle.state().rng.state);

    let mut rows = capture["records"].as_array().unwrap().iter();
    for span in battle
        .state()
        .opening_crt
        .as_ref()
        .unwrap()
        .trace
        .as_ref()
        .unwrap()
    {
        let mut rng = span.before;
        let count = span.after.draws - span.before.draws;
        for i in 0..count {
            let row = rows.next().expect("extra simulated call");
            assert_eq!(row["position"], 5120 + rng.draws);
            assert_eq!(row["before"], rng.state);
            let caller = match span.reason {
                "actor_initialization" if span.actor.unwrap().0 >= 3 => {
                    ["0x30D7424", "0x30D7604", "0x30D7065"][i as usize]
                }
                "actor_initialization" | "round_timing_revision" => "0x30D7065",
                "critical_hit_and_damage" if i + 1 == count => "0x30D7065",
                "enemy_ai_and_target" => "0x30D7424",
                "damage_notification_timer"
                | "break_slow_timer"
                | "broken_hit_slow_timer"
                | "round_bp_timer" => "0x2C52E57",
                "critical_hit_and_damage"
                | "current_order_shuffle"
                | "next_order_shuffle"
                | "normal_accuracy"
                | "hp_ratio_accuracy"
                | "order_voice"
                | "ability_voice"
                | "break_voice"
                | "kill_voice"
                | "damage_voice"
                | "attack_voice"
                | "last_attack_voice"
                | "boost_voice"
                | "victory_voice"
                | "enemy_drop" => "0x2C57B26",
                reason => panic!("unexpected callback {reason}"),
            };
            assert_eq!(
                row["returnRva"],
                caller,
                "{} at {}",
                span.reason,
                5120 + rng.draws
            );
            rng.next_u15();
        }
        assert_eq!(span.after, rng);
    }
    assert!(rows.next().is_none(), "unmodeled game call");
}
