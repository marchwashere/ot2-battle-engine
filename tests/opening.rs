use ot2_battle_engine::{
    ai::{AiAction, AiRule, AiState, AiTarget, BossAi, Condition, Selection},
    combat::{self, Attack, DamageInput, DamageKind, StatusKind, Weapon},
    *,
};

fn config() -> BattleConfig {
    BattleConfig::from_json(include_str!("../data/throne_opening.json")).unwrap()
}
fn make(c: BattleConfig) -> Battle {
    Battle::new(c, ItemCatalog::builtin().unwrap()).unwrap()
}
fn boost(id: &str, target: Option<ActorId>, bp: u8) -> Command {
    Command::Boost {
        id: id.into(),
        target,
        bp,
    }
}
fn until(b: &mut Battle, id: ActorId) {
    for _ in 0..100 {
        if b.current_actor() == Some(id) {
            return;
        }
        let actor = b.current_actor().expect("ongoing test battle");
        if b.side(actor) == Some(Side::Enemy) {
            b.step_enemy().unwrap();
        } else {
            b.step_player(Command::Wait).unwrap();
        }
    }
    panic!("actor did not get a turn");
}
fn durable() -> BattleConfig {
    let mut c = config();
    for p in &mut c.players {
        p.stats.max_hp = 10000;
        p.stats.max_sp = 1000;
        p.resources = Some(Resources {
            hp: 10000,
            sp: 1000,
            bp: 3,
            latent: 0,
        });
        p.combat.as_mut().unwrap().critical = 0;
    }
    for e in &mut c.enemies {
        e.stats.max_hp = 10000;
        e.resources = None;
    }
    c
}
fn first_opponent() -> BossAi {
    BossAi {
        schema_version: 1,
        initial_state: "loop".into(),
        states: vec![AiState {
            id: "loop".into(),
            rules: vec![AiRule {
                condition: Condition::Always,
                next_state: None,
                selection: Selection::Sequence {
                    actions: vec![AiAction {
                        skill: "ABI_ENE_0041".into(),
                        target: AiTarget::FirstOpponent,
                    }],
                },
            }],
        }],
    }
}

#[test]
fn opening_loadout_comes_from_the_selected_source_rows() {
    let c = config();
    assert_eq!(
        c.players
            .iter()
            .map(|p| (&*p.id, p.stats.max_hp, p.stats.max_sp))
            .collect::<Vec<_>>(),
        vec![
            ("throne", 250, 40),
            ("pirro", 275, 45),
            ("scaracci", 237, 35)
        ]
    );
    assert_eq!(c.enemies.len(), 2);
    for e in &c.enemies {
        assert_eq!(e.stats.max_hp, 185);
        let p = e.combat.as_ref().unwrap();
        assert_eq!(p.shields, 1);
        assert_eq!(
            p.weapon_weaknesses,
            vec![Weapon::Sword, Weapon::Dagger, Weapon::Axe]
        );
        assert_eq!(p.element_weaknesses, vec![combat::Element::Dark]);
    }
    let b = make(c);
    assert!(
        b.state().actors[3..]
            .iter()
            .all(|a| (2..=4).contains(&a.enemy_charge_max))
    );
    assert!(b.state().rng.draws >= 2);
}

#[test]
fn arithmetic_matches_existing_original_blueprint_calculation_fixtures() {
    // The legacy tests/damage_tests.cpp executes the original compiled Blueprint.
    // Its player cases use ATK=200/DEF=40 and MATK=400/MDEF=80, level-one growth 80.
    let skill = Attack {
        kind: DamageKind::Physical,
        weapon: None,
        element: None,
        invocation: 100,
        ability_ratio: 100,
        hit_bonus: 0,
        critical_bonus: 0,
        critical_power: 200,
        normal_attack: false,
    };
    let input = DamageInput {
        attack: 200,
        defense: 40,
        growth: 80,
        guard: false,
        weapon_weak: false,
        element_weak: false,
        broken: false,
        critical: false,
        attack_up: false,
        defense_down: false,
        defense_up: false,
    };
    assert_eq!(combat::damage(input, &skill, 1.0), 101);
    assert_eq!(
        combat::damage(
            DamageInput {
                attack: 400,
                defense: 80,
                ..input
            },
            &skill,
            1.0
        ),
        202
    );
    // Growth 64 gives neutral level ratio, matching the base-character fixtures.
    let input = DamageInput {
        growth: 64,
        ..input
    };
    assert_eq!(combat::damage(input, &skill, 1.0), 90);
    assert_eq!(
        combat::damage(
            DamageInput {
                critical: true,
                ..input
            },
            &skill,
            1.0
        ),
        180
    );
    assert_eq!(
        combat::damage(
            DamageInput {
                weapon_weak: true,
                element_weak: true,
                ..input
            },
            &skill,
            1.0
        ),
        152
    );
    assert_eq!(
        combat::damage(
            DamageInput {
                attack: 0,
                defense: 999,
                ..input
            },
            &skill,
            1.0
        ),
        1
    );
    assert_eq!(
        combat::damage(
            DamageInput {
                attack: 1000000,
                ..input
            },
            &skill,
            1.0
        ),
        9999
    );
    assert_eq!(
        combat::damage(
            DamageInput {
                guard: true,
                ..input
            },
            &skill,
            1.0
        ),
        87
    );
    assert_eq!(
        combat::damage(
            DamageInput {
                broken: true,
                weapon_weak: true,
                ..input
            },
            &skill,
            1.0
        ),
        180
    );
    let mut rng = CrtRng::new(123);
    assert_eq!(
        combat::damage(input, &skill, combat::variance(&mut rng)),
        90
    );
    assert_eq!(rng.state, 28854610);
}

#[test]
fn accuracy_and_critical_thresholds_are_not_unconditional_draws() {
    assert_eq!(combat::hit_probability(80, 80, 0), 1.0);
    assert!((combat::hit_probability(0, 999, 0) - 0.2).abs() < 1e-6);
    assert_eq!(combat::critical_probability(80, 80, false, 0), 0.0);
    assert!((combat::critical_probability(90, 56, true, 0) - 0.03).abs() < 1e-6);
    let mut rng = CrtRng::new(1);
    assert!(!rng.weighted_bool(0.0));
    assert_eq!(rng.draws, 0);
    assert!(rng.weighted_bool(1.0));
    assert_eq!(rng.draws, 1);
}

#[test]
fn multi_hit_break_uses_pre_break_damage_then_broken_damage() {
    let mut b = make(durable());
    let actor = ActorId(0);
    until(&mut b, actor);
    let r = b
        .step_player(boost("attack_dagger", Some(ActorId(3)), 1))
        .unwrap();
    let hits: Vec<_> = r
        .events
        .iter()
        .filter_map(|e| {
            if let Event::Damage {
                amount,
                broken_before,
                ..
            } = e
            {
                Some((*amount, *broken_before))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(hits.len(), 2);
    assert!(!hits[0].1);
    assert!(hits[1].1);
    assert!(hits[1].0 > hits[0].0);
    assert_eq!(b.state().actors[3].shields, 0);
    assert!(b.state().actors[3].broken());
    assert_eq!(
        r.events
            .iter()
            .filter(|e| matches!(e, Event::Broken { .. }))
            .count(),
        1
    );
}

#[test]
fn neutral_hits_and_misses_do_not_remove_shields() {
    let mut c = durable();
    c.enemies[0]
        .combat
        .as_mut()
        .unwrap()
        .weapon_weaknesses
        .clear();
    let mut b = make(c);
    until(&mut b, ActorId(0));
    b.step_player(boost("attack_dagger", Some(ActorId(3)), 0))
        .unwrap();
    assert_eq!(b.state().actors[3].shields, 1);
    let mut saw_miss = false;
    for seed in 0..30 {
        let mut c = durable();
        c.seed = seed;
        c.players[0].combat.as_mut().unwrap().accuracy = 0;
        c.enemies[0].combat.as_mut().unwrap().evasion = 999;
        let mut b = make(c);
        until(&mut b, ActorId(0));
        let hp = b.state().actors[3].resources.hp;
        let r = b
            .step_player(boost("attack_dagger", Some(ActorId(3)), 0))
            .unwrap();
        if r.events.iter().any(|e| matches!(e, Event::Miss { .. })) {
            saw_miss = true;
            assert_eq!(b.state().actors[3].resources.hp, hp);
            assert_eq!(b.state().actors[3].shields, 1);
        }
    }
    assert!(saw_miss);
}

#[test]
fn breaking_cancels_remaining_turn_and_entire_next_round_then_restores() {
    let mut b = make(durable());
    until(&mut b, ActorId(0));
    let round = b.state().round;
    b.step_player(boost("attack_dagger", Some(ActorId(3)), 0))
        .unwrap();
    let decisions = b.state().ai[3].as_ref().unwrap().actions_taken;
    let mut recovered = false;
    for _ in 0..30 {
        let actor = b.current_actor().unwrap();
        if b.state().round <= round + 1 {
            assert_ne!(actor, ActorId(3));
        }
        if b.state().round == round + 2 {
            recovered = true;
            break;
        }
        if b.side(actor) == Some(Side::Enemy) {
            b.step_enemy().unwrap();
        } else {
            b.step_player(Command::Wait).unwrap();
        }
        assert_eq!(b.state().ai[3].as_ref().unwrap().actions_taken, decisions);
    }
    assert!(recovered);
    assert!(!b.state().actors[3].broken());
    assert_eq!(b.state().actors[3].shields, 1);
    assert_eq!(
        b.current_actor(),
        Some(ActorId(3)),
        "recovered enemy has priority"
    );
}

#[test]
fn magic_uses_elemental_weakness_sp_cost_and_boosted_row() {
    let c = durable();
    let mut b = make(c);
    until(&mut b, ActorId(0));
    let mut boosted = b.clone();
    let sp = b.state().actors[0].resources.sp;
    let low = b
        .step_player(boost("ABI_SET_THI_020", Some(ActorId(3)), 0))
        .unwrap();
    let high = boosted
        .step_player(boost("ABI_SET_THI_020", Some(ActorId(3)), 3))
        .unwrap();
    let amount = |r: &StepReport| {
        r.events
            .iter()
            .find_map(|e| {
                if let Event::Damage { amount, .. } = e {
                    Some(*amount)
                } else {
                    None
                }
            })
            .unwrap()
    };
    assert!(amount(&high) >= amount(&low) * 4);
    assert!(amount(&high) < amount(&low) * 4 + 4);
    assert_eq!(b.state().actors[0].resources.sp, sp - 7);
    assert!(b.state().actors[3].broken());
    assert_eq!(boosted.state().actors[0].resources.bp, 0);
}

#[test]
fn invalid_boost_and_missing_combat_data_fail_without_partial_action() {
    let mut b = make(config());
    until(&mut b, ActorId(0));
    let state = b.state().clone();
    for command in [
        boost("attack_dagger", Some(ActorId(3)), 4),
        boost("attack_dagger", Some(ActorId(3)), 3),
        boost("attack_axe", Some(ActorId(3)), 0),
        boost("attack_dagger", Some(ActorId(1)), 0),
    ] {
        assert!(b.step_player(command).is_err());
        assert_eq!(*b.state(), state);
    }
    let mut c = config();
    c.enemies[0].combat = None;
    assert!(Battle::new(c, ItemCatalog::builtin().unwrap()).is_err());
}

#[test]
fn cover_me_redirects_damage_to_selected_ally_and_counts_hits() {
    let mut c = durable();
    c.players.swap(0, 2);
    c.enemies.truncate(1);
    c.enemies[0].ai = first_opponent();
    // Exercise redirected damage independently of the seed's accuracy roll.
    c.enemies[0].combat.as_mut().unwrap().accuracy = 999;
    let mut b = make(c);
    until(&mut b, ActorId(0));
    b.step_player(boost("ABI_SET_GST_0200", Some(ActorId(2)), 0))
        .unwrap();
    let protected = b.state().actors[0].resources.hp;
    let covering = b.state().actors[2].resources.hp;
    until(&mut b, ActorId(3));
    let r = b.step_enemy().unwrap();
    assert!(r.events.iter().any(|e| matches!(
        e,
        Event::Covered {
            protected: ActorId(0),
            covering: ActorId(2)
        }
    )));
    assert_eq!(b.state().actors[0].resources.hp, protected);
    assert!(b.state().actors[2].resources.hp < covering);
    assert_eq!(b.state().actors[0].cover_hits, 0);
}

#[test]
fn heckle_affects_both_sides_except_caster_and_buffs_stack_to_nine() {
    let mut b = make(durable());
    until(&mut b, ActorId(2));
    // Four turns avoids ambiguity from crossing the round boundary in this assertion.
    b.step_player(boost("ABI_SET_GST_0210", None, 3)).unwrap();
    for (i, a) in b.state().actors.iter().enumerate() {
        assert_eq!(a.has_status(StatusKind::DefenseDown), i != 2);
    }
    until(&mut b, ActorId(1));
    b.step_player(boost("ABI_SET_GST_0180", None, 3)).unwrap();
    let attack = b.state().actors[1]
        .statuses
        .iter()
        .find(|s| s.kind == StatusKind::AttackUp)
        .unwrap();
    assert!(attack.turns <= 9 && attack.turns >= 8);
}

#[test]
fn steal_awards_one_item_per_enemy_and_defend_advances_next_turn() {
    let mut b = make(durable());
    until(&mut b, ActorId(0));
    let mut stolen = None;
    for _ in 0..10 {
        let mut branch = b.clone();
        let report = branch
            .step_player(boost("ABI_SET_THI_010", Some(ActorId(3)), 3))
            .unwrap();
        if report
            .events
            .iter()
            .any(|e| matches!(e, Event::Steal { success: true, .. }))
        {
            stolen = Some(branch);
            break;
        }
        b.step_player(Command::Wait).unwrap();
        until(&mut b, ActorId(0));
    }
    let mut b = stolen.expect("95% steal succeeds within deterministic fixture");
    assert_eq!(b.item_count("healing_grape").unwrap(), 1);
    until(&mut b, ActorId(0));
    let r = b
        .step_player(boost("ABI_SET_THI_010", Some(ActorId(3)), 0))
        .unwrap();
    assert!(
        r.events
            .iter()
            .any(|e| matches!(e, Event::Steal { success: false, .. }))
    );
    assert_eq!(b.item_count("healing_grape").unwrap(), 1);
    until(&mut b, ActorId(0));
    b.step_player(Command::Defend).unwrap();
    assert!(b.state().actors[0].guard);
    assert_eq!(b.state().next_queue[0], ActorId(0));
}

fn attack_policy(b: &Battle) -> Command {
    let actor = b.current_actor().unwrap();
    let target = b
        .state()
        .actors
        .iter()
        .enumerate()
        .find(|(i, s)| s.alive() && b.side(ActorId(*i)) == Some(Side::Enemy))
        .map(|(i, _)| ActorId(i))
        .unwrap();
    let id = if b.actor_id("scaracci") == Some(actor) {
        "attack_axe"
    } else {
        "attack_dagger"
    };
    let bp = if b.state().actors[target.0].broken() {
        b.state().actors[actor.0].resources.bp.min(3)
    } else {
        0
    };
    boost(id, Some(target), bp)
}
#[test]
fn opening_runs_to_victory_and_replays_for_128_seeds() {
    for seed in 0..128 {
        let mut c = config();
        c.seed = seed;
        let mut a = make(c);
        let mut replay = a.clone();
        let mut broke = false;
        for _ in 0..100 {
            if a.state().outcome != Outcome::Ongoing {
                break;
            }
            let actor = a.current_actor().expect("no deadlock");
            let (x, y) = if a.side(actor) == Some(Side::Enemy) {
                (a.step_enemy().unwrap(), replay.step_enemy().unwrap())
            } else {
                let command = attack_policy(&a);
                (
                    a.step_player(command.clone()).unwrap(),
                    replay.step_player(command).unwrap(),
                )
            };
            broke |= x.events.iter().any(|e| matches!(e, Event::Broken { .. }));
            assert_eq!(x, y);
            assert_eq!(a.state(), replay.state());
        }
        assert_eq!(a.state().outcome, Outcome::Victory, "seed {seed}");
        assert!(broke);
    }
}

#[test]
fn every_starting_skill_and_boost_tier_executes_and_spends_once() {
    for owner in 0..3 {
        for level in 0..=3 {
            for id in &config().players[owner].skills {
                let mut b = make(durable());
                until(&mut b, ActorId(owner));
                let skill = b.catalog().skill(id).unwrap();
                let cost = skill.sp_cost;
                let target = match skill.target {
                    TargetKind::Opponent => Some(ActorId(3)),
                    TargetKind::Ally | TargetKind::OtherAlly => Some(ActorId((owner + 1) % 3)),
                    _ => None,
                };
                let before = b.state().actors[owner].resources.sp;
                b.step_player(boost(id, target, level)).unwrap();
                assert_eq!(
                    b.state().actors[owner].resources.sp,
                    before - cost,
                    "{id} boost {level}"
                );
            }
        }
    }
}

#[test]
fn waiting_allows_enemy_ai_to_finish_the_battle_in_defeat() {
    let mut b = make(config());
    for _ in 0..500 {
        if b.state().outcome != Outcome::Ongoing {
            break;
        }
        if b.side(b.current_actor().unwrap()) == Some(Side::Enemy) {
            b.step_enemy().unwrap();
        } else {
            b.step_player(Command::Wait).unwrap();
        }
    }
    assert_eq!(b.state().outcome, Outcome::Defeat);
}

#[test]
fn status_applications_preserve_empty_slot_rng_and_reject_self_cover() {
    let b = make(durable());
    // Compare with Wait from the identical state: any round-boundary work is
    // avoided by choosing a seed where the relevant player is not the last actor.
    let mut checked = 0;
    for (owner, command, expected) in [
        (0, Command::Defend, 21),
        (1, boost("ABI_SET_GST_0180", None, 0), 21),
        (2, boost("ABI_SET_GST_0210", None, 0), 84),
    ] {
        let mut action = b.clone();
        until(&mut action, ActorId(owner));
        let mut waiting = action.clone();
        let before_round = action.state().round;
        let report = action.step_player(command).unwrap();
        waiting.step_player(Command::Wait).unwrap();
        if action.state().round == before_round {
            assert_eq!(report.rng_after.draws - report.rng_before.draws, expected);
            checked += 1;
        }
    }
    assert!(checked >= 2);
    let mut b = b;
    until(&mut b, ActorId(2));
    let before = b.state().clone();
    assert!(
        b.step_player(boost("ABI_SET_GST_0200", Some(ActorId(2)), 0))
            .is_err()
    );
    assert_eq!(b.state(), &before);
}

#[test]
fn boosting_suppresses_only_the_next_bp_regeneration() {
    let mut b = make(durable());
    until(&mut b, ActorId(0));
    b.step_player(boost("attack_dagger", Some(ActorId(3)), 3))
        .unwrap();
    until(&mut b, ActorId(0));
    assert_eq!(b.state().actors[0].resources.bp, 0);
    b.step_player(Command::Wait).unwrap();
    until(&mut b, ActorId(0));
    assert_eq!(b.state().actors[0].resources.bp, 1);
}

#[test]
fn charged_enemy_failure_restores_bp_cap_rng_and_ai_memory() {
    let mut c = durable();
    for e in &mut c.enemies {
        e.resources = Some(Resources {
            hp: 10000,
            sp: 0,
            bp: 5,
            latent: 0,
        });
    }
    c.skills
        .iter_mut()
        .find(|s| s.id == "ABI_ENE_0041")
        .unwrap()
        .sp_cost = 1;
    let mut b = make(c);
    while b.side(b.current_actor().unwrap()) == Some(Side::Player) {
        b.step_player(Command::Wait).unwrap();
    }
    let actor = b.current_actor().unwrap();
    assert!(b.state().actors[actor.0].resources.bp >= b.state().actors[actor.0].enemy_charge_max);
    let before = b.state().clone();
    assert!(b.step_enemy().is_err());
    assert_eq!(b.state(), &before);
}
