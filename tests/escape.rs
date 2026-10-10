use ot2_battle_engine::{
    CrtRng, Outcome, Resources,
    escape::{FirstEscapeBattle, Situation, success_percent},
};
fn resources() -> Resources {
    Resources {
        hp: 261,
        sp: 28,
        bp: 1,
        latent: 0,
    }
}
#[test]
fn mansion_boosted_dagger_matches_retail_round_two_menu() {
    // Retail seed 0, shared recreation, title4916: composition boundary5525.
    let initial=FirstEscapeBattle::mansion(0x9E28D147,Situation::Normal,resources(),true).unwrap();
    assert_eq!(initial.state().rng.draws,5549-5525);
    let menu=initial.break_menu(&[1]).unwrap().unwrap();
    assert_eq!(menu.state().rng.draws,5582-5525);
    let mut expected=CrtRng::new(0);
    expected.advance(5582);
    assert_eq!(menu.state().rng.state,expected.state);
    let trace=menu.state().opening_crt.as_ref().unwrap().trace.as_ref().unwrap();
    for reason in ["attack_voice","last_attack_voice"] {
        let span=trace.iter().find(|s|s.reason==reason).unwrap();
        assert_eq!(span.after.draws-span.before.draws,1);
    }
}
#[test]
fn mansion_break_snapshot_replays_next_menu_boosts_only_with_remaining_bp() {
    let mut covered=[false;2];
    for situation in [Situation::Normal,Situation::Surprise] {
        for seed in 0..32 {
            let initial=FirstEscapeBattle::mansion(seed,situation,resources(),true).unwrap();
            let original=initial.state().rng;let initial_bp=initial.state().actors[0].resources.bp;
            let Some(menu)=initial.clone().break_menu(&[1]).unwrap() else {continue};
            assert_eq!(initial.state().rng,original);
            assert_eq!(menu.state().actors[0].resources.bp,initial_bp-1);
            let before=menu.state().rng;
            let baseline=menu.clone().attempt_broken(&[]).unwrap();
            assert!(baseline.success && baseline.guaranteed);
            assert_eq!(baseline.before_check,baseline.after);
            assert_eq!(initial.clone().break_then_escape(&[1]).unwrap().unwrap().after,baseline.after);
            if initial_bp==1 {
                covered[0]=true;assert!(menu.clone().attempt_broken(&[1,0]).is_err());
            } else {
                covered[1]=true;
                let adjusted=menu.clone().attempt_broken(&[1,0]).unwrap();
                assert_eq!(adjusted.after.draws,baseline.after.draws+1);
                assert_eq!(adjusted.before_check,adjusted.after);
                assert!(adjusted.success);
            }
            assert_eq!(menu.state().rng,before);
        }
    }
    assert_eq!(covered,[true;2]);
}
#[test]
fn source_escape_level_revision_uses_four_slots_and_attempt_bonus() {
    assert_eq!(success_percent(&[2], 3, 0, 0).unwrap(), 55);
    assert_eq!(success_percent(&[20], 3, 0, 0).unwrap(), 55); // 20/4 - 3 = 2
    assert_eq!(success_percent(&[20; 4], 3, 0, 0).unwrap(), 100);
    assert_eq!(success_percent(&[2], 3, 1, 0).unwrap(), 70);
    assert_eq!(success_percent(&[2], 3, 2, 0).unwrap(), 85);
    assert_eq!(success_percent(&[2], 3, 3, 0).unwrap(), 155);
}
#[test]
fn first_menu_initialization_and_escape_roll_are_contiguous() {
    for situation in [
        Situation::Normal,
        Situation::Preemptive,
        Situation::Surprise,
    ] {
        for seed in 0..100 {
            let b = FirstEscapeBattle::mansion(seed, situation, resources(), true).unwrap();
            // Unlike Pirro's designated Throné speaker, this group has no
            // TalkPlayerID: the random fallback draws even for a solo party.
            let speaker = b
                .state()
                .opening_crt
                .as_ref()
                .unwrap()
                .trace
                .as_ref()
                .unwrap()
                .iter()
                .find(|s| s.reason == "battle_start_speaker")
                .unwrap();
            assert_eq!(speaker.after.draws - speaker.before.draws, 1);
            assert!(b.state().actors[0].resources.bp >= 1);
            assert_eq!(b.state().actors[1].resources.hp, 217);
            assert_eq!(b.state().actors[1].shields, 2);
            assert_eq!(
                b.enemy_actions_before_menu.is_empty(),
                situation != Situation::Surprise
            );
            let a = b.attempt(&[1, 0], 3).unwrap();
            assert_eq!(a.before_check.draws, a.menu.draws + 1);
            assert_eq!(
                a.after.draws,
                a.before_check.draws + u64::from(situation != Situation::Preemptive)
            );
            if situation == Situation::Preemptive {
                assert!(a.success);
            }
            if a.success {
                assert_eq!(a.state.outcome, Outcome::Escaped);
            }
            let mut rng = CrtRng::new(seed);
            for span in a.state.opening_crt.unwrap().trace.unwrap() {
                assert_eq!(rng, span.before);
                rng.advance(span.after.draws - span.before.draws);
                assert_eq!(rng, span.after);
            }
            assert_eq!(rng, a.after);
        }
    }
}
