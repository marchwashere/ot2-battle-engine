# Turn-order controls

Source audit: `node tools/audit-turn-order.cjs` extracts hashed cooked Blueprint
functions to `data/source/turn-order-controls.json`. Native enum identities were
checked with the reference tool `enums.load` against the reviewed executable.

The ordinary battle rules implemented by the shared core are:

- Defend marks the actor's next row and invokes GUARD (order type 3). The latest
  Defend precedes earlier Defends and Springy Boots recipients. GUARD protection
  ends with the generated queue; retaining damage reduction does not preserve
  priority in a newly generated queue.
- Springy Boots is disease 15, not battle-start disease 92. Applying it moves
  the next row to FIRST. On later previews, BattlePlayerBP's event at 23419 adds
  20,000 to BaseWaitTime before sorting. CheckShuffleEnable excludes recipients
  from random swaps (and skips those RNG calls). There is no post-shuffle stable
  partition for Boots. The already generated current queue survives expiration;
  new previews use the remaining status duration.
- Recovering broken enemies stay before these promotions. Ordinary single-order
  profiles do not manufacture BREAK_STYLE (6) rows; that enum is not Defend.
- BreakActionOrder removes outstanding current and next entries and appends hidden
  BREAK (7) entries immediately, without RNG. Delays respect these reserved rows.
- OrderCountControl marks a pending current row, or the next row if the target
  already acted. It does not move both. Multi-target movement traverses forward
  for advancement, backward for delay, tightening boundaries as rows are moved.

Regression coverage includes both Boots/Defend sequences, reversed order after
four Defends, no persistent Defend priority, protected break recovery, Boots
shuffle draws and expiration, immediate break insertion, and single-/multi-target
shifts. Existing seven-Defend A Step Ahead tests, native sort oracle fixtures,
captured game traces, Chubby Cait and Lord of the Sands routines also run.

These are source-derived corrections. New Boots/Defend combinations have not
yet been captured in the retail game. Special boss multi-action ordering, ExVide
and extra-party modes remain outside the bounded supported profiles.
