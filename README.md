# OT2BattleEngineRust

A standalone, headless Rust simulator for selected Octopath Traveler II battles.
**Throné's Chapter 1 opening battle now runs from initialization to victory or
defeat**, with player commands and automatic enemy AI. The encounter is
`ENG_EVE_THI_C01_010`: Throné, Pirro and Scaracci against two pursuers.

The engine implements source-derived combat mechanics. **It is not yet a
trace-verified RNG manipulation tool.** Some game callbacks consume the main CRT
stream even with their presentation removed; those calls are not yet fully
modeled. See [encounter details and accuracy gaps](docs/throne-opening.md).

## Run

Requires Rust 1.85 or newer. No Unreal, graphics, audio, Node.js, or old C++ engine
is needed to build or run the checked-in crate.

```sh
cargo run --release -- throne 123
cargo run --release -- run data/throne_opening.json data/throne_commands.json
cargo test --release
```

Add `--offline` before `--` if dependencies are cached. `throne` uses a simple
policy: attack the first living enemy's weakness, then boost against broken
enemies. `run` consumes supplied player commands and executes intervening enemy
turns. Each emits JSON action reports and a final state.

The seed-123 example wins in **6 actions, round 2**, with party HP
**213 / 241 / 237** and CRT state **4014490537 after 46 modeled draws**. This is a
simulator regression result, not a captured in-game result.

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

Only the main MSVC CRT generator is present. There are no Unreal/CRI random
streams, audio output, rendering, animation, UI, or frame loop.

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

The default inventory is empty. Set `inventory` in a copied config to the owned
items, for example `{"healing_grape": 2, "inspiriting_plum": 1}`. Steal can add
grapes during battle. No revival items or post-victory rewards are implemented.

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
are shared through `Arc`, and references compile to dense indices. Reports and
temporary target lists still allocate; a no-report fast path and throughput
benchmark remain on [the checklist](PLAN.md). Snapshot loading is not implemented;
use `Battle::clone()` for in-process branches.

```sh
cargo fmt --all -- --check
cargo clippy --offline --all-targets -- -D warnings
cargo test --offline --release
```

Tests cover arithmetic fixtures from the original compiled Blueprint, each
starting skill/boost tier, shields/Break/recovery, AI conditions and draw order,
items, invalid-action rollback, full victory/defeat, and replay across 128 opening
seeds plus the scaffold fixtures. These are source and simulator checks, not
live-game equivalence tests.
