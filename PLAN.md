# Implementation checklist

The immediate deliverable is a runnable Rust foundation. Exact simulation should
grow around the few required encounters, with CRT traces as the acceptance test.

## 1. Barebones core — complete

- [x] New standalone Git repository and Rust library/CLI.
- [x] Player/enemy definitions and separate mutable combatant state.
- [x] Skill/item definitions and imported recovery-item data.
- [x] Explicit per-battle CRT generator, state, and draw count.
- [x] Validated actions that run to completion, costs and inventory consumption.
- [x] Basic turns, KO, victory, and defeat under explicit scaffold rules.
- [x] Parsed/validated AI JSON with phases, sequences, weighted choices, targets.
- [x] Cloned branches and deterministic replay tests.
- [x] A runnable full synthetic battle, including items and enemy actions.

## 2. First real encounter

- [ ] Identify the first boss, game build/data revision, party setup, and required
      player actions; record original script/skill IDs.
- [ ] Translate that boss's decision logic, including action counters, phase
      changes, target ordering, and conditional random calls.
- [ ] Implement the damage, defense, accuracy, variance, shield/break, boost, and
      turn-order mechanics that its permitted action set actually uses.
- [ ] Account for battle-affecting passives/statuses and CRT-consuming callbacks
      on those paths. Omit graphics/audio output and unrelated random streams.
- [ ] Compare a known battle trace action by action: actor, selected skill,
      target, damage/resources, AI state, CRT state, and number of draws.
- [ ] Gate an exact encounter profile on passing those reference fixtures; keep
      the synthetic scaffold profile separate.

## 3. Search and more encounters

- [ ] Add a no-trace execution path and measure action/branch throughput in
      release builds. Keep exact and fast paths behaviorally equivalent.
- [ ] Add search over supplied player actions/seeds with bounded depth, branch
      pruning, reproducible results, and shared immutable encounter data.
- [ ] Add snapshot persistence if searches need to resume across processes.
- [ ] Add encounter-specific import adapters and reusable mechanics as demanded
      by further bosses; reject unsupported source opcodes explicitly.

No Unreal runtime, visual/audio implementation, or general decompiler interpreter
is part of this plan. Exactness and performance claims require measured evidence.
