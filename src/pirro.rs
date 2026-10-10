//! Source port of BattleAI_Bos_Thi_C01_010. No engine objects or presentation.
use crate::{CrtRng, Result, ensure};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PirroState {
    pub state: u8,
    previous: u8,
    tables: Vec<Vec<u8>>,
    working: Vec<u8>,
    pub second_phase: bool,
    pub(crate) phase_frame_pending: bool,
    pub latent_unlocked: bool,
    pub hold_out: bool,
    used_mercy: bool,
    used_finish: bool,
    used_speed: bool,
    fatal_ability: u8,
    pub omen_pending: bool,
    pub sign_of_boost: bool,
    pub double_action_remaining: u8,
    pub force_miss: bool,
}
fn shuffle(table: &mut [u8], fixed: bool, rng: &mut CrtRng) {
    // GenericArray_Shuffle, 142c4a590: ascending swaps, including final size-1 draw.
    for i in usize::from(fixed)..table.len() {
        let j = i + rng.index((table.len() - i) as u32) as usize;
        table.swap(i, j);
    }
}
impl PirroState {
    pub(crate) fn new(rng: &mut CrtRng) -> Self {
        let mut tables = vec![
            vec![],
            vec![],
            vec![3, 5, 0, 2],
            vec![0, 5, 2],
            vec![0, 5],
            vec![7, 12, 4],
            vec![5, 12, 4],
            vec![1, 4, 12, 5],
            vec![6, 5, 4, 12],
        ];
        for (i, fixed) in [(3, true), (4, false), (6, true), (7, true), (8, true)] {
            shuffle(&mut tables[i], fixed, rng);
        }
        Self {
            state: 1,
            previous: 0,
            tables,
            working: vec![],
            second_phase: false,
            phase_frame_pending: false,
            latent_unlocked: false,
            hold_out: true,
            used_mercy: false,
            used_finish: false,
            used_speed: false,
            fatal_ability: 4,
            omen_pending: false,
            sign_of_boost: false,
            double_action_remaining: 0,
            force_miss: false,
        }
    }
    fn change(&mut self, s: u8) {
        self.previous = self.state;
        self.state = s;
    }
    fn table(&mut self, table: usize, state: u8, reshuffle: Option<bool>, rng: &mut CrtRng) {
        self.working = self.tables[table].clone();
        if let Some(fixed) = reshuffle {
            shuffle(&mut self.working, fixed, rng);
        }
        self.change(state);
    }
    pub(crate) fn choose(
        &mut self,
        boss_hp: f32,
        player_hp: f32,
        hp: u32,
        rng: &mut CrtRng,
    ) -> Result<u8> {
        if self.second_phase && hp == 1 && !self.used_mercy {
            self.hold_out = false;
            self.used_mercy = true;
            self.change(20);
        }
        match self.state {
            1 => self.table(2, 2, None, rng),
            2..=4 => {
                if boss_hp < 0.75 {
                    self.table(5, 5, None, rng);
                } else if self.working.is_empty() {
                    if self.state == 3 {
                        self.table(4, 4, Some(false), rng);
                    } else {
                        self.table(3, 3, Some(true), rng);
                    }
                }
            }
            5 if self.working.is_empty() => self.table(7, 6, Some(true), rng),
            6 | 8 => {
                if player_hp < 0.34 && !self.used_mercy {
                    self.change(10);
                } else if player_hp < 0.25 && self.used_mercy && !self.used_finish {
                    self.change(11);
                } else if boss_hp < 0.5 && !self.used_speed {
                    self.change(7);
                } else if self.working.is_empty() {
                    if boss_hp < 0.5 {
                        self.table(8, 8, Some(true), rng);
                    } else {
                        self.table(7, 6, Some(true), rng);
                    }
                }
            }
            _ => (),
        }
        Ok(match self.state {
            2 | 3 | 4 | 5 | 6 | 8 => {
                ensure(
                    !self.working.is_empty(),
                    "Pirro AI action table exhausted at an unsupported threshold",
                )?;
                self.working.remove(0)
            }
            7 => 11,
            10 => {
                self.hold_out = false;
                8
            }
            11 => 9,
            15 => self.fatal_ability,
            20 => 8,
            _ => {
                return Err(crate::SimError(format!(
                    "unsupported Pirro AI state {}",
                    self.state
                )));
            }
        })
    }
    /// PostUpdateThink runs before the chosen action's sequence.
    pub(crate) fn post_think(&mut self, last_order: bool, rng: &mut CrtRng) {
        match self.state {
            5 if !self.second_phase => {
                self.second_phase = true;
                self.phase_frame_pending = true;
            }
            7 => {
                self.used_speed = true;
                self.omen_pending = true;
                self.change(15);
            }
            8 if self.working.is_empty() => {
                self.omen_pending = true;
                self.fatal_ability = [1, 4][rng.index(2) as usize];
                self.change(15);
            }
            10 => {
                self.used_mercy = true;
                self.change(self.previous);
            }
            11 => {
                self.used_finish = true;
                self.change(self.previous);
            }
            15 => {
                if last_order {
                    self.table(8, 8, None, rng);
                } else {
                    self.fatal_ability = 10;
                }
            }
            20 => self.change(6),
            _ => (),
        }
    }
    pub(crate) fn on_break(&mut self) {
        self.sign_of_boost = false;
        if self.state == 15 {
            self.working = self.tables[8].clone();
            self.change(8);
        }
    }
}

/// The healing branch is frozen from the R8 preview at the R7 menu. The
/// no-grape branch checks the actual latent gauge at the R8 command menu.
#[derive(Debug, Clone, Default, Serialize)]
pub struct PirroRoutine {
    pub heal_before_round_eight: Option<bool>,
    pub latent_used: bool,
    pub latent_delayed: bool,
    pub wind_pending: bool,
    pub ice_pending: bool,
}
impl PirroRoutine {
    pub fn step(&mut self, battle: &mut crate::Battle) -> Result<crate::StepReport> {
        self.step_inputs(battle, None)
    }
    /// Replay the complete boost-button path, including the action's final BP.
    pub fn step_with_boost_inputs(
        &mut self,
        battle: &mut crate::Battle,
        levels: &[u8],
    ) -> Result<crate::StepReport> {
        self.step_inputs(battle, Some(levels))
    }
    fn step_inputs(
        &mut self,
        battle: &mut crate::Battle,
        levels: Option<&[u8]>,
    ) -> Result<crate::StepReport> {
        use crate::{ActorId, Command, Side};
        let actor = battle
            .current_actor()
            .ok_or_else(|| crate::SimError("Pirro battle already ended".into()))?;
        if battle.side(actor) == Some(Side::Enemy) {
            ensure(levels.is_none(), "boost inputs require a player menu")?;
            return battle.step_enemy();
        }
        let round = battle.state().round;
        let mut next = self.clone();
        let skill = |id: &str, bp| Command::Boost {
            id: id.into(),
            target: Some(ActorId(1)),
            bp,
        };
        let item = |id: &str| Command::Item {
            id: id.into(),
            target: if id == "healing_grape" {
                Some(ActorId(0))
            } else {
                None
            },
        };
        let command = match round {
            1 | 2 => skill("ABI_SET_THI_020", 0),
            3 => skill("ABI_SET_THI_020", 3),
            4 | 5 => skill("attack_sword", 0),
            6 => item("shadow_soulstone"),
            _ => {
                if next.heal_before_round_eight.is_none() {
                    ensure(round == 7, "Pirro policy must inspect the preview at R7")?;
                    next.heal_before_round_eight =
                        Some(battle.state().next_queue.first() == Some(&ActorId(1)));
                }
                let delay = u32::from(next.heal_before_round_eight.unwrap());
                if round == 7 && delay == 1 {
                    item("healing_grape")
                } else if round == 7 + delay {
                    item("light_soulstone")
                } else if round == 8 + delay {
                    if next.wind_pending {
                        next.wind_pending = false;
                        item("wind_soulstone")
                    } else {
                        ensure(!next.latent_used, "unexpected third action in latent turn")?;
                        if delay == 0 && !battle.can_use_latent() {
                            // Keep the max-boost sword on R8. Move only latent
                            // activation to R9, pairing Wind with Ice there.
                            next.latent_delayed = true;
                            skill("attack_sword", 3)
                        } else {
                            next.latent_used = true;
                            next.wind_pending = true;
                            Command::Latent {
                                command: Box::new(skill("attack_sword", 3)),
                            }
                        }
                    }
                } else if round == 9 + delay {
                    if next.latent_delayed && !next.latent_used {
                        next.latent_used = true;
                        next.ice_pending = true;
                        Command::Latent {
                            command: Box::new(item("wind_soulstone")),
                        }
                    } else {
                        if next.latent_delayed {
                            ensure(next.ice_pending, "unexpected third action in latent turn")?;
                            next.ice_pending = false;
                        }
                        item("ice_soulstone")
                    }
                } else {
                    ensure(
                        round == 10 + delay,
                        "Pirro routine exhausted without victory",
                    )?;
                    skill("attack_sword", 0)
                }
            }
        };
        let report = match levels {
            Some(levels) => battle.step_player_with_boost_inputs(command, levels)?,
            None => battle.step_player(command)?,
        };
        *self = next;
        Ok(report)
    }
}
