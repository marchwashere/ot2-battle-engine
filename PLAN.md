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

## Remaining before game-accurate manipulation

- [ ] Capture the opening battle's actual entry state: story gates, night-buff
      activation, party resources, inventory and seed boundary.
- [ ] Complete the **CRT-only projection** of omitted initialization, voice
      selection, notification timers and command callbacks. Implement no playback,
      graphics or Unreal object runtime. See [specific gaps](docs/throne-opening.md).
- [ ] Compare complete game traces action by action: actors, AI choices, targets,
      damage/resources, status lifetimes, current/next queues, CRT states/draws.
- [ ] Confirm source assumptions for long/contrived fights and unusual choices
      against captures, including cover reuse and exhausted enemy SP.
- [ ] Only label a profile exact after those comparisons pass. Reproducible
      simulation and formula fixtures alone do not establish exactness.

## Search and further encounters

- [ ] Add a no-report path and measure release action/branch throughput.
- [ ] Add bounded seed/action search, pruning and parallel execution.
- [ ] Add persisted snapshot loading if needed for resuming searches.
- [ ] Add further encounter adapters and only their required mechanics.

The engine stops at victory/defeat; rewards, overworld progression, other bosses,
later unlocked latent abilities/jobs/passives and unsupported statuses are outside
the implemented opening encounter. No UI/audio/graphics work is planned.
