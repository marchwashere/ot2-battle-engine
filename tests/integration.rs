use ot2_battle_engine::{
    ai::{AiAction, AiRule, AiState, AiTarget, BossAi, Condition, Selection, WeightedAction},
    content::{HitCheck, TargetKind},
    *,
};

fn config() -> BattleConfig {
    BattleConfig::from_json(include_str!("../data/demo_battle.json")).unwrap()
}
fn make(config: BattleConfig) -> Battle {
    Battle::new(config, ItemCatalog::builtin().unwrap()).unwrap()
}
fn item(id: &str, target: Option<usize>) -> Command {
    Command::Item {
        id: id.into(),
        target: target.map(ActorId),
    }
}
fn strike(target: usize) -> Command {
    Command::Skill {
        id: "training_strike".into(),
        target: Some(ActorId(target)),
    }
}

#[test]
fn crt_known_sequence_and_fixed_output_draws() {
    let mut rng = CrtRng::new(1);
    let output: Vec<_> = (0..10).map(|_| rng.next_u15()).collect();
    assert_eq!(
        output,
        [
            41, 18467, 6334, 26500, 19169, 15724, 11478, 29358, 26962, 24464
        ]
    );
    assert_eq!(rng.draws, 10);
    assert!(!rng.weighted_bool(0.0));
    assert_eq!(rng.draws, 10);
    assert!(rng.weighted_bool(1.0));
    assert_eq!(rng.draws, 11);
    assert_eq!(rng.index(0), 0);
    assert_eq!(rng.draws, 11);
    assert_eq!(rng.index(1), 0);
    assert_eq!(rng.draws, 12);
    let mut wrap = CrtRng::new(u32::MAX);
    assert_eq!(wrap.next_u15(), 35);
}

#[test]
fn every_imported_recovery_item_executes_with_caps_and_no_intrinsic_rng() {
    // Expected values come from this workspace's ItemDB, not a wiki/NPC skill.
    let cases = [
        ("healing_grape", 600, 5, 1, 10),
        ("healing_grape_m", 1350, 5, 1, 10),
        ("healing_grape_bunch", 1000, 5, 1, 10),
        ("inspiriting_plum", 100, 45, 1, 10),
        ("inspiriting_plum_m", 100, 105, 1, 10),
        ("inspiriting_plum_basket", 100, 50, 1, 10),
        ("refreshing_jam", 5000, 200, 1, 10),
        ("rejuvenating_jam", 100, 5, 5, 100),
        ("reinforcing_jam", 5000, 200, 5, 10),
        ("revitalizing_jam", 5000, 200, 5, 100),
    ];
    for (id, hp, sp, bp, latent) in cases {
        let mut c = config();
        c.players[0].stats.max_hp = 5000;
        c.players[0].stats.max_sp = 200;
        c.players[0].resources = Some(Resources {
            hp: 100,
            sp: 5,
            bp: 1,
            latent: 10,
        });
        c.inventory.insert(id.into(), 1);
        let mut b = make(c);
        let target = if b.catalog().item(id).unwrap().target == TargetKind::AllAllies {
            None
        } else {
            Some(0)
        };
        let report = b.step_player(item(id, target)).unwrap();
        assert_eq!(
            b.state().actors[0].resources,
            Resources { hp, sp, bp, latent },
            "{id}"
        );
        assert_eq!(b.item_count(id).unwrap(), 0);
        assert_eq!(report.rng_before, report.rng_after);
        assert_eq!(b.current_actor(), Some(ActorId(1)));
    }
}

#[test]
fn party_items_skip_ko_and_consumption_is_once_per_action() {
    let mut c = config();
    let mut ally = c.players[0].clone();
    ally.id = "ally".into();
    ally.stats.speed = 15;
    ally.resources = Some(Resources {
        hp: 50,
        sp: 0,
        bp: 0,
        latent: 0,
    });
    c.players.push(ally.clone());
    ally.id = "ko".into();
    ally.resources.as_mut().unwrap().hp = 0;
    c.players.push(ally);
    c.inventory.insert("healing_grape_bunch".into(), 1);
    let mut b = make(c);
    b.step_player(item("healing_grape_bunch", None)).unwrap();
    assert_eq!(b.state().actors[0].resources.hp, 1000);
    assert_eq!(b.state().actors[1].resources.hp, 950);
    assert_eq!(b.state().actors[2].resources.hp, 0);
    assert_eq!(b.item_count("healing_grape_bunch").unwrap(), 0);
    assert_eq!(b.current_actor(), Some(ActorId(1)));
}

#[test]
fn invalid_commands_leave_the_entire_state_unchanged() {
    for command in [
        item("healing_grape", Some(1)),
        item("healing_grape", Some(99)),
        item("missing", Some(0)),
        item("revitalizing_jam", None),
        strike(1),
        Command::Skill {
            id: "enemy_strike".into(),
            target: Some(ActorId(1)),
        },
    ] {
        let mut b = make(config());
        let before = b.state().clone();
        assert!(b.step_player(command).is_err());
        assert_eq!(*b.state(), before);
    }
    let mut b = make(config());
    b.step_player(Command::Wait).unwrap();
    let before = b.state().clone();
    assert!(b.step_player(Command::Wait).is_err());
    assert_eq!(*b.state(), before);
    let mut c = config();
    c.inventory.insert("healing_grape".into(), 0);
    let mut b = make(c);
    let before = b.state().clone();
    assert!(b.step_player(item("healing_grape", Some(0))).is_err());
    assert_eq!(*b.state(), before);
}

#[test]
fn invalid_ai_decision_rolls_back_selection_draw_and_ai_memory() {
    let mut c = config();
    c.enemies[0].stats.speed = 100;
    c.skills[1].sp_cost = 10;
    c.enemies[0].ai = single_state(Selection::Weighted {
        choices: vec![WeightedAction {
            weight: 1,
            action: AiAction {
                skill: "enemy_strike".into(),
                target: AiTarget::RandomOpponent,
            },
        }],
    });
    let mut b = make(c);
    let before = b.state().clone();
    assert!(b.step_enemy().unwrap_err().0.contains("SP"));
    assert_eq!(*b.state(), before);
}

fn single_state(selection: Selection) -> BossAi {
    BossAi {
        schema_version: 1,
        initial_state: "loop".into(),
        states: vec![AiState {
            id: "loop".into(),
            rules: vec![AiRule {
                condition: Condition::Always,
                selection,
                next_state: None,
            }],
        }],
    }
}

#[test]
fn weighted_ai_and_random_target_each_advance_crt_even_with_one_choice() {
    let mut c = config();
    c.enemies[0].stats.speed = 100;
    c.enemies[0].ai = single_state(Selection::Weighted {
        choices: vec![WeightedAction {
            weight: 1,
            action: AiAction {
                skill: "enemy_strike".into(),
                target: AiTarget::RandomOpponent,
            },
        }],
    });
    let mut b = make(c);
    let mut expected = b.state().rng;
    expected.next_u15();
    expected.next_u15();
    b.step_enemy().unwrap();
    assert_eq!(b.state().rng, expected);
    assert_eq!(b.current_actor(), Some(ActorId(0)));
}

#[test]
fn sequence_ai_advances_and_phase_transition_persists() {
    let mut c = config();
    c.enemies[0].resources = Some(Resources {
        hp: 400,
        sp: 0,
        bp: 0,
        latent: 0,
    });
    let mut b = make(c);
    b.step_player(Command::Wait).unwrap();
    let report = b.step_enemy().unwrap();
    assert!(matches!(&report.events[0], Event::Action { name, .. } if name == "heavy_strike"));
    assert_eq!(b.state().ai[1].as_ref().unwrap().state, 1);
    assert_eq!(b.state().rng.draws, 0);
    b.step_player(Command::Wait).unwrap();
    b.step_enemy().unwrap();
    assert_eq!(b.state().rng.draws, 2);

    let mut c = config();
    c.players[0].resources = None;
    c.enemies[0].ai = single_state(Selection::Sequence {
        actions: vec![
            AiAction {
                skill: "enemy_strike".into(),
                target: AiTarget::FirstOpponent,
            },
            AiAction {
                skill: "heavy_strike".into(),
                target: AiTarget::FirstOpponent,
            },
        ],
    });
    let mut b = make(c);
    for expected in ["enemy_strike", "heavy_strike", "enemy_strike"] {
        b.step_player(Command::Wait).unwrap();
        let report = b.step_enemy().unwrap();
        assert!(matches!(&report.events[0], Event::Action { name, .. } if name == expected));
    }
    assert_eq!(b.state().rng.draws, 0);
}

#[test]
fn complete_battle_runs_to_victory_and_cloned_branches_replay() {
    let mut c = config();
    c.players[0].resources = None;
    c.players[0].stats.max_sp = 1000;
    c.skills[0].hit = HitCheck::Guaranteed;
    let mut original = make(c);
    let mut replay = original.clone();
    for _ in 0..20 {
        if original.state().outcome != Outcome::Ongoing {
            break;
        }
        let a = original.step_player(strike(1)).unwrap();
        let b = replay.step_player(strike(1)).unwrap();
        assert_eq!(a, b);
        assert_eq!(
            original.advance_enemies(10).unwrap(),
            replay.advance_enemies(10).unwrap()
        );
        assert_eq!(original.state(), replay.state());
    }
    assert_eq!(original.state().outcome, Outcome::Victory);
    assert_eq!(original.current_actor(), None);
    assert!(original.step_player(Command::Wait).is_err());
}

#[test]
fn defeat_and_ko_turn_removal_work() {
    let mut c = config();
    c.players[0].resources.as_mut().unwrap().hp = 1;
    let mut b = make(c);
    b.step_player(Command::Wait).unwrap();
    b.step_enemy().unwrap();
    assert_eq!(b.state().outcome, Outcome::Defeat);
    assert_eq!(b.current_actor(), None);
    let mut c = config();
    c.players[0].resources = None;
    c.skills[0].sp_cost = 0;
    c.skills[0].hit = HitCheck::Guaranteed;
    c.skills[0].effects = vec![Effect::FixedDamage { amount: 1000 }];
    let mut enemy = c.enemies[0].clone();
    enemy.id = "second_enemy".into();
    c.enemies.push(enemy);
    let mut b = make(c);
    b.step_player(strike(1)).unwrap();
    assert_eq!(b.current_actor(), Some(ActorId(2)));
    b.step_enemy().unwrap();
    assert_eq!(b.current_actor(), Some(ActorId(0)));
    assert_eq!(b.state().round, 2);
}

#[test]
fn healing_does_not_revive_and_full_resources_stay_capped() {
    let mut c = config();
    let mut ally = c.players[0].clone();
    ally.id = "ko".into();
    ally.resources.as_mut().unwrap().hp = 0;
    c.players.push(ally);
    let mut b = make(c);
    let before = b.state().clone();
    assert!(b.step_player(item("healing_grape", Some(1))).is_err());
    assert_eq!(*b.state(), before);
    let mut c = config();
    c.players[0].resources = Some(Resources {
        hp: 999,
        sp: 99,
        bp: 5,
        latent: 100,
    });
    let mut b = make(c);
    b.step_player(item("revitalizing_jam", Some(0))).unwrap();
    assert_eq!(
        b.state().actors[0].resources,
        Resources {
            hp: 1000,
            sp: 100,
            bp: 5,
            latent: 100
        }
    );
}

#[test]
fn malformed_or_unsupported_content_is_rejected() {
    let mut c = config();
    c.schema_version = 99;
    assert!(Battle::new(c, ItemCatalog::builtin().unwrap()).is_err());
    let mut c = config();
    c.enemies[0].id = c.players[0].id.clone();
    assert!(Battle::new(c, ItemCatalog::builtin().unwrap()).is_err());
    let mut c = config();
    c.enemies[0].ai.initial_state = "missing".into();
    assert!(Battle::new(c, ItemCatalog::builtin().unwrap()).is_err());
    let mut c = config();
    c.enemies[0].ai.states[0].rules[0].next_state = Some("missing".into());
    assert!(Battle::new(c, ItemCatalog::builtin().unwrap()).is_err());
    let mut c = config();
    c.enemies[0].ai = single_state(Selection::Weighted { choices: vec![] });
    assert!(Battle::new(c, ItemCatalog::builtin().unwrap()).is_err());
    let mut c = config();
    c.enemies[0].ai = single_state(Selection::Sequence {
        actions: vec![AiAction {
            skill: "missing".into(),
            target: AiTarget::SelfOnly,
        }],
    });
    assert!(Battle::new(c, ItemCatalog::builtin().unwrap()).is_err());
    let mut json = serde_json::to_value(config()).unwrap();
    json["animations"] = serde_json::json!([]);
    assert!(BattleConfig::from_json(&json.to_string()).is_err());
    assert!(
        BossAi::from_json(
            &include_str!("../data/demo_ai.json").replace("\"always\"", "\"unimplemented_script\"")
        )
        .is_err()
    );
    let mut c = config();
    c.skills[0].effects = vec![];
    assert!(Battle::new(c, ItemCatalog::builtin().unwrap()).is_err());
}

#[test]
fn random_actions_replay_across_many_seeds() {
    for seed in 0..64 {
        let mut c = config();
        c.seed = seed;
        c.players[0].resources = None;
        c.players[0].stats.max_sp = 1000;
        let mut a = make(c);
        let mut b = a.clone();
        for _ in 0..30 {
            if a.state().outcome != Outcome::Ongoing {
                break;
            }
            let report_a = if a.side(a.current_actor().unwrap()) == Some(Side::Player) {
                a.step_player(strike(1))
            } else {
                a.step_enemy()
            }
            .unwrap();
            let report_b = if b.side(b.current_actor().unwrap()) == Some(Side::Player) {
                b.step_player(strike(1))
            } else {
                b.step_enemy()
            }
            .unwrap();
            assert_eq!(report_a, report_b);
            assert_eq!(a.state(), b.state());
        }
        assert_ne!(a.state().outcome, Outcome::Ongoing);
    }
}
