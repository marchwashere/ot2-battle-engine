use ot2_battle_engine::{
    ActorId, Battle, BattleConfig, Command, CrtRng, Event, ItemCatalog, Outcome,
    pirro::PirroRoutine,
};
fn battle(seed: u32, trace: bool) -> Battle {
    Battle::new_pirro(
        BattleConfig::throne_pirro(seed).unwrap(),
        ItemCatalog::builtin().unwrap(),
        trace,
    )
    .unwrap()
}
#[test]
fn reported_first_menu_uses_fixed_throne_speaker_without_a_random_selection() {
    // User replay: seed 0, pre-cutscene advance 5595, first menu advance 5631.
    // The serial cutscene has no pre-initialization draws (assistant coverage).
    let mut entry = CrtRng::new(0);
    entry.advance(5595);
    let b = battle(entry.state, true);
    assert_eq!(b.current_actor(), Some(ActorId(0)));
    assert_eq!(5595 + b.state().rng.draws, 5631);
    let mut expected = CrtRng::new(entry.state);
    expected.advance(36);
    assert_eq!(b.state().rng, expected);
    let trace = b
        .state()
        .opening_crt
        .as_ref()
        .unwrap()
        .trace
        .as_ref()
        .unwrap();
    let speaker = trace
        .iter()
        .find(|t| t.reason == "battle_start_speaker")
        .unwrap();
    assert_eq!(speaker.actor, Some(ActorId(0)));
    assert_eq!(speaker.before, speaker.after);
    let voice = trace
        .iter()
        .find(|t| t.reason == "battle_start_voice")
        .unwrap();
    assert_eq!(voice.after.draws - voice.before.draws, 1);
}
#[test]
fn pirro_full_bp_does_not_run_normal_enemy_boost_lottery() {
    // Retail report: seed 0 +5595, no extra boosts, R7 menu at5798.
    let mut entry = CrtRng::new(0);
    entry.advance(5595);
    let mut b = battle(entry.state, true);
    let mut policy = PirroRoutine::default();
    while b.state().round < 7 {
        policy.step(&mut b).unwrap();
    }
    assert_eq!(b.current_actor(), Some(ActorId(1)));
    let boss = &b.state().actors[1];
    assert_eq!(boss.resources.bp, boss.enemy_charge_max);
    let bp = boss.resources.bp;
    let threshold = boss.enemy_charge_max;
    policy.step(&mut b).unwrap();
    assert_eq!(b.current_actor(), Some(ActorId(0)));
    assert_eq!(b.state().actors[1].resources.bp, bp);
    assert_eq!(b.state().actors[1].enemy_charge_max, threshold);
    assert_eq!(5595 + b.state().rng.draws, 5798);
    let trace = b
        .state()
        .opening_crt
        .as_ref()
        .unwrap()
        .trace
        .as_ref()
        .unwrap();
    let ai = trace
        .iter()
        .rfind(|s| s.reason == "pirro_ai_and_target")
        .unwrap();
    assert_eq!(ai.before, ai.after); // Resolution targets self; no target draw either.
    let phase = trace
        .iter()
        .rfind(|s| s.reason == "pirro_phase_frame")
        .unwrap();
    assert_eq!(phase.after.draws - phase.before.draws, 1);
}
#[test]
fn routine_replays_both_round_eight_branches_and_preserves_crt_continuity() {
    let template = battle(0, false);
    let mut branches = [0usize; 2];
    let mut no_grape_latent_branches = [0usize; 2];
    let mut wins = 0;
    for seed in 0..1024 {
        let mut b = template.restarted(seed);
        let mut p = PirroRoutine::default();
        let mut steps = 0;
        while b.state().outcome == Outcome::Ongoing {
            let round = b.state().round;
            let before = b.state().clone();
            let latent_ready = b.can_use_latent();
            let r = p
                .step(&mut b)
                .unwrap_or_else(|e| panic!("seed {seed} R{round}: {e}"));
            assert_eq!(before.rng, r.rng_before);
            let mut expected = before.rng;
            expected.advance(r.rng_after.draws - before.rng.draws);
            assert_eq!(expected, r.rng_after);
            if round == 8
                && r.actor == ActorId(0)
                && p.heal_before_round_eight == Some(false)
                && before.pirro.as_ref().unwrap().double_action_remaining == 0
            {
                let activated = r
                    .events
                    .iter()
                    .any(|e| matches!(e, Event::LatentActivated { .. }));
                assert_eq!(activated, latent_ready, "seed {seed}");
                assert_eq!(p.latent_delayed, !latent_ready, "seed {seed}");
                no_grape_latent_branches[usize::from(p.latent_delayed)] += 1;
            }
            if before.pirro.as_ref().unwrap().double_action_remaining == 2 {
                panic!("activation should execute its first action atomically")
            }
            if (p.wind_pending || p.ice_pending) && r.actor == ActorId(0) {
                assert_eq!(b.current_actor(), Some(ActorId(0)));
                assert_eq!(b.state().round, round);
            }
            steps += 1;
            assert!(steps < 40);
        }
        if b.state().outcome == Outcome::Victory {
            wins += 1;
        }
        if b.state().outcome == Outcome::Defeat {
            continue;
        }
        assert_eq!(b.state().outcome, Outcome::Victory, "seed {seed}");
        branches[usize::from(p.heal_before_round_eight.unwrap())] += 1;
        assert_eq!(b.item_count("shadow_soulstone").unwrap(), 0);
        assert_eq!(b.item_count("wind_soulstone").unwrap(), 0);
        assert_eq!(b.item_count("ice_soulstone").unwrap(), 0);
        assert_eq!(b.item_count("light_soulstone").unwrap(), 0);
        assert_eq!(
            b.item_count("healing_grape").unwrap(),
            u32::from(!p.heal_before_round_eight.unwrap())
        );
        assert_eq!(b.item_count("inspiriting_plum_basket").unwrap(), 1);
    }
    assert!(branches.iter().all(|n| *n > 0));
    assert!(no_grape_latent_branches.iter().all(|n| *n > 0));
    println!("{wins}/1024 victories; branches {branches:?}");
}
#[test]
fn early_critical_can_trigger_second_phase_before_the_planned_heal() {
    let mut b = battle(0xAA7C3306, false); // Retail: seed 0 + 5678.
    let mut p = PirroRoutine::default();
    let mut critical = false;
    while b.state().outcome == Outcome::Ongoing {
        let round = b.state().round;
        let r = p.step(&mut b).unwrap();
        critical |= round == 4
            && r.actor == ActorId(0)
            && r.events
                .iter()
                .any(|e| matches!(e, Event::Damage { critical: true, .. }));
    }
    assert!(critical);
    assert_eq!(b.state().outcome, Outcome::Defeat);
    assert_eq!(b.state().round, 7);
    assert_eq!(p.heal_before_round_eight, None);
}
#[test]
fn round_three_boost_path_replaces_normal_inputs_and_rejects_invalid_path_atomically() {
    let mut b = battle(7, true);
    let mut policy = PirroRoutine::default();
    while b.state().round != 3 || b.current_actor() != Some(ActorId(0)) {
        policy.step(&mut b).unwrap();
    }
    let mut ordinary = b.clone();
    let expected = policy.clone().step(&mut ordinary).unwrap();
    let actual = policy
        .clone()
        .step_with_boost_inputs(&mut b.clone(), &[1, 2, 3])
        .unwrap();
    assert_eq!(actual, expected);
    let before = b.state().clone();
    let before_policy = serde_json::to_value(&policy).unwrap();
    assert!(
        policy
            .step_with_boost_inputs(&mut b, &[1, 2, 3, 2])
            .is_err()
    );
    assert_eq!(b.state(), &before);
    assert_eq!(serde_json::to_value(&policy).unwrap(), before_policy);
    let preview = b.preview_boost_inputs(&[1, 2, 3, 2, 3]).unwrap();
    assert_eq!(b.state(), &before);
    assert_eq!(preview.rng.draws - before.rng.draws, 10);
    let report = policy
        .step_with_boost_inputs(&mut b, &[1, 2, 3, 2, 3])
        .unwrap();
    assert_eq!(report.rng_before, before.rng);
    assert_eq!(
        b.state().actors[0].resources.bp,
        ordinary.state().actors[0].resources.bp
    );
    let voice_draws: u64 = b
        .state()
        .opening_crt
        .as_ref()
        .unwrap()
        .trace
        .as_ref()
        .unwrap()
        .iter()
        .filter(|s| s.before.draws >= before.rng.draws && s.reason == "boost_voice")
        .map(|s| s.after.draws - s.before.draws)
        .sum();
    // This boss has a 50% special max-boost voice before the generic voice:
    // the first 2->3 uses four draws here, the repeated 2->3 uses three.
    assert_eq!(voice_draws, 10);
}
#[test]
fn unavailable_latent_is_atomic() {
    let mut b = battle(0, true);
    let before = b.state().clone();
    assert!(
        b.step_player(Command::Latent {
            command: Box::new(Command::Boost {
                id: "attack_sword".into(),
                target: Some(ActorId(1)),
                bp: 3
            })
        })
        .is_err()
    );
    assert_eq!(&before, b.state());
}
#[test]
fn unavailable_round_eight_latent_moves_to_round_nine_soulstones() {
    let mut entry = CrtRng::new(0);
    entry.advance(5595);
    let mut b = battle(entry.state, true);
    let mut routine = PirroRoutine::default();
    while b.state().round != 8 || b.current_actor() != Some(ActorId(0)) {
        routine.step(&mut b).unwrap();
    }
    assert!(b.state().pirro.as_ref().unwrap().latent_unlocked);
    assert_eq!(b.state().actors[0].resources.latent, 55);
    assert!(!b.can_use_latent());
    let sword = Command::Boost {
        id: "attack_sword".into(),
        target: Some(ActorId(1)),
        bp: 3,
    };
    let before = b.state().clone();
    assert!(
        b.step_player(Command::Latent {
            command: Box::new(sword.clone())
        })
        .is_err()
    );
    assert_eq!(b.state(), &before);
    let report = routine.step(&mut b).unwrap();
    assert!(routine.latent_delayed);
    assert!(!routine.latent_used);
    assert!(
        !report
            .events
            .iter()
            .any(|e| matches!(e, Event::LatentActivated { .. }))
    );
    assert_eq!(
        report
            .events
            .iter()
            .filter(|e| matches!(
                e,
                Event::Damage {
                    actor: ActorId(0),
                    ..
                }
            ))
            .count(),
        4
    );
    assert!(report.events.iter().any(|e| matches!(
        e,
        Event::Broken {
            actor: ActorId(1),
            ..
        }
    )));
    assert_eq!(b.state().actors[0].resources.latent, 80);
    assert_eq!(b.state().round, 9);
    assert!(b.can_use_latent());
    assert_eq!(5595 + b.state().rng.draws, 5832);
    let wind = routine.step(&mut b).unwrap();
    assert!(
        wind.events
            .iter()
            .any(|e| matches!(e, Event::LatentActivated { .. }))
    );
    assert!(
        wind.events
            .iter()
            .any(|e| matches!(e, Event::Action { name, .. } if name=="wind_soulstone"))
    );
    assert!(routine.ice_pending);
    assert_eq!(b.state().round, 9);
    assert_eq!(b.current_actor(), Some(ActorId(0)));
    assert!(!b.can_use_latent());
    assert_eq!(5595 + b.state().rng.draws, 5836);
    let ice = routine.step(&mut b).unwrap();
    assert!(
        ice.events
            .iter()
            .any(|e| matches!(e, Event::Action { name, .. } if name=="ice_soulstone"))
    );
    assert!(
        !ice.events
            .iter()
            .any(|e| matches!(e, Event::LatentActivated { .. }))
    );
    assert!(!routine.ice_pending);
    assert_eq!(b.item_count("wind_soulstone").unwrap(), 0);
    assert_eq!(b.item_count("ice_soulstone").unwrap(), 0);
    assert_eq!(b.state().round, 10);
    assert_eq!(b.current_actor(), Some(ActorId(1)));
    let hp = b.state().actors[0].resources.hp;
    let mercy = routine.step(&mut b).unwrap();
    // Retail: Pierce Through (穿心), a small hit, then menu at5892.
    // HP_RATIO_DAMAGE is CalcType 2; the forced-miss status only gates type 0.
    assert!(mercy.events.iter().any(|e| matches!(
        e, Event::Action { name, .. } if name == "ABI_BOS_THI_C01_010_008"
    )));
    assert!(mercy.events.iter().any(|e| matches!(
        e,
        Event::Damage {
            actor: ActorId(1),
            target: ActorId(0),
            amount: 2,
            ..
        }
    )));
    assert!(!mercy.events.iter().any(|e| matches!(e, Event::Miss { .. })));
    assert_eq!(b.state().actors[0].resources.hp, hp - 2);
    assert_eq!(b.current_actor(), Some(ActorId(0)));
    assert_eq!(5595 + b.state().rng.draws, 5892);
    let trace = b
        .state()
        .opening_crt
        .as_ref()
        .unwrap()
        .trace
        .as_ref()
        .unwrap();
    let hit = trace
        .iter()
        .rfind(|t| t.reason == "hp_ratio_accuracy")
        .unwrap();
    assert_eq!(
        (5595 + hit.before.draws, 5595 + hit.after.draws),
        (5865, 5866)
    );
    let reaction = trace.iter().rfind(|t| t.reason == "damage_voice").unwrap();
    assert_eq!(reaction.actor, Some(ActorId(0)));
    assert_eq!(reaction.after.draws - reaction.before.draws, 3);
    while b.state().outcome == Outcome::Ongoing {
        routine.step(&mut b).unwrap();
    }
    assert_eq!(b.state().outcome, Outcome::Victory);
    assert_eq!(b.state().round, 10);
    assert_eq!(5595 + b.state().rng.draws, 5900);
}
#[test]
fn active_ability_camera_gates_broken_hit_timer_but_not_new_break_timer() {
    let mut config = BattleConfig::throne_pirro(0).unwrap();
    config.enemies[0].combat.as_mut().unwrap().shields = 1;
    let mut b = Battle::new_pirro(config, ItemCatalog::builtin().unwrap(), true).unwrap();
    while b.current_actor() != Some(ActorId(0)) {
        b.step_enemy().unwrap();
    }
    b.step_player(Command::Boost {
        id: "ABI_SET_THI_020".into(),
        target: Some(ActorId(1)),
        bp: 0,
    })
    .unwrap();
    assert!(b.state().actors[1].broken());
    assert_eq!(b.current_actor(), Some(ActorId(0)));
    assert!(
        b.state()
            .opening_crt
            .as_ref()
            .unwrap()
            .trace
            .as_ref()
            .unwrap()
            .iter()
            .any(|s| s.reason == "break_slow_timer")
    );

    // Start all three commands from the same genuinely broken target. The
    // item and ability share a gate; normal attacks must still draw the timer.
    for (command, timer_count) in [
        (
            Command::Item {
                id: "shadow_soulstone".into(),
                target: None,
            },
            0,
        ),
        (
            Command::Boost {
                id: "ABI_SET_THI_020".into(),
                target: Some(ActorId(1)),
                bp: 0,
            },
            0,
        ),
        (
            Command::Boost {
                id: "attack_sword".into(),
                target: Some(ActorId(1)),
                bp: 0,
            },
            1,
        ),
    ] {
        let mut trial = b.clone();
        let report = trial.step_player(command).unwrap();
        assert!(report.events.iter().any(|e| matches!(
            e,
            Event::Damage {
                target: ActorId(1),
                ..
            }
        )));
        let timers = trial
            .state()
            .opening_crt
            .as_ref()
            .unwrap()
            .trace
            .as_ref()
            .unwrap()
            .iter()
            .filter(|s| {
                s.before.draws >= report.rng_before.draws && s.reason == "broken_hit_slow_timer"
            })
            .count();
        assert_eq!(timers, timer_count);
    }
}
#[test]
fn soulstone_uses_no_variance_draw_and_holdout_preserves_displayed_damage() {
    let mut rng = CrtRng::new(0);
    rng.advance(5595);
    let mut b = battle(rng.state, true);
    let mut p = PirroRoutine::default();
    let mut stones = vec![];
    while b.state().outcome == Outcome::Ongoing {
        let r = p.step(&mut b).unwrap();
        if r.events
            .iter()
            .any(|e| matches!(e,Event::Action{name,..} if name.ends_with("soulstone")))
        {
            stones.extend(r.events.iter().filter_map(|e| {
                if let Event::Damage { amount, .. } = e {
                    Some(*amount)
                } else {
                    None
                }
            }));
            assert!(
                !b.state()
                    .opening_crt
                    .as_ref()
                    .unwrap()
                    .trace
                    .as_ref()
                    .unwrap()
                    .iter()
                    .any(|t| t.before.draws >= r.rng_before.draws
                        && t.after.draws <= r.rng_after.draws
                        && t.reason == "critical_hit_and_damage")
            );
        }
    }
    assert_eq!(stones, [400, 260, 400, 400]);
    let mut trace_rng = CrtRng::new(rng.state);
    for span in b
        .state()
        .opening_crt
        .as_ref()
        .unwrap()
        .trace
        .as_ref()
        .unwrap()
    {
        assert_eq!(span.before, trace_rng, "{}", span.reason);
        trace_rng.advance(span.after.draws - span.before.draws);
        assert_eq!(span.after, trace_rng, "{}", span.reason);
    }
    assert_eq!(trace_rng, b.state().rng);
}
