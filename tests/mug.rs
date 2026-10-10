use ot2_battle_engine::{ActorId, CrtRng, Event, Outcome, mug};

#[test]
fn round_two_boost_inputs_replay_combat_and_preserve_earlier_menus() {
    let mut entry = CrtRng::new(0);
    entry.advance(6165);
    let normal = mug::predict(entry).unwrap();
    let explicit = mug::predict_with_boost_inputs(entry, Some(&[1, 2])).unwrap();
    assert_eq!(normal.after, explicit.after);
    let adjusted = mug::predict_with_boost_inputs(entry, Some(&[1, 0, 1, 2])).unwrap();
    assert_eq!(adjusted.outcome, Outcome::Victory);
    assert_eq!(adjusted.menus[0].rng, normal.menus[0].rng);
    assert_eq!(adjusted.menus[1].rng, normal.menus[1].rng);
    assert_ne!(adjusted.after, normal.after);
    assert!(mug::predict_with_boost_inputs(entry, Some(&[1])).is_err());
    assert!(mug::predict_with_boost_inputs(entry, Some(&[1, 2, 3])).is_err());
}

#[test]
fn mug_initialization_matches_retail_call_sequence() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../data/source/cape-cold-mug-first-menu-live.json"
    )).unwrap();
    let calls = fixture["calls"].as_array().unwrap();
    let mut rng = CrtRng::new(0);
    rng.advance(6165);
    let p = mug::predict(rng).unwrap();
    let expected = [
        ("actor_initialization", 6165, 6166),
        ("actor_initialization", 6166, 6168),
        ("current_order_shuffle", 6168, 6169),
        ("battle_start_speaker", 6169, 6170),
        ("battle_start_voice", 6170, 6171),
        ("study_foe_weakness", 6171, 6172),
        ("next_order_shuffle", 6172, 6173),
        ("order_voice", 6173, 6192),
    ];
    for (span, (reason, start, end)) in p.trace.iter().zip(expected) {
        assert_eq!((span.reason, span.before.draws, span.after.draws), (reason, start, end));
        assert_eq!(span.before, rng);
        while rng.draws < end {
            let call = &calls[(rng.draws - 6154) as usize];
            assert_eq!(call["position"], rng.draws);
            assert_eq!(call["before"], rng.state);
            rng.next_u15();
        }
        assert_eq!(span.after, rng);
    }
    // These are the native integer-range BP draw and Blueprint timing draw;
    // no ordinary enemy RandomInteger sprite-frame call occurs between them.
    assert_eq!(calls[12]["return_rva"], "0x30D7604");
    assert!(calls[13]["node"].as_str().unwrap().ends_with("InitializeEnemy"));
    assert!(calls[17]["node"].as_str().unwrap().ends_with("OpenWeakIcon"));
    assert_eq!(p.menus[0].rng, CrtRng { state: 0x3c165bb0, draws: 6192 });
}

#[test]
fn first_mug_fixed_routine_covers_both_damage_branches() {
    let mut branches = [false; 2];
    for seed in 0..128 {
        let p = mug::predict(CrtRng::new(seed)).unwrap();
        assert_eq!(p.outcome, Outcome::Victory);
        branches[usize::from(p.survived_round_two)] = true;
        let damage: Vec<_> = p
            .steps
            .iter()
            .flat_map(|s| &s.events)
            .filter_map(|e| match e {
                Event::Damage {
                    actor,
                    target,
                    amount,
                    ..
                } => {
                    assert_eq!((*actor, *target), (ActorId(0), ActorId(1)));
                    Some(*amount)
                }
                _ => None,
            })
            .collect();
        assert!((52..=54).contains(&damage[0]));
        assert!((225..=234).contains(&damage[1]));
        assert_eq!(p.survived_round_two, damage[0] + damage[1] < 280);
        assert_eq!(damage.len(), if p.survived_round_two { 3 } else { 2 });
        let variance: Vec<_> = p
            .trace
            .iter()
            .filter(|s| s.reason == "critical_hit_and_damage")
            .collect();
        assert_eq!(variance[1].after.draws - variance[0].after.draws, 29);
        let buffs: Vec<_> = p
            .trace
            .iter()
            .filter(|s| s.reason == "npc_status_application")
            .collect();
        assert_eq!(buffs.len(), if p.survived_round_two { 2 } else { 0 });
        assert!(buffs.iter().all(|s| s.after.draws - s.before.draws == 21));
    }
    assert_eq!(branches, [true, true]);
}

#[test]
fn cape_cold_6154_retail_battle_prediction() {
    let mut entry = CrtRng::new(0);
    entry.advance(6165);
    let p = mug::predict(entry).unwrap();
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../data/source/cape-cold-mug-completion-live.json"
    )).unwrap();
    let calls = fixture["calls"].as_array().unwrap();
    let mut replay = entry;
    for span in &p.trace {
        assert_eq!(span.before, replay, "{}", span.reason);
        while replay.draws < span.after.draws {
            let call = &calls[(replay.draws - 6165) as usize];
            assert_eq!(call["position"], replay.draws);
            assert_eq!(call["before"], replay.state);
            replay.next_u15();
        }
        assert_eq!(span.after, replay);
    }
    let ending: Vec<_> = p.trace.iter().filter(|s| s.before.draws >= 6289)
        .map(|s| (s.reason, s.before.draws, s.after.draws)).collect();
    assert_eq!(ending, [
        ("normal_accuracy", 6289, 6290), ("attack_voice", 6290, 6291),
        ("critical_hit_and_damage", 6291, 6292), ("damage_notification_timer", 6292, 6293),
        ("kill_voice", 6293, 6294), ("break_slow_timer", 6294, 6295),
        ("victory_voice", 6295, 6296), ("enemy_drop", 6296, 6297),
    ]);
    assert!(calls[6293-6165]["node"].as_str().unwrap().ends_with("PlayBattleVoice"));
    assert_eq!(calls[6294-6165]["return_rva"], "0x2C52E57");
    assert!(calls[6296-6165]["node"].as_str().unwrap().ends_with("LotteryDropItem"));
    assert_eq!(
        p.menus.iter().map(|m| m.rng.draws).collect::<Vec<_>>(),
        [6192, 6218, 6289]
    );
    let damages: Vec<_> = p
        .steps
        .iter()
        .flat_map(|s| &s.events)
        .filter_map(|e| match e {
            Event::Damage { amount, .. } => Some(*amount),
            _ => None,
        })
        .collect();
    assert_eq!(damages, [53, 226, 11]);
    assert_eq!(p.after.draws, 6297);
    let mut expected = CrtRng::new(0);
    expected.advance(6297);
    assert_eq!(p.after, expected);
}
