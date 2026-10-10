use ot2_battle_engine::{
    CrtRng,
    escape::Situation,
    field_escape::{FieldEscapeMenu, level_for_experience},
};

#[test]
fn snows_zero_bp_break_replays_solo_and_party_initialization() {
    use ot2_battle_engine::{combat::Weapon,field_escape::opening_break_menu_with_party};
    let enemies=["ENE_NML_FLD_SNW_010"];
    let mut actors=[false;2];
    for levels in [&[5][..],&[5,1][..]] {
        for night in [false,true] {
            for seed in 0..64 {
                let entry=CrtRng {state:seed,draws:6161};
                let expected=if levels.len()==1 {FieldEscapeMenu::new(entry,Situation::Normal,5,&enemies)}
                    else {FieldEscapeMenu::new_party(entry,Situation::Normal,[5,1],&enemies)}.unwrap();
                let weapon=if expected.first_actor==0 {Weapon::Dagger} else {Weapon::Staff};
                let menu=opening_break_menu_with_party(entry,Situation::Normal,levels,&enemies,weapon,&[],night)
                    .unwrap().unwrap_or_else(||panic!("No break: levels={levels:?}, night={night}, seed={seed}, first={}",expected.first_actor));
                let escaped=menu.attempt(&[]).unwrap();
                assert_eq!(escaped.at_attack_menu,expected.rng,"Break and Escape must share the same actual initialization");
                assert_eq!(escaped.attack_actor,expected.first_actor);
                assert_eq!(escaped.boost,0);
                assert!(escaped.attempt.success);
                assert_eq!(escaped.attempt.before_check,escaped.attempt.after,"Broken Escape has no probability roll");
                assert!(escaped.attempt.trace.windows(2).all(|t|t[0].after==t[1].before));
                actors[escaped.attack_actor]=true;
            }
        }
    }
    assert_eq!(actors,[true;2]);
    assert!(opening_break_menu_with_party(CrtRng::new(0),Situation::Normal,&[5],
        &["ENE_NML_FLD_SNW_020"],Weapon::Sword,&[],false).unwrap().is_none());
}

#[test]
fn unboosted_break_retains_bp_and_round_two_boosts_replay_real_voice_draws() {
    use ot2_battle_engine::{combat::Weapon,field_escape::opening_break_menu};
    let entry=CrtRng {state:123,draws:6000};
    let enemy=["ENE_NML_FLD_CTY_010"];
    let menu=opening_break_menu(entry,Situation::Normal,5,&enemy,Weapon::Dagger,&[]).unwrap().unwrap();
    assert_eq!(menu.available_bp(),2);
    let before=menu.rng();
    let baseline=menu.clone().attempt(&[]).unwrap();
    let adjusted=menu.clone().attempt(&[1,2,1,2,1,0]).unwrap();
    assert_eq!(adjusted.at_escape_menu,before);
    assert_eq!(baseline.attempt.before_check,baseline.attempt.after);
    assert_eq!(adjusted.attempt.before_check,adjusted.attempt.after);
    assert_eq!(adjusted.attempt.after.draws-baseline.attempt.after.draws,5);
    assert_eq!(menu.rng(),before,"Search trials must not consume the source snapshot");
    assert!(adjusted.attempt.trace.windows(2).all(|w|w[0].after==w[1].before));
    let mut replay=entry;replay.advance(adjusted.attempt.after.draws-entry.draws);
    assert_eq!(replay,adjusted.attempt.after);
    assert!(menu.attempt(&[1,2,3,2,1,0]).is_err());
    let spent=opening_break_menu(entry,Situation::Normal,5,&["ENE_NML_FLD_CTY_020"],Weapon::Dagger,&[1]).unwrap().unwrap();
    assert_eq!(spent.available_bp(),0);
    assert!(spent.clone().attempt(&[1,0]).is_err());
    assert!(spent.attempt(&[]).unwrap().attempt.success);
}

#[test]
fn party_escape_reveals_per_type_and_uses_actual_menu_actor_and_party_levels() {
    let same=["ENE_NML_FLD_WLD_010","ENE_NML_FLD_WLD_010"];
    let distinct=["ENE_NML_FLD_WLD_010","ENE_NML_FLD_WLD_020"];
    for (enemies,expected) in [(&same[..],1),(&distinct[..],2)] {
        for seed in 0..128 {
            let entry=CrtRng::new(seed);
            let menu=FieldEscapeMenu::new_party(entry,Situation::Normal,[5,1],enemies).unwrap();
            let study:Vec<_>=menu.trace.iter().filter(|t|t.reason=="study_foe_weakness").collect();
            assert_eq!(study.len(),expected);
            assert!(study.iter().all(|t|t.after.draws-t.before.draws==1));
            assert!(menu.first_actor<2);
            assert_eq!(menu.current_order[0],menu.first_actor);
            assert_eq!(menu.trace.last().unwrap().actor.unwrap().0,menu.first_actor);
            assert!(menu.trace.windows(2).all(|t|t[0].after==t[1].before));
            let first_actor=menu.first_actor;
            let attempt=menu.attempt(&[1,0,1,0],5).unwrap();
            assert_eq!(attempt.chance_percent,ot2_battle_engine::escape::success_percent(&[5,1],5,0,0).unwrap());
            assert!(attempt.trace.iter().filter(|t|t.reason=="boost_voice").all(|t|t.actor.unwrap().0==first_actor));
            let mut replay=entry;
            replay.advance(attempt.after.draws-entry.draws);
            assert_eq!(replay,attempt.after);
        }
    }
    let menu=FieldEscapeMenu::new_party(CrtRng::new(0),Situation::Preemptive,[5,1],&same).unwrap();
    assert_eq!(menu.current_order.len(),2);
    let rng=menu.rng;
    let attempt=menu.attempt(&[],5).unwrap();
    assert!(attempt.success);
    assert_eq!(attempt.after,rng);
}

#[test]
fn immediate_break_uses_actual_hits_and_guarantees_next_menu_escape() {
    use ot2_battle_engine::{Event, combat::Weapon, field_escape::first_action_break};
    let cases = [
        ("ENE_NML_FLD_CTY_010", Weapon::Dagger, 0),
        ("ENE_NML_FLD_ISD_100", Weapon::Sword, 1),
        ("ENE_NML_FLD_CTY_020", Weapon::Dagger, 1),
        ("ENE_NML_FLD_CTY_120", Weapon::Dagger, 0),
    ];
    for (enemy, weapon, bp) in cases {
        let entry = CrtRng {
            state: 123,
            draws: 6000,
        };
        let result = first_action_break(entry, Situation::Normal, 5, &[enemy])
            .unwrap()
            .unwrap();
        assert_eq!(result.weapon, weapon);
        assert_eq!(result.boost, bp);
        assert_eq!(result.escape_round, 2);
        assert_eq!(
            result.at_attack_menu,
            FieldEscapeMenu::new(entry, Situation::Normal, 5, &[enemy])
                .unwrap()
                .rng
        );
        assert!(
            result
                .report
                .events
                .iter()
                .any(|e| matches!(e, Event::Broken { .. }))
        );
        assert!(result.attempt.success);
        assert_eq!(result.attempt.before_check, result.attempt.after);
        assert!(
            result
                .attempt
                .trace
                .windows(2)
                .all(|t| t[0].after == t[1].before)
        );
        let mut rng = entry;
        rng.advance(result.attempt.after.draws - entry.draws);
        assert_eq!(rng, result.attempt.after);
    }
    assert!(
        first_action_break(
            CrtRng::new(123),
            Situation::Normal,
            5,
            &["ENE_NML_FLD_CTY_120", "ENE_NML_FLD_CTY_120"]
        )
        .unwrap()
        .is_none()
    );
    // High evasion: shield count and weapon weakness alone do not prove a break.
    assert!((0..32).any(|seed| {
        first_action_break(
            CrtRng::new(seed),
            Situation::Normal,
            5,
            &["ENE_NML_OTR_130"],
        )
        .unwrap()
        .is_none()
    }));
}

#[test]
fn ordinary_highroad_menu_and_escape_preserve_source_boundaries() {
    let mut entry = CrtRng::new(0);
    entry.advance(5971);
    let menu = FieldEscapeMenu::new(
        entry,
        Situation::Normal,
        5,
        &["ENE_NML_FLD_CTY_120", "ENE_NML_FLD_CTY_120"],
    )
    .unwrap();
    assert_eq!(menu.rng.draws, 6002);
    assert_eq!(menu.rng.state, 0x5d64baea);
    for repeats in 0..5 {
        let path: Vec<_> = (0..repeats).flat_map(|_| [1, 0]).collect();
        assert!(!menu.clone().attempt(&path, 5).unwrap().success);
    }
    let a = menu.attempt(&[1, 0, 1, 0, 1, 0, 1, 0, 1, 0], 5).unwrap();
    assert!(a.success);
    assert_eq!(a.chance_percent, 40);
    assert_eq!(a.after.draws, 6008);
    assert!(a.trace.windows(2).all(|w| w[0].after == w[1].before));
    assert_eq!(level_for_experience(51), 5);
    assert_eq!(level_for_experience(91), 5);
}

#[test]
fn preemptive_escape_bypasses_lottery_and_unsupported_inputs_fail() {
    for enemies in [
        vec!["ENE_NML_FLD_CTY_010"],
        vec!["ENE_NML_FLD_CTY_020", "ENE_NML_FLD_ISD_100"],
        vec!["ENE_NML_OTR_130", "ENE_NML_WTR_530", "ENE_NML_FLD_CTY_120"],
    ] {
        let menu =
            FieldEscapeMenu::new(CrtRng::new(12), Situation::Preemptive, 5, &enemies).unwrap();
        let a = menu.clone().attempt(&[], 5).unwrap();
        assert!(a.success);
        assert_eq!(a.before_check, a.after);
        assert!(menu.clone().attempt(&[1, 2], 5).is_err());
        assert!(menu.attempt(&[1], 5).is_err());
    }
    assert!(
        FieldEscapeMenu::new(
            CrtRng::new(0),
            Situation::Surprise,
            5,
            &["ENE_NML_FLD_CTY_010"]
        )
        .is_err()
    );
    assert!(FieldEscapeMenu::new(CrtRng::new(0), Situation::Normal, 5, &["unknown"]).is_err());
}
