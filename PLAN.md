# Implementation checklist

## Runnable core — complete

- [x] Standalone Rust repository, library and CLI.
- [x] Player/enemy definitions and separate mutable combat state.
- [x] Skills/items, parsed AI, explicit CRT state, deterministic branches.
- [x] Validated actions, costs, inventory, KO, victory and defeat.
- [x] Grapes, plums, jams and party recovery.

## Throné Chapter 1 opening — implemented and runnable

- [x] Select `ENG_EVE_THI_C01_010`, two `ENE_EVE_THI_CTY_010` pursuers.
- [x] Import original formation, default level-one party, equipment and abilities.
- [x] Preserve source rows and asset SHA-256 provenance.
- [x] Physical/magical damage, hit, critical, variance, weakness and caps.
- [x] Shields, per-hit Break, skipped actions, recovery and priority.
- [x] Boosted attacks/skills, BP consumption/regeneration, SP costs.
- [x] Defend, night buffs, Sharpen Dagger, Heckle, Cover Me! and Steal.
- [x] Pursuer AI: strict HP threshold, probability/counter ordering, charge reset.
- [x] Current/next order, native small-array sort and conditional shuffle.
- [x] Source-derived CRT calls in those implemented combat paths, including
      empty ailment slots and fixed-output ranges.
- [x] Replayable command script; complete victories over 128 seeds, defeat path,
      every starting skill at every boost tier, transactional error checks.
- [x] User's two-round default-weapon routine, including Defend clarification.
- [x] Main-CRT initialization, voice candidates, notification timers, weapon-specific
      hit/check order and overkill callbacks for that routine.
- [x] Apply night talent after first shuffle and move affected actors one slot.
- [x] Item-drop checks and the first post-battle field encounter-counter reset.
- [x] English/2× reaction-gate duration audit; no CRI RNG or audio runtime.
- [x] Optional labeled CRT trace and deterministic restart from another seed.
- [x] Routine replay tests across 1,024 seeds; release sweep across 65,536 seeds.
- [x] User-requested local result viewer: seed entry, action details, CRT checkpoints,
      full trace, copy/export and a Windows launcher; separate from the headless core.

## Remaining before game-accurate manipulation

- [ ] Capture the opening battle's actual entry state: story gates, night-buff
      activation, party resources, inventory and seed boundary.
- [ ] Compare complete game traces action by action: actors, AI choices, targets,
      damage/resources, status lifetimes, current/next queues, CRT states/draws.
- [ ] Verify the boundary through the first field reset; model following story
      event resets separately if prediction is needed after those cutscenes.
- [ ] Extend callback coverage to skills/items only when needed; the full-CRT
      opening constructor currently rejects these commands.
- [ ] Confirm source assumptions for long/contrived fights and unusual choices
      against captures, including cover reuse and exhausted enemy SP.
- [ ] Only label a profile exact after those comparisons pass. Reproducible
      simulation and formula fixtures alone do not establish exactness.

## Search and further encounters

- [x] Add a bounded seed sweep and measure release battle throughput.
- [ ] Add a no-report path to eliminate report allocation during search.
- [ ] Add bounded seed/action search, pruning and parallel execution.
- [ ] Add persisted snapshot loading if needed for resuming searches.
- [ ] Add further encounter adapters and only their required mechanics.

The opening routine ends after grape-drop checks and the first field counter
reset. Later cutscenes/field movement, XP/level-ups, other bosses, later unlocked
latent abilities/jobs/passives and unsupported statuses remain outside scope.
The result viewer is separate from the engine. No game rendering or audio is planned.
