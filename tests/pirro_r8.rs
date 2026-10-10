use ot2_battle_engine::{ActorId, Battle, BattleConfig, ItemCatalog, Outcome, pirro::PirroRoutine};

#[test]
fn r8_swords_boost_preview_matches_early_and_late_latent_execution() {
    let mut branches = [false; 2];
    for seed in 0..1024 {
        let mut b = Battle::new_pirro(
            BattleConfig::throne_pirro(seed).unwrap(),
            ItemCatalog::builtin().unwrap(),
            true,
        )
        .unwrap();
        let mut routine = PirroRoutine::default();
        while b.state().outcome == Outcome::Ongoing {
            if b.state().round == 8 && b.current_actor() == Some(ActorId(0)) {
                break;
            }
            if routine.step(&mut b).is_err() {
                break;
            }
        }
        if b.state().round != 8
            || b.current_actor() != Some(ActorId(0))
            || routine.heal_before_round_eight != Some(false)
        {
            continue;
        }
        let early = b.can_use_latent();
        if branches[usize::from(early)] {
            continue;
        }
        let before = b.state().clone();
        let path = [1, 0, 1, 2, 3];
        let preview = if early {
            b.preview_boost_inputs_after_latent(&path)
        } else {
            b.preview_boost_inputs(&path)
        }
        .unwrap();
        assert_eq!(&before, b.state());
        let report = routine.step_with_boost_inputs(&mut b, &path).unwrap();
        assert_eq!(report.rng_before, before.rng);
        let trace = b
            .state()
            .opening_crt
            .as_ref()
            .unwrap()
            .trace
            .as_ref()
            .unwrap();
        let last_boost = trace.iter().rfind(|t| t.reason == "boost_voice").unwrap();
        assert_eq!(last_boost.after, preview.rng);
        assert_eq!(routine.latent_used, early);
        assert_eq!(routine.latent_delayed, !early);
        branches[usize::from(early)] = true;
        if branches == [true, true] {
            break;
        }
    }
    assert_eq!(branches, [true, true]);
}
