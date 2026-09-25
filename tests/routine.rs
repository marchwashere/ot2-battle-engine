use ot2_battle_engine::{routine::ThroneOpeningRoutine, *};

fn opening(seed: u32, trace: bool) -> Battle {
    let mut config = BattleConfig::from_json(include_str!("../data/throne_opening.json")).unwrap();
    config.seed = seed;
    Battle::new_opening(config, ItemCatalog::builtin().unwrap(), trace).unwrap()
}

#[test]
fn supplied_routine_wins_across_shuffled_orders_and_defends_the_booster() {
    let config = BattleConfig::from_json(include_str!("../data/throne_opening.json")).unwrap();
    let mut defended = 0;
    let mut enemy_actions = 0;
    let mut orders = std::collections::BTreeSet::new();
    for seed in 0..1024 {
        let mut c = config.clone();
        c.seed = seed;
        let mut battle = Battle::new_opening(c, ItemCatalog::builtin().unwrap(), false).unwrap();
        let mut routine = ThroneOpeningRoutine::default();
        let mut replay = battle.clone();
        let mut replay_routine = routine.clone();
        for _ in 0..16 {
            if battle.state().outcome != Outcome::Ongoing {
                break;
            }
            let result = routine
                .step(&mut battle)
                .unwrap_or_else(|e| panic!("seed {seed}: {e}"));
            assert_eq!(result, replay_routine.step(&mut replay).unwrap());
            enemy_actions += usize::from(battle.side(result.actor) == Some(Side::Enemy));
        }
        assert_eq!(battle.state().outcome, Outcome::Victory, "seed {seed}");
        assert_eq!(battle.state().round, 2);
        assert_eq!(routine.first_round.len(), 3);
        assert_eq!(routine.second_round_attackers.len(), 2);
        assert!(
            !routine
                .second_round_attackers
                .contains(&routine.first_round[1])
        );
        assert_eq!(battle.state(), replay.state());
        orders.insert(routine.first_round.clone());
        defended += u32::from(routine.second_round_defended);
    }
    assert_eq!(orders.len(), 6);
    assert!(defended > 0 && defended < 1024);
    assert!(enemy_actions > 0);
}

#[test]
fn trace_covers_every_draw_and_tracing_does_not_change_results() {
    let mut traced = opening(123, true);
    let mut fast = opening(123, false);
    let mut a = ThroneOpeningRoutine::default();
    let mut b = a.clone();
    let mut attacks = Vec::new();
    while traced.state().outcome == Outcome::Ongoing {
        let report = a.step(&mut traced).unwrap();
        assert_eq!(report, b.step(&mut fast).unwrap());
        attacks.push(report);
    }
    let crt = traced.state().opening_crt.as_ref().unwrap();
    let mut expected = CrtRng::new(123);
    for span in crt.trace.as_ref().unwrap() {
        assert_eq!(span.before, expected, "gap before {}", span.reason);
        while expected.draws < span.after.draws {
            expected.next_u15();
        }
        assert_eq!(span.after, expected, "{}", span.reason);
    }
    assert_eq!(expected, traced.state().rng);
    // Fixture describes this simulator/source projection, not a live-game trace.
    assert_eq!(
        expected,
        CrtRng {
            state: 3_423_024_866,
            draws: 145
        }
    );
    let after = crt.after_battle.as_ref().unwrap();
    assert_eq!(
        after.rng_after_rewards.draws - after.rng_before_rewards.draws,
        2
    );
    assert_eq!(
        after.rng_after_field_reset.draws - after.rng_after_rewards.draws,
        2
    );
    assert_eq!(after.encounter_steps, 798);
    assert_eq!(
        after,
        fast.state()
            .opening_crt
            .as_ref()
            .unwrap()
            .after_battle
            .as_ref()
            .unwrap()
    );
    // A KO on hit one still leaves two authored damage/timer calls.
    let killing_action = &attacks[3];
    assert_eq!(
        killing_action
            .events
            .iter()
            .filter(|e| matches!(e, Event::Damage { .. }))
            .count(),
        3
    );
    assert_eq!(
        killing_action
            .events
            .iter()
            .filter(|e| matches!(e, Event::KnockedOut { .. }))
            .count(),
        1
    );
    assert_eq!(traced.restarted(123).state(), opening(123, true).state());
}

#[test]
fn night_talent_moves_existing_first_order_then_buffs_the_preview() {
    let mut config = BattleConfig::from_json(include_str!("../data/throne_opening.json")).unwrap();
    config.night_buff_turns = 0;
    let unbuffed = Battle::new(config.clone(), ItemCatalog::builtin().unwrap()).unwrap();
    let original = serde_json::to_value(unbuffed.state()).unwrap()["queue"].clone();
    // Nine initialization calls, eight shuffle calls: source rows sort before
    // the talent applies. This order is deliberately not a sort by buffed AGI.
    assert_eq!(original, serde_json::json!([0, 3, 4, 1, 2]));
    config.night_buff_turns = 3;
    let buffed = Battle::new(config, ItemCatalog::builtin().unwrap()).unwrap();
    assert_eq!(
        serde_json::to_value(buffed.state()).unwrap()["queue"],
        serde_json::json!([0, 3, 1, 2, 4])
    );
    assert_ne!(unbuffed.state().next_queue, buffed.state().next_queue);
    assert_eq!(unbuffed.state().rng, buffed.state().rng); // direct talent costs no CRT
    for actor in &buffed.state().actors[..3] {
        assert!(actor.statuses.iter().all(|s| s.turns == 3));
        assert_eq!(actor.statuses.len(), 3);
    }
}

#[test]
fn routine_and_crt_state_roll_back_rejected_actions() {
    let mut battle = opening(123, true);
    let before = battle.state().clone();
    assert!(battle.step_player(Command::Wait).is_err());
    assert!(
        battle
            .step_player(Command::Skill {
                id: "attack_sword".into(),
                target: Some(ActorId(3))
            })
            .is_err()
    );
    assert!(
        battle
            .step_player(Command::Skill {
                id: "ABI_SET_THI_020".into(),
                target: Some(ActorId(3))
            })
            .is_err()
    );
    assert_eq!(battle.state(), &before);
    let mut routine = ThroneOpeningRoutine {
        first_round: vec![ActorId(0)],
        ..Default::default()
    };
    let policy_before = routine.clone();
    assert!(routine.step(&mut battle).is_err());
    assert_eq!(routine, policy_before);
    assert_eq!(battle.state(), &before);
}

#[test]
fn defend_preserves_captured_guard_cost_and_next_actor_voice_cost() {
    for seed in 0..64 {
        let mut battle = opening(seed, true);
        let mut routine = ThroneOpeningRoutine::default();
        while battle.state().outcome == Outcome::Ongoing {
            let result = routine.step(&mut battle).unwrap();
            if result
                .events
                .iter()
                .any(|e| matches!(e, Event::Action { name, .. } if name == "defend"))
            {
                let trace = battle
                    .state()
                    .opening_crt
                    .as_ref()
                    .unwrap()
                    .trace
                    .as_ref()
                    .unwrap();
                let guard = trace
                    .iter()
                    .rev()
                    .find(|s| s.reason == "guard_disease")
                    .unwrap();
                assert_eq!(guard.after.draws - guard.before.draws, 21);
                let next_cost = if battle.current_actor() == Some(ActorId(0)) {
                    17
                } else {
                    3
                };
                assert_eq!(
                    result.rng_after.draws - result.rng_before.draws,
                    22 + next_cost
                );
                return;
            }
        }
    }
    panic!("no Defend branch exercised");
}
