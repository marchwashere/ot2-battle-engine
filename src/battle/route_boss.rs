//! Initialization and two authored order slots for the no-enemy-action route.
//! Source: subB_Dng_Dst_2_2 / subB_Dng_Ocn_1_3 AIInitialize,
//! BP_BattleAIUtility.CreateActionTable and native Array_Shuffle.
use super::*;
pub(super) fn validate_boss_ai(config: &BattleConfig, crt: Option<&OpeningCrtState>) -> Result<()> {
    let Some(crt) = crt else { return Ok(()) };
    if crt.lord_sands {
        ensure(
            config.enemies.len() == 1
                && matches!(
                    config.enemies[0].id.as_str(),
                    "ENE_NML_FLD_DST_280" | "ENE_NML_FLD_CTY_260"
                ),
            "Boss requires its own audited AI initialization and action-slot profile",
        )?;
    } else if crt.mansion_situation.is_some() && !crt.pirro_battle {
        static IDS: std::sync::OnceLock<std::collections::BTreeSet<String>> =
            std::sync::OnceLock::new();
        let ids = IDS.get_or_init(|| {
            serde_json::from_str(include_str!("../../data/boss-ai-enemies.json"))
                .expect("boss AI identities")
        });
        for enemy in &config.enemies {
            ensure(!ids.contains(&enemy.id),format!("{} uses boss AI; ordinary field initialization cannot model its action tables or multiple slots",enemy.id))?;
        }
    }
    Ok(())
}
impl Battle {
    fn route_boss_profile(&self, id: ActorId) -> Option<bool> {
        if !self
            .state
            .opening_crt
            .as_ref()
            .is_some_and(|c| c.lord_sands)
        {
            return None;
        }
        match self.scenario.actors[id.0].id.as_str() {
            "ENE_NML_FLD_DST_280" => Some(true),
            "ENE_NML_FLD_CTY_260" => Some(false),
            _ => None,
        }
    }
    pub(super) fn route_boss_order_count(&self, id: ActorId) -> usize {
        if self.route_boss_profile(id).is_some() {
            2
        } else {
            1
        }
    }
    pub(super) fn route_boss_order_ratio(&self, id: ActorId, slot: usize) -> f32 {
        // NORMAL ignores DelayRatio; DELAY multiplies the cached action time.
        match (self.route_boss_profile(id), slot) {
            (Some(true), 1) => 1.3,
            (Some(false), 1) => 0.6,
            _ => 1.0,
        }
    }
    pub(super) fn initialize_route_boss_ai(&mut self) {
        for i in 0..self.state.actors.len() {
            let id = ActorId(i);
            let Some(sands) = self.route_boss_profile(id) else {
                continue;
            };
            let tables: Vec<(Vec<u32>, bool)> = if sands {
                vec![
                    (vec![0], true),
                    (vec![1, 5, 4, 1], false),
                    (vec![2], true),
                    (vec![1, 7, 3, 1], true),
                    (vec![0], true),
                    (vec![4, 5, 7, 1], true),
                ]
            } else {
                vec![
                    (vec![8], true),
                    (vec![0, 7, 2, 5], true),
                    (vec![9], true),
                    (vec![0, 4, 3, 5], true),
                    (vec![9], true),
                    (vec![0, 3, 7, 5], true),
                ]
            };
            for (mut table, shuffle) in tables {
                if !shuffle {
                    continue;
                }
                let before = self.state.rng;
                // Native Array_Shuffle draws even for the final singleton.
                for at in 0..table.len() {
                    let offset = self.state.rng.index((table.len() - at) as u32) as usize;
                    table.swap(at, at + offset);
                }
                self.trace_rng("boss_ai_action_table_shuffle", Some(id), before);
            }
            // ExecSettingNextOrder removes the default enemy row, then inserts
            // NORMAL and DELAY among the existing player rows. Initialization
            // subsequently rebuilds and sorts the complete authored row list.
            let mut rows: Vec<_> = (0..self.scenario.actors.len())
                .filter(|&n| n != i)
                .map(ActorId)
                .collect();
            for _ in 0..2 {
                let before = self.state.rng;
                let at = self.state.rng.index((rows.len() + 1) as u32) as usize;
                rows.insert(at, id);
                self.trace_rng("boss_ai_insert_action_order", Some(id), before);
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unsupported_boss_cannot_use_generic_route_or_field_profile() {
        let mut c = BattleConfig::from_json(include_str!("../../data/gigantes.json")).unwrap();
        c.enemies[0].id = "ENE_BOS_WAR_C01_010".into();
        assert!(Battle::new_route_boss(c.clone(), None)
            .unwrap_err()
            .to_string()
            .contains("audited AI"));
        let mut crt = OpeningCrtState::new(true);
        crt.mansion_situation = Some(crate::escape::Situation::Normal);
        assert!(validate_boss_ai(&c, Some(&crt))
            .unwrap_err()
            .to_string()
            .contains("boss AI"));
    }
    #[test]
    fn gigantes_initializes_fifteen_shuffle_draws_and_two_order_slots() {
        let mut c = BattleConfig::from_json(include_str!("../../data/gigantes.json")).unwrap();
        c.players =
            serde_json::from_str(include_str!("../../data/source/lord-sands-party.json")).unwrap();
        for (p, skill) in c.players.iter_mut().zip([
            "attack_sword",
            "elemental_barrage",
            "attack_staff",
            "attack_lance",
        ]) {
            p.skills = vec![skill.into()];
        }
        let b = Battle::new_route_boss(c, None).unwrap();
        let trace = b
            .state()
            .opening_crt
            .as_ref()
            .unwrap()
            .trace
            .as_ref()
            .unwrap();
        assert_eq!(
            trace
                .iter()
                .filter(|t| t.reason == "boss_ai_action_table_shuffle")
                .map(|t| t.after.draws - t.before.draws)
                .sum::<u64>(),
            15
        );
        assert_eq!(
            trace
                .iter()
                .filter(|t| t.reason == "boss_ai_insert_action_order")
                .count(),
            2
        );
        assert_eq!(
            b.state()
                .next_queue
                .iter()
                .filter(|&&id| id == ActorId(4))
                .count(),
            2
        );
    }
    #[test]
    fn live_sands_opening_and_two_slots() {
        let v: serde_json::Value = serde_json::from_str(include_str!(
            "../../data/source/lord-sands-live-20261016.json"
        ))
        .unwrap();
        let mut c = BattleConfig::from_json(include_str!("../../data/gigantes.json")).unwrap();
        c.enemies = BattleConfig::from_json(include_str!("../../data/lord_sands.json"))
            .unwrap()
            .enemies;
        c.players = serde_json::from_value(v["players"].clone()).unwrap();
        c.seed = v["records"][15]["before"].as_u64().unwrap() as u32;
        for (p, skill) in c.players.iter_mut().zip([
            "attack_sword",
            "elemental_barrage",
            "attack_staff",
            "attack_lance",
        ]) {
            p.skills = vec![skill.into()];
        }
        let b = Battle::new_route_boss(c, None).unwrap();
        assert_eq!(
            b.state().rng.state,
            v["records"][77]["before"].as_u64().unwrap() as u32
        );
        assert_eq!(
            b.state().turn_order(),
            [ActorId(0), ActorId(1), ActorId(3), ActorId(2)]
        );
        assert_eq!(
            b.state().next_queue,
            [
                ActorId(0),
                ActorId(2),
                ActorId(1),
                ActorId(3),
                ActorId(4),
                ActorId(4)
            ]
        );
        let players = serde_json::from_value::<Vec<PlayerCharacter>>(v["players"].clone()).unwrap();
        let p = crate::gigantes::predict_sands(
            CrtRng::new(v["records"][15]["before"].as_u64().unwrap() as u32),
            &players,
            &Default::default(),
            0,
        )
        .unwrap();
        assert_eq!(
            p.actions
                .iter()
                .map(|a| (
                    a.turn,
                    a.actor,
                    a.report.rng_before.draws + 15,
                    a.report.rng_after.draws + 15
                ))
                .collect::<Vec<_>>(),
            vec![
                (1, 0, 77, 118),
                (1, 1, 118, 157),
                (1, 3, 157, 181),
                (1, 2, 181, 220),
                (2, 1, 220, 267),
                (2, 0, 267, 308),
                (2, 2, 308, 347),
                (2, 3, 347, 374)
            ]
        );
        assert_eq!(
            p.after.state,
            v["records"][375]["before"].as_u64().unwrap() as u32
        );
    }
}
