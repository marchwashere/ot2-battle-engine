# OT2BattleEngineRust

A standalone, headless Rust simulator for selected Octopath Traveler II battles.
The [Scholar job helpers](docs/scholar-jobs.md) apply source-derived stat and
equipment changes to carried characters and preserve job-aware level growth.
The [Hikari recruitment fight](docs/hikari-recruitment.md) accepts the carried
Throné/Osvald/Agnea/Partitio party: regular Ice Soulstone, then the next player's
default attack. It returns complete CRT, resources, enemy history and rewards.
Use `cargo run --example hikari_recruitment -- <seed>` for a standalone trace.
The [second Chapter 1 sewer fight](docs/throne-second.md) is also available:
level-two party, Pursuer Leader, Darkest Night, Pierce Through and 2-BP Slice.
Use `ot2-sim throne-second [seed]` or `throne-second-sweep <seed> <count>`.
Its routine wins all 4,096 regression seeds; its CRT timing awaits live validation.

**Throné's Chapter 1 opening battle now runs from initialization to victory or
defeat**, with player commands and automatic enemy AI. The encounter is
`ENG_EVE_THI_C01_010`: Throné, Pirro and Scaracci against two pursuers.

The engine also models the source's main-CRT callbacks, item drops and both
post-battle encounter-counter resets. **All 154 CRT calls match the historical
seed 0 + 4929 replay using its original actions.** The current routine boosts
the first actor instead and still needs a live comparison. The erroneous opening night buff is
removed: full first-round order, Pirro's 31 damage and Throné's second-hit 50
match. Throné's first hit remains 33 versus reported 31; other damage and seeds
still need comparison. See the [recorded comparison](docs/live-opening-4929.md), [routine and seed boundary](docs/opening-routine.md)
and [combat mechanics](docs/throne-opening.md).

## Run

Requires Rust 1.85 or newer. No Unreal, graphics, audio, Node.js, or old C++ engine
is needed to build or run the checked-in crate.

### GUI and route assistant

The GUI, field encounter predictor and route orchestration now live in the
separate sibling [OT2SpeedrunAssistant](../OT2SpeedrunAssistant/README.md) repository.
It imports this crate through Cargo. This battle library has no reverse dependency
and can be built, benchmarked or used for brute-force sweeps independently.

The existing **Open Battle Viewer.cmd** still works and forwards to the assistant.
For the GUI, build/run from that repository; this crate contains only the library
and `ot2-sim` command-line binary. External callers can load the bundled opening
configuration with `BattleConfig::throne_opening(seed)` without copying data files.

### Command line

```sh
cargo run --release -- throne 123
cargo run --release -- throne 0x7b --trace
cargo run --release -- throne-sweep 0 65536
cargo test --release
```

Add `--offline` before `--` if dependencies are cached. `throne` follows the user's
routine using each actor's default weapon:

- Enemies are numbered right to left: enemy 1 is the right-hand pursuer;
  enemy 2 is the left-hand pursuer.
- Round 1 party actions: enemy 1 with 1 BP (two hits); enemy 2 unboosted;
  enemy 2 unboosted.
- Round 2: the two actors who attacked enemy 2 spend 2 BP on enemy 1 and enemy 2,
  respectively, in their actual turn order. The first-round booster Defends if needed.

Enemy turns execute automatically. English voices and switching from 1× to 2×
before the first attack are the supported timing conditions. The program emits
JSON action reports and the final state; `--trace` adds labeled CRT spans.
`throne-sweep` suppresses per-action output and reuses compiled content.

The seed-123 routine wins in **6 party actions plus 1 enemy action, round 2**,
with party HP **250 / 240 / 237**. After two drop checks and both counter resets,
CRT state is **3217448444 after 163 modeled draws**, and the encounter counter is
**639**. These seed-123 results are internal regressions, not a captured game trace.

## Implemented

- Imported opening formation, level-one stats, equipment, weaknesses, shields,
  ability coefficients and boosted tiers, with original row IDs and SHA-256s.
- Physical/magical damage, binary32 arithmetic, accuracy, critical hits,
  variance, weapon/element weaknesses, and the damage cap.
- Enemy shield reduction per successful weak hit; Break cancels remaining
  actions that round and all actions next round; recovery restores shields.
- Separate damage for the breaking hit and subsequent hits against Break.
- Boost levels 0–3, normal attack hit counts, SP costs, BP regeneration and
  suppression after boosting.
- Current and next turn queues, source-derived sort/shuffle, break recovery
  priority, Defend's defense modifier and next-turn promotion.
- Steal, Darkest Night, Slice, Sharpen Dagger, Cover Me!, and Heckle.
- Night attack/magic/speed buffs, status durations and stacking, cover redirection.
- Pursuer Attack/Slash AI, HP threshold, no consecutive Slash, randomized BP
  charge ceiling/reset, and random living targets.
- Ten recovery items: grapes, plums and jams; consumption, recovery caps,
  and party recovery that skips KO characters.
- KO, victory/defeat, deterministic clones, and rejected-action rollback.
- Weapon-specific cached-hit schedules, remaining hits after a mid-action KO,
  main-CRT voice candidate checks and damage/miss timer draws.
- User routine, optional full draw trace, post-battle drop/counter checkpoints,
  and a release seed sweep.

The simulation core uses only the main MSVC CRT generator, with no Unreal/CRI
random streams, audio output, rendering, animation, game UI or frame loop. The
optional result viewer is a separate inspection shell.

## Supply actions

Actor indices follow the config's players, then enemies:
`0` Throné, `1` Pirro, `2` Scaracci, `3` pursuer A, `4` pursuer B.

```json
[
  { "kind": "skill", "id": "ABI_SET_THI_020", "target": 3 },
  { "kind": "boost", "id": "attack_axe", "target": 3, "bp": 1 },
  { "kind": "defend" }
]
```

Commands apply to the next player who actually gets a turn. Use the library to
choose commands based on `current_actor()`. Self/all-target skills accept `null`;
Cover Me! requires another living ally. `Wait` is a simulator testing operation,
separate from Defend. Escape is disabled for this encounter.

`Battle::new` and `run <battle.json> <commands.json>` expose the broader combat
core, without the opening CRT callback projection. `Battle::new_opening` adds
that projection for default-weapon attacks, Defend and the two pursuer actions;
it rejects skills/items whose callback schedules are not covered. The previous
auto-attack policy remains available as `throne-combat [seed]`.

The default inventory is empty. Set `inventory` in a copied config to the owned
items, for example `{"healing_grape": 2, "inspiriting_plum": 1}`. Steal can add
grapes during battle. The opening routine awards modeled grape drops; no revival
items, XP/level-up processing or other rewards are implemented.

```rust
use ot2_battle_engine::{Battle, BattleConfig, Command, ItemCatalog};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = BattleConfig::from_json(&std::fs::read_to_string(
        "data/throne_opening.json",
    )?)?;
    let mut battle = Battle::new(config, ItemCatalog::builtin()?)?;
    battle.advance_enemies(100)?;
    let actor = battle.current_actor().unwrap();
    let branch = battle.clone(); // mutable state copied; definitions shared
    let report = battle.step_player(Command::Defend)?;
    println!("{actor:?}: {report:?}");
    assert_ne!(battle.state(), branch.state());
    Ok(())
}
```

Individual failed actions preserve all state, AI memory and RNG. The bounded
`advance_enemies(limit)` helper keeps earlier successful actions if a later
action fails; clone first if the whole batch needs rollback.

## Data, AI and scope

`data/throne_opening.json` is ready-to-run normalized content.
`data/source/throne_opening.json` records original rows and asset hashes.
`node tools/import-throne.cjs` can regenerate both using the local reference
workspace. This optional import step has no runtime dependency.
`node tools/import-opening-crt.cjs` imports scalar callback candidates and weapon
plans. `python tools/audit-opening-timing.py` checks the English duration bounds
from metadata only. Their source records are in `data/source/`.

`data/items.json` contains ItemDB recovery values and its source hash. IDs:

| ID | Recovery |
| --- | --- |
| `healing_grape` / `healing_grape_m` | 500 / 1250 HP |
| `healing_grape_bunch` | 900 HP to living allies |
| `inspiriting_plum` / `inspiriting_plum_m` | 40 / 100 SP |
| `inspiriting_plum_basket` | 45 SP to living allies |
| `refreshing_jam` | Full HP/SP |
| `rejuvenating_jam` | 5 BP, latent recovery capped at 100 |
| `reinforcing_jam` | Full HP/SP, 5 BP |
| `revitalizing_jam` | Full HP/SP, 5 BP, latent recovery capped at 100 |

The general AI parser accepts normalized JSON state machines with ordered
conditions, sequences, weighted choices and phase transitions. `opening_guard`
implements the source decision branches used by these two enemies. Arbitrary
Blueprint boss scripts are not automatically translated.

The old synthetic example remains available:

```sh
cargo run --release -- demo 123
cargo run --release -- validate-ai data/demo_ai.json
```

`scaffold_v1` uses fixed damage and deterministic speed order. `ot2_opening_v1`
requires combat profiles and supports up to eight combatants, using the native
small-array sorting path. Neither profile implies support for every OT2 mechanic
or encounter. Unknown JSON fields and unsupported effects are rejected.

## Development and performance

`combat.rs` holds arithmetic and order helpers; `battle.rs` executes actions;
`ai.rs` compiles AI; `content.rs` and `model.rs` hold data; `rng.rs` owns CRT state.
Use the library directly for search to avoid CLI JSON I/O. Immutable definitions
are shared through `Arc`, and references compile to dense indices.
`Battle::restarted(seed)` starts another seed without reparsing content or AI;
`Battle::clone()` branches from the current state. Reports and temporary target
lists still allocate. A no-report fast path, parallel search and persisted
snapshot loading remain on [the checklist](PLAN.md).

A local single-thread release sweep completed 65,536 battles in 0.194 seconds
(about 338,000 battles/second), with all victories. This measures the modeled
routine with tracing/output disabled, not live-game accuracy or a portable
performance guarantee. `data/throne_commands.json` is a replayable seed-123
combat-only example for `run`; use `throne` for the user's routine and CRT callbacks.

```sh
cargo fmt --all -- --check
cargo clippy --offline --all-targets -- -D warnings
cargo test --offline --release
```

Tests cover arithmetic fixtures from the original compiled Blueprint, each
starting skill/boost tier, shields/Break/recovery, AI conditions and draw order,
items, invalid-action rollback, full victory/defeat, and 1,024 routine seeds
covering every party order, enemy turns and Defend. The release sweep additionally
checks 65,536 seeds. These are source and simulator checks, not live-game
equivalence tests.

Pirro's Chapter 1 boss routine is available through `Battle::new_pirro` and
`pirro::PirroRoutine`. Run `cargo run --example pirro -- 0 5595` to inspect a
standalone replay. See [the profile and limits](docs/throne-pirro.md); live CRT
validation is pending and the supplied routine can lose on some rolls.
