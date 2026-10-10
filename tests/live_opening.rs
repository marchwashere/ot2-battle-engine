//! User-reported checkpoints; never replace them with simulator output.
use ot2_battle_engine::*;

/// Explicit actions from the historical capture. They must not change when
/// the user's current automatic routine changes.
#[derive(Default)]
struct CapturedReplay {
    next: usize,
    first_round: Vec<ActorId>,
}
impl CapturedReplay {
    fn step(&mut self, battle: &mut Battle) -> Result<StepReport> {
        let (name, target, bp) = [
            ("pirro", Some("pursuer_a"), 0),
            ("throne", Some("pursuer_b"), 1),
            ("scaracci", Some("pursuer_a"), 0),
            ("scaracci", Some("pursuer_a"), 2),
            ("throne", None, 0),
            ("pirro", Some("pursuer_b"), 2),
        ][self.next];
        let actor = battle.actor_id(name).unwrap();
        assert_eq!(battle.current_actor(), Some(actor));
        assert_eq!(battle.state().round, if self.next < 3 { 1 } else { 2 });
        let command = match target {
            Some(target) => Command::Boost {
                id: battle.default_attack(actor)?.into(),
                target: battle.actor_id(target),
                bp,
            },
            None => Command::Defend,
        };
        let report = battle.step_player(command)?;
        if self.next < 3 {
            self.first_round.push(actor);
        }
        self.next += 1;
        Ok(report)
    }
}

fn case() -> (serde_json::Value, Battle) {
    let observed: serde_json::Value =
        serde_json::from_str(include_str!("../data/source/live-opening-4929.json")).unwrap();
    let mut rng = CrtRng::new(observed["initial_seed"].as_u64().unwrap() as u32);
    rng.advance(observed["prebattle_advances"].as_u64().unwrap());
    assert_eq!(rng.state, 0x0EA7_9703);
    let mut config = BattleConfig::from_json(include_str!("../data/throne_opening.json")).unwrap();
    config.seed = rng.state;
    let battle = Battle::new_opening(config, ItemCatalog::builtin().unwrap(), true).unwrap();
    (observed, battle)
}

fn ids(observed: &serde_json::Value, key: &str, battle: &Battle) -> Vec<ActorId> {
    observed[key]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| battle.actor_id(v.as_str().unwrap()).unwrap())
        .collect()
}

#[test]
fn reported_first_command_menu_state() {
    let (observed, battle) = case();
    let expected = u32::from_str_radix(
        observed["first_command_menu"]
            .as_str()
            .unwrap()
            .trim_start_matches("0x"),
        16,
    )
    .unwrap();
    assert_eq!(battle.state().rng.state, expected);
    assert_eq!(battle.state().rng.draws, 30);
}

#[test]
fn captured_title_to_pirro_transition_preserves_every_crt_call() {
    let capture: serde_json::Value = serde_json::from_str(include_str!(
        "../data/source/live-throne-title-to-pirro-20260926-trace.json"
    ))
    .unwrap();
    let previous: serde_json::Value =
        serde_json::from_str(include_str!("../data/source/live-opening-4929-trace.json")).unwrap();
    assert_eq!(
        capture["finish"],
        "Detached with original debug registers restored"
    );
    let rows = capture["records"].as_array().unwrap();
    assert_eq!(rows.len(), 43);
    let mut rng = CrtRng::new(0);
    rng.advance(4916);
    assert_eq!(capture["initialState"], rng.state);
    let prebattle_callers = [
        "0x2C52E57", // Timer setup.
        "0x11C010F",
        "0x11C010F",
        "0x11C010F", // Streamed-level colors.
        "0x11C010F",
        "0x11C010F",
        "0x11C010F",
        "0x30D7604", // Field NPC starting animation frame.
        "0x30D7424",
        "0x30D7424", // First encounter-counter pair.
        "0xEA7749",  // Global Black Market flag lottery.
        "0x30D7424",
        "0x30D7424", // Second encounter-counter pair.
    ];
    for (i, row) in rows.iter().enumerate() {
        assert_eq!(row["kind"], "rand", "unexpected reseed at {i}");
        assert_eq!(row["before"], rng.state, "CRT discontinuity at {i}");
        assert_eq!(row["position"], rng.draws);
        if i < prebattle_callers.len() {
            assert_eq!(row["returnRva"], prebattle_callers[i]);
        } else {
            // The earlier full battle capture is independently checked against
            // the simulator below; this replay reproduces its first 30 calls.
            let old = &previous["records"][i - prebattle_callers.len()];
            assert_eq!(row["before"], old["before"]);
            assert_eq!(row["returnRva"], old["returnRva"]);
        }
        rng.next_u15();
        if i == 12 {
            assert_eq!(rng.state, 0x0EA7_9703);
            assert_eq!(rng.draws, 4929);
        }
    }
    assert_eq!(capture["finalState"], rng.state);
    assert_eq!(rng.state, 0xF485_D421);
    assert_eq!(rng.draws, 4959);
    assert_eq!(case().1.state().rng.state, rng.state);
}

#[test]
fn reported_second_round_party_preview() {
    let (observed, battle) = case();
    let party: Vec<_> = battle
        .state()
        .next_queue
        .iter()
        .copied()
        .filter(|id| battle.side(*id) == Some(Side::Player))
        .collect();
    assert_eq!(party, ids(&observed, "second_round_party", &battle));
}

#[test]
fn captured_repeat_omits_one_level_color_draw() {
    let capture: serde_json::Value = serde_json::from_str(include_str!(
        "../data/source/live-throne-repeat-20260926-trace.json"
    ))
    .unwrap();
    let first: serde_json::Value = serde_json::from_str(include_str!(
        "../data/source/live-throne-title-to-pirro-20260926-trace.json"
    ))
    .unwrap();
    assert_eq!(
        capture["finish"],
        "Detached with original debug registers restored"
    );
    let all = capture["records"].as_array().unwrap();
    assert_eq!(all.len(), 244);
    // Preserve the discontinuity before the replay. The title marker is not
    // its actual starting RNG state, and no srand was captured at the change.
    let mut marker = CrtRng::new(all[201]["before"].as_u64().unwrap() as u32);
    marker.next_u15();
    assert_eq!(marker.state, 0xEA4F_203D);
    let rows = &all[202..];
    assert_eq!(rows.len(), 42);
    let expected: Vec<_> = first["records"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .filter_map(|(i, row)| (i != 6).then_some(row))
        .collect();
    assert_eq!(first["records"][6]["returnRva"], "0x11C010F");
    let mut rng = CrtRng::new(0);
    rng.advance(4917);
    assert_ne!(marker.state, rng.state);
    for (i, (row, old)) in rows.iter().zip(expected).enumerate() {
        assert_eq!(row["kind"], "rand");
        assert_eq!(row["before"], rng.state, "repeat CRT gap at {i}");
        assert_eq!(row["position"], rng.draws);
        assert_eq!(row["returnRva"], old["returnRva"], "repeat caller at {i}");
        rng.next_u15();
        if i == 11 {
            assert_eq!(rng.state, 0x0EA7_9703);
            assert_eq!(rng.draws, 4929);
        }
    }
    assert_eq!(capture["finalState"], rng.state);
    assert_eq!(rng.state, 0xF485_D421);
    assert_eq!(rng.draws, 4959);
    assert_eq!(case().1.state().rng.state, rng.state);
}

#[test]
fn reported_first_round_party_order() {
    let (observed, mut battle) = case();
    let expected = ids(&observed, "first_round_party", &battle);
    let mut routine = CapturedReplay::default();
    while battle.state().round == 1 && battle.state().outcome == Outcome::Ongoing {
        routine.step(&mut battle).unwrap();
    }
    assert_eq!(routine.first_round, expected);
}

#[test]
fn reported_first_round_display_without_night_talent() {
    let (observed, battle) = case();
    assert!(
        battle.state().actors[..3]
            .iter()
            .all(|a| a.statuses.is_empty())
    );
    let state = serde_json::to_value(battle.state()).unwrap();
    let display: Vec<_> = state["queue"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| ["throne", "pirro", "scaracci", "grunt", "grunt"][id.as_u64().unwrap() as usize])
        .collect();
    assert_eq!(serde_json::json!(display), observed["first_round_display"]);
}

#[test]
fn reported_pirro_damage_and_throne_broken_hit_without_night_talent() {
    let (observed, mut battle) = case();
    let mut routine = CapturedReplay::default();
    let pirro = routine.step(&mut battle).unwrap();
    let throne = routine.step(&mut battle).unwrap();
    let hits = |report: &StepReport| -> Vec<u32> {
        report
            .events
            .iter()
            .filter_map(|e| match e {
                Event::Damage { amount, .. } => Some(*amount),
                _ => None,
            })
            .collect()
    };
    assert_eq!(pirro.actor, battle.actor_id("pirro").unwrap());
    assert_eq!(throne.actor, battle.actor_id("throne").unwrap());
    assert_eq!(
        serde_json::json!(hits(&pirro)),
        observed["damage_reports"][0]["hits"]
    );
    // Only this hit matches. Throné's first hit still predicts 33 versus the
    // reported 31; preserve the report and track that discrepancy in the doc.
    assert_eq!(
        serde_json::json!(hits(&throne)[1]),
        observed["damage_reports"][1]["hits"][1]
    );
}

#[test]
fn reported_after_area_reset_state() {
    let (observed, mut battle) = case();
    let expected = u32::from_str_radix(
        observed["after_area_reset"]
            .as_str()
            .unwrap()
            .trim_start_matches("0x"),
        16,
    )
    .unwrap();
    let mut routine = CapturedReplay::default();
    while battle.state().outcome == Outcome::Ongoing {
        routine.step(&mut battle).unwrap();
    }
    assert_eq!(battle.state().rng.state, expected);
}

#[test]
fn captured_replay_matches_every_crt_call_and_action_boundary() {
    let capture: serde_json::Value =
        serde_json::from_str(include_str!("../data/source/live-opening-4929-trace.json")).unwrap();
    assert_eq!(
        capture["finish"],
        "Detached with original debug registers restored"
    );
    let rows = capture["records"].as_array().unwrap();
    let (_, mut battle) = case();
    let mut routine = CapturedReplay::default();
    // Last call before the next actor's input; final includes both field resets.
    for boundary in [52, 65, 80, 110, 135, 154] {
        let report = routine.step(&mut battle).unwrap();
        assert_eq!(report.rng_after.draws, boundary);
        let mut expected = CrtRng::new(0x0EA7_9703);
        expected.advance(boundary);
        assert_eq!(report.rng_after, expected);
    }
    let crt = battle.state().opening_crt.as_ref().unwrap();
    let mut observed = rows.iter();
    let mut rng = CrtRng::new(capture["initialState"].as_u64().unwrap() as u32);
    for span in crt.trace.as_ref().unwrap() {
        assert_eq!(span.before, rng);
        for index in 0..span.after.draws - span.before.draws {
            let row = observed.next().expect("simulator has an extra call");
            assert_eq!(row["kind"], "rand");
            assert_eq!(row["before"], rng.state, "{} at {}", span.reason, rng.draws);
            let caller = match span.reason {
                "actor_initialization" if span.actor.unwrap().0 >= 3 => {
                    ["0x30D7424", "0x30D7604", "0x30D7065"][index as usize]
                }
                "actor_initialization" | "round_timing_revision" | "critical_hit_and_damage" => {
                    "0x30D7065"
                }
                "battle_start_speaker"
                | "field_encounter_reset_b_then_a"
                | "post_battle_event_encounter_reset_b_then_a" => "0x30D7424",
                "damage_notification_timer"
                | "break_slow_timer"
                | "broken_hit_slow_timer"
                | "round_bp_timer" => "0x2C52E57",
                "guard_disease" if index > 0 => "0x30D7604",
                "current_order_shuffle"
                | "next_order_shuffle"
                | "normal_accuracy"
                | "battle_start_voice"
                | "order_voice"
                | "boost_voice"
                | "attack_voice"
                | "last_attack_voice"
                | "guard_disease"
                | "guard_voice"
                | "enemy_drop" => "0x2C57B26",
                other => panic!("unclassified callback {other}"),
            };
            assert_eq!(row["returnRva"], caller, "{} at {}", span.reason, rng.draws);
            rng.next_u15();
        }
        assert_eq!(span.after, rng);
    }
    assert!(observed.next().is_none(), "game has unmodeled calls");
    assert_eq!(rng.draws, 154);
    assert_eq!(capture["finalState"], rng.state);
    let after = crt.after_battle.as_ref().unwrap();
    assert_eq!(after.rng_before_rewards.draws, 148);
    assert_eq!(after.rng_after_rewards.draws, 150);
    assert_eq!(after.rng_after_field_reset.draws, 152);
    assert_eq!(after.rng_after_post_battle_event.draws, 154);
}
