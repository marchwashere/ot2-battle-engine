# Throné Chapter 1 opening encounter

The mechanical simulation runs under `ot2_opening_v1`. That name means
source-derived opening mechanics, **not end-to-end verified game RNG**.

## Formation and initial state

Group `ENG_EVE_THI_C01_010` (ID 4597) contains two `ENE_EVE_THI_CTY_010` pursuers.
Each has HP 185, SP 50, one shield, ATK 97, DEF 80, MATK 92, MDEF 80,
accuracy/evasion 80, critical 90, speed 80, FirstBP 0 and charge base 2.
Weaknesses: sword, dagger, axe and dark. Escape is disabled; items, guard and
boost are enabled. The later Pirro boss group `ENG_BOS_THI_C01_010` is not used.

| Index | Player record | HP/SP | Weapons | Starting skills |
| --- | --- | --- | --- | --- |
| 0 | eTHIEF / Throné | 250 / 40 | Sword, dagger | Steal; Darkest Night |
| 1 | eGUEST_006 / Pirro | 275 / 45 | Dagger | Slice; Sharpen Dagger |
| 2 | eGUEST_007 / Scaracci | 237 / 35 | Axe | Cover Me!; Heckle |

Import uses `InitLevel`, `FirstEquipment`, character parameter revisions, growth
rows and job abilities with `GetParam=1`. The base job revision is not multiplied
twice. Armor contributes to base stats; the selected weapon contributes on attack.

The checked-in setup assumes full HP/SP, 1 BP, zero latent gauge and three turns
of night ATK/MATK/AGI buffs. Inventory is empty until supplied or items are stolen.
These are explicit setup assumptions from default records and night-talent
behavior; the opening story event has not been replayed to verify every entry
field. Initial resources, equipment, statuses and inventory are configurable.

`seed` initializes the modeled headless setup. It is not a game-process startup
seed or a captured seed at the first command menu.

## Actions

| ID | Effect at boost 0/1/2/3 |
| --- | --- |
| attack_sword / attack_dagger / attack_axe | 1/2/3/4 hits |
| ABI_SET_THI_010 | Steal, 2 SP; success bonus 4/14/34/94 |
| ABI_SET_THI_020 | Darkest Night, 7 SP; dark, invocation 130%, ability ratio 100/200/300/400% |
| ABI_SET_GST_0170 | Slice, 8 SP; dagger, invocation 110%, ability ratio 100/200/300/400% |
| ABI_SET_GST_0180 | Sharpen Dagger, 3 SP; self ATK up for 3/5/7/9 turns |
| ABI_SET_GST_0200 | Cover Me!, 4 SP; another ally receives caster's next 1/2/3/4 incoming hits |
| ABI_SET_GST_0210 | Heckle, 6 SP; DEF down for everyone except caster, 1/2/3/4 turns |
| ABI_ENE_0041 | Enemy Attack; 0 SP, physical invocation 100% |
| ABI_ENE_0500 | Enemy Slash; 5 SP, physical invocation 120% |

Normal attacks use ability ratio 80%, hit bonus 20 and critical power 125%.
Other player physical moves use critical power 125%; these enemy moves use 150%.
Costs are paid once per action. Steal chance is
`1 - HP/maxHP + .01 + bonus*.01`, with one item available per enemy.

Defend applies guard value 130 in the defense term and promotes the actor in
the next queue. It ends when that actor next acts. `Wait` is a testing helper.
Items restore capped resources and never revive. Latent actions are not exposed
for this opening encounter. Battle execution ends before rewards/drop RNG.

## Damage, Break and turns

Preserve each original Blueprint float32 operation in this order:

```text
base = attack * (invocation * .01) / 2 - defense * guard / 4
damage = base
       * resistance * critical_power * variance
       * outgoing_modifier * incoming_modifier
       * (ability_ratio * .01)
       * attack_status_ratio * defense_status_ratio
       * (growth_stat * .0078125 + .5)
result = clamp(truncate(damage), 1, 9999)
```

For this loadout, outgoing modifier is 1. Weakness is 1.3; double weakness is
1.3×1.3. Against Break, resistance is 1 and incoming modifier is 2. ATK/MATK up
and physical DEF down each contribute 1.5. Guard is 1.3 in the defense term,
not a flat 30% final damage reduction. Variance is `1.01+(1.05-1.01)*CRT_unit`.

Accuracy: `clamp(50*(3-EVA/ACC)+HitRatio,20,100)/100`.
Critical: `clamp((CON-target_AGI)/2+CriticalRatio,0,100)/100`, including speed buffs.
Magic skips ordinary accuracy/critical checks. Other physical skills test critical
first; a critical or broken target skips accuracy. Normal attacks have a separate
preliminary/cached accuracy path.

A successful weak hit removes one shield; misses/neutral hits remove none. The
breaking hit uses pre-break damage, while subsequent hits use broken damage.
Break cancels the enemy's remaining actions this round and the whole next round.
At round N+2 it recovers full shields and receives recovery priority. KO actors
stop acting/receiving further hits.

Boost spends up to 3 BP and uses that tier's original coefficients/duration/hits.
Boosting suppresses the next BP regeneration; otherwise players regain 1 per
round, capped at 5. Status counters tick at subsequent round starts, with
extensions capped at 9.

Order uses the native <=8 selection sort, including unstable ties, and backward
sliding-triple shuffle. The next-round preview is retained. Enemy next-turn speed
uses a 0.7–1.5 revision after round one; initialization uses 1–1 for both sides.
Break recovery adds 1,000,000 to the next-turn score before eligible shuffles.

## Pursuer AI

NormalEnemy slots 0 and 1 are Attack; slot 7 is Slash. A charged enemy rerolls
its ceiling in [2,4], resets BP and uses Attack. Otherwise, strictly below 60% HP,
it rolls 60% for Slash **before** checking that the previous action was not Slash.
Failed/ineligible choices use Attack. Targets are random living opponents,
including a draw when only one remains. This implements this row's populated
slots, not all possible NormalEnemy behaviors.

## CRT coverage and remaining accuracy work

CRT uses wrapping `214013*state+2531011`, bits 16..30 and the original float32
scale. Implemented calls cover charge initialization/reset, timing revisions,
order shuffle, AI choices/targets, accuracy, critical, variance and Steal.
Defend, Sharpen Dagger and each Heckle target consume one 100% status check plus
20 duration draws: nineteen empty AddDisease entries still draw a [0,0] range.
Cover uses its separate direct path.

**Known omissions prevent seed-for-seed game predictions today:**

- Full entry-path CRT ordering, including actor initialization callbacks and
  story-event/night-buff ordering, has not been reproduced against a capture.
- Main-CRT voice candidate checks and damage/miss notification timer draws are
  not integrated. They must be projected as draws only; no playback, widgets,
  animation runtime or rendering is required. Unreal global/CRI RNG stays excluded.
- The synchronous action loop still needs full-action comparison of command
  callbacks, all boosted sequence branches and KO callbacks.
- Long-fight edge cases need original-game comparison, including cover reuse and
  exhausted enemy SP. Currently unaffordable enemy actions reject transactionally
  rather than choosing an invented fallback.

Other encounters/jobs/passives/status families, later unlocked latent actions,
reward processing and overworld progression are outside the implemented scope.

## Evidence and verification

`data/source/throne_opening.json` preserves selected rows and hashes for the
nine tables used by the importer. Regeneration rejects unexpected formation,
status RNG layout, ailments and target variants. Original data is needed only
for optional regeneration, not building or running the simulator.

Read-only parent-workspace references:

- `docs/damage_formula_verified.md`: arithmetic and surrounding hit pipeline.
- `tools/speedrun_audit/Library__LibAilment.txt`: status application, Steal, cover.
- `tools/speedrun_audit/Battle__BP__BattleCharacterBaseBP.txt`: shields, damage,
  Break counters, statuses; adjacent player/enemy traces supply turn behavior.
- `tools/speedrun_audit/Character__BP__BattleAI__GeneralEnemy__AI_Pettern__NewAIPattern__BattleAI_NormalEnemy.txt`:
  decision branches.
- `OT2BattleEngine/include/ot2/native_order_sort.hpp`: native sort/shuffle.
- `OT2BattleEngine/tests/damage_tests.cpp`: original compiled-Blueprint arithmetic
  expectations reused by Rust tests.
- `docs/defend_rng_trace.md`: captured ailment-slot draws and additional voice
  draws; only the ailment portion is implemented here.
- `docs/deserted_highroad_cait_4940_trace.md`: captured initialization/shuffle
  evidence for another encounter, not a capture of this opening fight.

Tests exercise all imported skills/boost levels, weak/neutral/missed hits,
mid-action Break, cancellation/recovery, status stacking, cover, Steal, BP rules,
conditional AI draws, rollback, all-wait defeat and victories/replay for 128 seeds.
Release/debug CLI output is compared for the supplied seed-123 command script.
These checks establish runnable mechanics and source regressions, not complete
live-game equivalence.
