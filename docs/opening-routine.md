# Opening routine and CRT boundary

`throne [seed] [--trace]` runs the user's policy in `routine.rs` and the scalar
callback projection in `opening_crt.rs`. Only main CRT state is stored. There is
no playback, renderer, widget, particle system, Unreal/CRI RNG or frame loop.

## Supported entry and exit

Entry is immediately before battle actor initialization, after the scripted
formation `ENG_EVE_THI_C01_010` has been chosen. It is not process startup, the
first input menu, or an earlier story-event seed. Default level-one party,
equipment, full HP/SP and 1 BP are assumed. The night talent is disabled by
`MS_TOU_10_0100` setting `BAN_TOU_SPECIAL_ACTION` (652) before the fight.

The supported timing is English and switching 1× to 2× at the first opportunity,
before executing the first attack. By default no extra boost toggles or cancelled
commands are included. A 2-BP action represents two successive boost inputs.

`Battle::boost_cancel_cycle()` explicitly models an extra **0 -> 1 BP -> 0**
cycle at a player command menu before the action's usual boosts. It requires
available BP and the opening CRT profile. The cycle selects the level-one boost
voice, then releases held BP without changing the turn or spending BP. All three
opening actors' level-one candidates have rate 100, are first in type 8, and
are not PlayOnce: one CRT draw per cycle. Decreasing boost adds no CRT draw.
Trace records both the voice and the zero-draw cancellation.

`Battle::boost_sequence(&[1, 2, 1, 0])` additionally supports level-two boosts
when at least 2 BP is available. **1 -> 2 consumes two CRT draws**: the first
candidate's rate check occurs before its level condition rejects it, then the
level-two candidate is checked. Thus `0 -> 1 -> 2 -> 1 -> 0` adds 3 draws.
Sequences validate all one-level transitions and available BP before mutation,
and must return to zero; reductions do not consume CRT.

Sources: `BattleCommandMenuActorBP.OnBoostChange` @078e gates the voice on
`add`, @0798 calls `PlayBattleVoice_Boost`, and @07c6 holds the selected BP.
`BattlePlayerBP.PlayBattleVoice_Boost` calls voice type 8 with the selected level
as Border. The imported `data/opening_crt.json` preserves candidate order/rates.
The assistant searches and replays these cycles; the core has no dependency on
the route calculator. This extra-input path still needs a live-game comparison.

The final endpoint is **after both post-battle encounter-counter resets**.
`after_battle` exposes CRT checkpoints before drops, after drops, after the first
field reset, and after the post-battle event reset. `MS_TOU_10_0300` sets
EncountReset; `EventManager.EndEvent` performs the second reset. Only the CRT
effect is projected, with no cutscene playback. The first and final encounter
counters are retained separately.

## Routine

Party action order is observed as the simulation runs; enemy turns are automatic.
Enemies are numbered **right to left**: `pursuer_a` is enemy 1 (right),
`pursuer_b` is enemy 2 (left). `PMT_TYPE_2_001` places them at (-100, 100, 0)
and (-700, -300, 0), respectively. The importer preserves the placement rows in
`data/source/throne_opening.json`; actor initialization order is unchanged.

| Round | Party action | Target | BP spent |
| --- | --- | --- | --- |
| 1 | First | Enemy 1 (right) | 1 |
| 1 | Second | Enemy 2 (left) | 0 |
| 1 | Third | Enemy 2 (left) | 0 |
| 2 | First eligible actor | Enemy 1 | 2 |
| 2 | Second eligible actor | Enemy 2 | 2 |

The first-round booster is ineligible for the two attacks in round two and
Defends if their turn occurs. Default weapons are dagger, dagger, axe. There is
no fallback retarget or third-round policy: an invalid policy state returns an
error. Rejected commands preserve both battle and policy state.
GUI and CLI exports identify this policy as `first_actor_boosts_enemy_one`.

## Ordered source projection

1. Initialize each player with one timing draw. Initialize each enemy with a
   frame draw, charge-ceiling draw, and timing draw.
2. Sort/shuffle the first order using unbuffed speed. `SetupFirstTurnPlayer`
   promotes its first party member to the front without drawing RNG. Select the
   opening speaker and traverse their voice candidates. The group's
   BattleStartEvent is None; the rich event passes no fixed first character.
3. Skip the night talent for this opening. Promote the first order to the
   current queue, then sort/shuffle the next queue using the initialization
   timing cache. Draw the first actor's order-start candidates. Custom setups
   can explicitly enable the talent; its direct status application and order
   promotion consume no CRT.
4. For each player action, traverse boost candidates for each input, then execute
   the weapon-specific scalar check/voice/damage plan. Enemy AI runs its source
   HP/charge/previous-action branches, selects a target, and resolves damage.
5. Each damage/miss notification schedules a timer that consumes main CRT even
   with zero random variance. Preserve this draw without creating a notification.
   The source damage pool has 50 slots, more than this entire routine needs.
   A breaking hit also schedules a delayed break timer. Hits against an already
   broken target schedule the finish-blow timer unless the break sequence is
   busy or final-hit death has cleared Break. Scalar authored ticks preserve
   these conditions and their order relative to voice checks; no media runs.
6. On a new round, draw one timing revision per living actor, retain the previous
   preview as current order, schedule the BP timer (one CRT draw), and generate
   the following preview. Break excludes
   actors from acting but not all order bookkeeping.
7. On victory, roll one 30% grape-drop check for each pursuer. This group's
   UseVictoryAction is false, so there is no victory voice selection.
8. Reset `Dng_Cty_3_1` encounter steps: draw B then A with RandomInteger(400), then
   truncate `(1000 + A + B) * 0.5`. Scripted event battle entry does not clear
   FirstEncount; the ordinary encounter lottery does.
9. Reset again when the post-battle event with EncountReset=true completes.

Voice candidates roll their probability **before** eligibility tests. A 100%
candidate still draws. PlayOnce candidates are removed after use. Consequently,
healthy Throné's order selection uses 17 draws; each guest's first and second
turn selections use 3. Defend uses 21 ailment draws, one guard-voice draw, then
the next actor's order selection if the battle continues.

The normal-attack plan comes from PLAYER_ATTACK_SWORD/DAGGER/AXE ability sets,
including sequence jumps and cached accuracy refreshes. It is not simply one
accuracy roll per hit. The target remains attached through the final hit even
when an earlier hit reduces HP to zero.

## Why language and speed matter

The source gates weakness/break/kill voice reactions on whether the attack voice
is still playing. Passing that gate traverses candidates using **main CRT**.
Language changes clip duration; battle speed changes when the hit occurs.
Ignoring CRI's internal generator does not remove this main-CRT branch.

For the user's English/2× setup, all available ordinary attack variants last
past the unboosted hit. Thus the branch is resolved without sampling CRI or
tracking audio. Boosted normal attacks disable these reactions by a separate
source condition.

| Actor | Shortest English clip | Attack voice to hit at 2× |
| --- | --- | --- |
| Throné | 0.397667 s | 0.183333 s |
| Pirro | 0.279521 s | 0.183333 s |
| Scaracci | 0.425396 s | 0.266667 s |

These bounds describe uninterrupted ordinary actions, with over 1/30 second of
margin for every variant. Pauses or interrupted actions are outside this timing
profile. Startup at 1× does not change the per-attack bound when speed is changed
before the first attack.

## Evidence and remaining verification

The current routine was revised to boost the first actor on enemy 1. The live
capture below used the previous routine (first/third hit enemy 1 unboosted;
second hit enemy 2 with 1 BP). `tests/live_opening.rs` now replays that capture's
explicit actions independently of the automatic routine. Its observed numbers
have not been replaced with predictions from the revised routine.

See [the recorded case](live-opening-4929.md): the corrected input is seed 0
advanced 4929 calls. The first menu matches `0xF485D421`, and party orders match
Pirro/Throné/Scaracci then Scaracci/Throné/Pirro. After source-backed timer and
endpoint corrections, all 154 CRT calls match the captured replay in state,
order and native caller category. Final state is `0x194611CD`. Disabling the
incorrect night talent also matches the full reported first-round queue, Pirro's
31 damage and Throné's broken-target 50. Throné's first hit is still 33 versus
reported 31; remaining damage amounts and other seeds need live comparison.

`data/source/opening_crt.json` stores table rows, source hashes, extracted plans
and event timing. `data/source/opening_timing.json` stores English waveform
duration metadata and hashes. Regeneration uses the read-only parent workspace:

```sh
node tools/import-throne.cjs
node tools/import-opening-crt.cjs
python tools/audit-opening-timing.py
```

Relevant decoded functions in the parent's `tools/speedrun_audit/` are:

- BattleManagerBP Initialize/Start/BeginTurn and ExecSpecialAction_BuffInNight.
- BattleActionOrderBP InitializeActionOrder, MoveActionOrder, ChangeBattleTurn.
- BattleCharacterBaseBP OnTakeDisease_ForActionOrder and OnHitDamage callbacks.
- Battle voice component PlayBattleVoice, OrderStartVoice, and damage reactions.
- LevelManagerBP On Shown Field After Battle Impl and EncountManagerBP
  CalcEncountStepCount/ResetEncountStep/StartEventBattle paths.

Parent captures `docs/defend_rng_trace.md` and
`docs/deserted_highroad_cait_4940_trace.md` support shared CRT paths. They are
captures of other battles, not ground truth for this opening routine.

For seed 123, the revised routine predicts six party actions plus one enemy
action, victory in round 2, 35 damage taken by Pirro, one drop, and CRT state
3217448444 after 163 draws. The final counter is 639. Trace-on/off and
cloned/restarted runs agree; tests cover
all six party orders and both Defend/no-Defend paths.

**One historical CRT replay is verified; the revised routine still needs live
comparison.** Compare
the entry seed and every action's actor, target, damage, queue and CRT state,
then both reward and field-reset checkpoints. Internal replay and seed sweeps
show consistency, not equivalence to the game. Other commands remain available
in `Battle::new`, but the full-callback constructor rejects unsupported skills,
items, Wait and weapon changes. Longer fights and altered loadouts have not been
validated by this routine audit.
