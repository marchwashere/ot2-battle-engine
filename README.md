# OT2BattleEngineRust

A standalone, headless Rust battle simulation core for a small set of OT2
encounters. This is a new repository, independent of the old C++ engine.

**The scaffold can execute actions and finish battles. It does not yet reproduce
OT2 combat or its complete RNG call sequence.** The included encounter is
synthetic, and its required `scaffold_v1` rules flag makes that boundary explicit.

## Run

Requires Rust 1.85 or newer. From this repository:

```sh
cargo run --release -- demo 123
cargo run --release -- run data/demo_battle.json data/demo_commands.json
cargo run --release -- validate-ai data/demo_ai.json
cargo test
```

Use `--offline` after `cargo run` / `cargo test` when dependencies are cached.
No Unreal installation, graphics, audio, decompiler, Node.js, or old engine is
needed to build or run the checked-in crate.

`demo` supplies a simple player policy. `run` reads an array of player commands
and automatically executes intervening enemy turns. Both emit one JSON action
report per line, followed by the final state. A command script may stop at a
player decision without ending the battle; extra commands after battle end are
rejected. Each report contains the CRT state and draw count before and after the
action, including enemy decision draws.

For seed 123, both checked-in examples finish in **19 actions**, with victory in
round 10, player HP 425 / SP 30, and CRT state `3486328484` after 11 draws. This is
a regression fixture for this scaffold, not an in-game trace.

## Implemented

- Player and enemy structs, HP/SP/BP/latent resources, shared skill/item data.
- Battle initialization, player commands, enemy turns, resource costs, target
  validation, KO handling, and victory/defeat.
- Ten recovery items: grapes, plums, and jams, including party recovery.
- Fixed-damage and recovery skill effects, optional weighted hit checks.
- JSON AI state machines with ordered conditions, phases, action sequences,
  weighted choices, and target selection.
- Explicit MSVC CRT `rand` state and draw count; reproducible battle clones for
  exploring alternative commands.
- Invalid individual actions leave battle state and RNG unchanged.

`advance_enemies(limit)` is a bounded convenience loop, not a transaction: if a
later enemy action fails or the budget runs out, earlier successful actions stay
applied. Clone the battle first when the whole sequence needs rollback.

## Rules and accuracy

There are no visual/audio systems or Unreal/CRI random streams. Only the main
CRT stream is modeled. Any future source logic retained solely for CRT advancement
must be supported by a battle/RNG trace.

The CRT generator uses wrapping `state * 214013 + 2531011`, returning bits 16..30.
Its float helper uses the source's binary32 multiplier. Correct generator output
alone does **not** establish the right number or order of calls in a real battle.

The current rules are deliberately small:

| Behavior | `scaffold_v1` |
| --- | --- |
| Turn order | Descending speed, actor ID breaks ties, one action per round; no RNG |
| Damage | Explicit fixed amount; no OT2 damage formula or damage variance |
| Guaranteed hit | No CRT draw |
| Weighted hit | One draw per target if probability > 0, including probability 1 |
| AI sequence | No selection draw |
| AI weighted choice | One draw, even with one choice |
| Random enemy target | One draw, even with one living opponent |
| Recovery | Capped at max HP/SP, BP 5, latent 100; no revival |
| Intrinsic item effects | No draws; complete OT2 item-action callbacks are not ported |
| Wait | Advances the turn; this is not OT2 Defend |

Not implemented: original damage/hit/critical formulas, equipment and jobs,
weakness/shield/break, boost and BP regeneration, statuses, passives, latent
abilities, multiple actions per turn, exact turn order, real boss encounters,
automatic Blueprint AI import, or trace-verified OT2 RNG advancement. Unknown
JSON fields and unsupported effect/AI variants are rejected rather than ignored.

## Items and source data

`data/items.json` stores recovery values extracted from the local
`/Game/Item/Database/ItemDB.ItemDB` asset. It records the source SHA-256 and original
row labels; English display names are assigned by the importer.

| Item ID | Source row suffix | Intrinsic recovery |
| --- | --- | --- |
| `healing_grape` | 0010 | 500 HP |
| `healing_grape_m` | 0020 | 1250 HP |
| `healing_grape_bunch` | 0030 | 900 HP to living allies |
| `inspiriting_plum` | 0040 | 40 SP |
| `inspiriting_plum_m` | 0050 | 100 SP |
| `inspiriting_plum_basket` | 0060 | 45 SP to living allies |
| `refreshing_jam` | 0100 | 100% max HP and SP |
| `rejuvenating_jam` | 0101 | 5 BP and 150 latent, capped by the core |
| `reinforcing_jam` | 0110 | 100% max HP and SP, 5 BP |
| `revitalizing_jam` | 0111 | 100% max HP and SP, 5 BP, 150 latent |

All rows have prefix `ITM_CSM_`. Values above describe this local asset, not an
assertion about every game version or the complete item execution path.

The optional `node tools/import-items.cjs <legacy-repo-path>` build-time tool
regenerates this JSON through the old repository's asset extractor. The runtime
uses the checked-in JSON and has no dependency on that extractor.

## Library API

```rust
use ot2_battle_engine::{Battle, BattleConfig, Command, ItemCatalog};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let json = std::fs::read_to_string("data/demo_battle.json")?;
    let config = BattleConfig::from_json(&json)?;
    let mut battle = Battle::new(config, ItemCatalog::builtin()?)?;
    let player = battle.actor_id("player").unwrap();
    let alternative = battle.clone(); // shares immutable content

    let report = battle.step_player(Command::Item {
        id: "healing_grape".into(),
        target: Some(player),
    })?;
    let enemy_reports = battle.advance_enemies(100)?;
    assert_ne!(battle.state(), alternative.state());
    println!("Player report: {report:?}; enemy reports: {enemy_reports:?}");
    Ok(())
}
```

Actor indices are assigned in player-array order, then enemy-array order. JSON
commands use those numeric indices; Rust callers can resolve IDs with `actor_id`.
Self-target actions accept `null` or the acting actor's index; all-target actions
require `null`. Single-target ally/opponent actions require a living target.

Use the library directly for search, avoiding CLI JSON output. AI and content
references compile to dense indices, and battle clones share definitions through
`Arc`. The initial action API still allocates event reports and some temporary
vectors. A no-trace fast path, benchmarks, and parallel search are future work;
no throughput claim is made yet. State can be serialized for inspection, but
serialized snapshot loading is not implemented; use `Battle::clone` for branches.

## Boss AI input

`data/demo_ai.json` is a synthetic example of the supported normalized AI format.
Embed an AI object in each enemy's `ai` field, as in `data/demo_battle.json`.
`validate-ai` checks structure; battle construction also checks skill references,
learned skills, and target compatibility.

- Each state evaluates rules in order; its last rule must be `always`.
- Conditions: `always`, `hp_at_most { percent }`,
  `round_modulo { divisor, remainder }`, `actions_at_least { count }`.
- Selections: `sequence { actions }` or `weighted { choices }`.
- Actions specify `skill` and `target`. Targets: `self_only`, `first_opponent`,
  `random_opponent`, `lowest_hp_ally`, `all_opponents`, `all_allies`.
- `lowest_hp_ally` compares absolute HP; actor index resolves ties.
- Optional `next_state` takes effect for the next decision. Sequence positions
  persist per state/rule. `actions_at_least` counts the enemy's prior actions.

This parser consumes curated JSON. It does not parse arbitrary decompiler output
or reconstruct original boss scripts automatically. The next encounter should
translate verified source decisions into this format, extending it only for
behavior that encounter needs. The selected first real encounter is **the very
first battle in Throné's Chapter 1**. Shield reduction and Break must be implemented
and verified before that encounter is considered supported. See [PLAN.md](PLAN.md).

## Source layout and checks

`model.rs`: combatants/resources; `content.rs`: skills/items/effects;
`rng.rs`: CRT; `ai.rs`: AI parser/compiler; `battle.rs`: state/action loop;
`main.rs`: CLI. `tests/integration.rs` covers recovery/caps, rejected actions,
AI rollback, RNG draws, phases, KO, full battles, and replay across 64 seeds.

```sh
cargo fmt --all -- --check
cargo clippy --offline --all-targets -- -D warnings
cargo test --offline --release
```
