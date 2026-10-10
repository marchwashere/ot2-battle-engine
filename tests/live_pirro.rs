use ot2_battle_engine::{pirro::PirroRoutine, *};

#[test]
fn captured_pirro_victory_matches_every_call_through_boss_death_and_drop() {
    let capture: serde_json::Value =
        serde_json::from_str(include_str!("../data/source/live-pirro-5595-victory.json")).unwrap();
    let mut battle = Battle::new_pirro(
        BattleConfig::throne_pirro(capture["initial_state"].as_u64().unwrap() as u32).unwrap(),
        ItemCatalog::builtin().unwrap(),
        true,
    )
    .unwrap();
    let mut routine = PirroRoutine::default();
    let mut menus = vec![];
    while battle.state().outcome == Outcome::Ongoing {
        if battle.current_actor() == Some(ActorId(0)) {
            menus.push(5595 + battle.state().rng.draws);
        }
        routine.step(&mut battle).unwrap();
    }
    assert_eq!(battle.state().outcome, Outcome::Victory);
    assert_eq!(
        menus,
        [
            5631, 5669, 5693, 5739, 5758, 5768, 5798, 5806, 5832, 5836, 5892
        ]
    );
    assert_eq!(battle.state().rng.draws, 305);
    assert_eq!(capture["final_state"], battle.state().rng.state);
    assert_eq!(capture["final_advances"], 5595 + battle.state().rng.draws);
    let trace = battle
        .state()
        .opening_crt
        .as_ref()
        .unwrap()
        .trace
        .as_ref()
        .unwrap();
    let records = capture["records"].as_array().unwrap();
    let mut matched = 0;
    for span in trace {
        let mut rng = span.before;
        let count = span.after.draws - span.before.draws;
        for i in 0..count {
            let row = &records[rng.draws as usize];
            assert_eq!(row["position"], 5595 + rng.draws);
            assert_eq!(row["before"], rng.state);
            let caller = match span.reason {
                "actor_initialization" if span.actor == Some(ActorId(1)) => {
                    ["0x30D7424", "0x30D7604", "0x30D7065"][i as usize]
                }
                "actor_initialization" | "round_timing_revision" => "0x30D7065",
                "pirro_ai_initialization" => "0x2C4A60D",
                "pirro_ai_and_target" if i + 1 < count => "0x2C4A60D", // AI table reshuffle.
                "pirro_phase_frame" | "pirro_ai_and_target" => "0x30D7424",
                "critical_hit_and_damage" if i + 1 == count => "0x30D7065",
                "pirro_status_application" | "scripted_forced_miss_status" if i > 0 => "0x30D7604",
                "damage_notification_timer"
                | "break_slow_timer"
                | "round_bp_timer"
                | "enemy_death_slow_timer"
                | "broken_hit_slow_timer" => "0x2C52E57",
                "current_order_shuffle"
                | "next_order_shuffle"
                | "battle_start_voice"
                | "order_voice"
                | "ability_voice"
                | "weakness_voice"
                | "damage_voice"
                | "death_voice"
                | "boost_voice"
                | "normal_accuracy"
                | "attack_voice"
                | "item_voice"
                | "critical_hit_and_damage"
                | "pirro_status_application"
                | "scripted_forced_miss_status"
                | "hp_ratio_accuracy"
                | "last_attack_voice"
                | "latent_voice"
                | "victory_voice"
                | "enemy_drop" => "0x2C57B26",
                reason => panic!("unmapped captured callback {reason}"),
            };
            assert_eq!(
                row["returnRva"],
                caller,
                "{} at {}",
                span.reason,
                5595 + rng.draws
            );
            if span.reason == "enemy_death_slow_timer" {
                assert_eq!(5595 + rng.draws, 5897);
            }
            rng.next_u15();
            matched += 1;
        }
    }
    assert_eq!(matched, records.len());
}

#[test]
fn captured_pirro_menus_critical_and_all_crt_calls_match() {
    let capture: serde_json::Value =
        serde_json::from_str(include_str!("../data/source/live-pirro-5678.json")).unwrap();
    let entry = capture["initial_state"].as_u64().unwrap() as u32;
    let mut battle = Battle::new_pirro(
        BattleConfig::throne_pirro(entry).unwrap(),
        ItemCatalog::builtin().unwrap(),
        true,
    )
    .unwrap();
    let mut routine = PirroRoutine::default();
    let mut menus = vec![];
    let mut critical = None;
    while battle.state().outcome == Outcome::Ongoing {
        let round = battle.state().round;
        if battle.current_actor() == Some(ActorId(0)) {
            menus.push(serde_json::json!({"round":round,"actor":0,
                "advances":5678 + battle.state().rng.draws}));
        }
        let report = routine.step(&mut battle).unwrap();
        if round == 4 && report.actor == ActorId(0) {
            critical = report.events.iter().find_map(|event| match event {
                Event::Damage {
                    amount, critical, ..
                } => Some((*amount, *critical)),
                _ => None,
            });
        }
    }
    assert_eq!(menus, *capture["command_menus"].as_array().unwrap());
    assert_eq!(critical, Some((38, true)));
    assert_eq!(battle.state().outcome, Outcome::Defeat);
    assert_eq!(battle.state().round, 7);
    assert_eq!(battle.state().rng.draws, 203);
    assert_eq!(capture["final_advances"], 5678 + battle.state().rng.draws);
    assert_eq!(capture["final_state"], battle.state().rng.state);
    let trace = battle
        .state()
        .opening_crt
        .as_ref()
        .unwrap()
        .trace
        .as_ref()
        .unwrap();
    let reactions: Vec<_> = trace
        .iter()
        .filter(|s| s.reason == "weakness_voice")
        .collect();
    assert_eq!(reactions.len(), 1); // Darkest Night only reveals dark once.
    assert_eq!(reactions[0].before.draws + 5678, 5717);
    assert_eq!(reactions[0].after.draws - reactions[0].before.draws, 1);
    assert_ne!(battle.state().actors[1].revealed_element_weaknesses, 0);
    assert_ne!(battle.state().actors[1].revealed_weapon_weaknesses, 0);

    let recorded = capture["records"].as_array().unwrap();
    let mut matched = 0;
    for span in trace {
        let mut rng = span.before;
        let count = span.after.draws - span.before.draws;
        for i in 0..count {
            let row = &recorded[rng.draws as usize];
            assert_eq!(row["position"], 5678 + rng.draws);
            assert_eq!(row["before"], rng.state);
            let caller = match span.reason {
                "actor_initialization" if span.actor == Some(ActorId(1)) => {
                    ["0x30D7424", "0x30D7604", "0x30D7065"][i as usize]
                }
                "actor_initialization" | "round_timing_revision" => "0x30D7065",
                "pirro_ai_initialization" => "0x2C4A60D",
                "pirro_phase_frame" | "pirro_ai_and_target" => "0x30D7424",
                "critical_hit_and_damage" if i + 1 == count => "0x30D7065",
                "pirro_status_application" if i > 0 => "0x30D7604",
                "damage_notification_timer" | "break_slow_timer" | "round_bp_timer" => "0x2C52E57",
                "current_order_shuffle"
                | "next_order_shuffle"
                | "battle_start_voice"
                | "order_voice"
                | "ability_voice"
                | "weakness_voice"
                | "damage_voice"
                | "death_voice"
                | "boost_voice"
                | "normal_accuracy"
                | "attack_voice"
                | "item_voice"
                | "critical_hit_and_damage"
                | "pirro_status_application" => "0x2C57B26",
                reason => panic!("unmapped captured callback {reason}"),
            };
            assert_eq!(
                row["returnRva"],
                caller,
                "{} at {}",
                span.reason,
                5678 + rng.draws
            );
            if span.reason == "pirro_phase_frame" {
                assert!(
                    row["blueprint"]
                        .as_str()
                        .unwrap()
                        .contains("ExecuteUbergraph_KSBattleCharacterEnemy")
                );
            } else if span.reason == "pirro_ai_and_target" {
                assert!(
                    row["blueprint"]
                        .as_str()
                        .unwrap()
                        .ends_with("SelectRandomTargetWithWeight")
                );
            }
            rng.next_u15();
            matched += 1;
        }
    }
    assert_eq!(matched, recorded.len());
}
