# Opening routine and CRT boundary

`throne [seed] [--trace]` runs the user's policy in `routine.rs` and the scalar
callback projection in `opening_crt.rs`. Only main CRT state is stored. There is
no playback, renderer, widget, particle system, Unreal/CRI RNG or frame loop.

## Supported entry and exit

Entry is immediately before battle actor initialization, after the scripted
formation `ENG_EVE_THI_C01_010` has been chosen. It is not process startup, the
first input menu, or an earlier story-event seed. Default level-one party,
equipment, full HP/SP, 1 BP and the night talent are assumed.

The supported timing is English and switching 1× to 2× at the first opportunity,
before executing the first attack. No extra boost toggles or cancelled commands
are included. A 2-BP action represents two successive boost inputs.

The final endpoint is the **first field encounter-counter reset after victory**.
`after_battle` exposes CRT checkpoints before drops, after drops, and after that
reset. The following story cutscene is not executed. `EventManager.EndEvent`
can reset encounters again when its EncountReset flag is set; a seed requested
after that cutscene is a different boundary.

## Routine

Party action order is observed as the simulation runs; enemy turns are automatic.

| Round | Party action | Target | BP spent |
| --- | --- | --- | --- |
| 1 | First | Enemy 1 | 0 |
| 1 | Second | Enemy 2 | 1 |
| 1 | Third | Enemy 1 | 0 |
| 2 | First eligible actor | Enemy 1 | 2 |
| 2 | Second eligible actor | Enemy 2 | 2 |

The round-one enemy-2 attacker is ineligible for the two attacks in round two and
Defends if their turn occurs. Default weapons are dagger, dagger, axe. There is
no fallback retarget or third-round policy: an invalid policy state returns an
error. Rejected commands preserve both battle and policy state.

## Ordered source projection

1. Initialize each player with one timing draw. Initialize each enemy with a
   frame draw, charge-ceiling draw, and timing draw.
2. Sort/shuffle the first order using unbuffed speed. Select the opening speaker
   and traverse their voice candidates. The group's BattleStartEvent is None.
3. Apply the night talent directly, without the probabilistic ailment path.
   AGI_UP marks party order rows; `ExecMoveNextOrder(0,-1)` moves each forward
   one slot. Promote this list to the current queue, then sort/shuffle the buffed
   next queue. Draw the first actor's order-start candidates.
4. For each player action, traverse boost candidates for each input, then execute
   the weapon-specific scalar check/voice/damage plan. Enemy AI runs its source
   HP/charge/previous-action branches, selects a target, and resolves damage.
5. Each damage/miss notification schedules a timer that consumes main CRT even
   with zero random variance. Preserve this draw without creating a notification.
   The source damage pool has 50 slots, more than this entire routine needs.
6. On a new round, draw one timing revision per living actor, retain the previous
   preview as current order, and generate the following preview. Break excludes
   actors from acting but not all order bookkeeping.
7. On victory, roll one 30% grape-drop check for each pursuer. This group's
   UseVictoryAction is false, so there is no victory voice selection.
8. Reset `Dng_Cty_3_1` encounter steps: draw B then A with RandomInteger(400), then
   truncate `(1000 + A + B) * 0.5`. Scripted event battle entry does not clear
   FirstEncount; the ordinary encounter lottery does.

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

For seed 123, the source projection predicts six party actions, victory in round
2, zero damage taken, zero drops, and CRT state 3423024866 after 145 draws. The
field counter is 798. Trace-on/off and cloned/restarted runs agree; tests cover
all six party orders and both Defend/no-Defend paths.

**An actual full opening trace is still required to establish exactness.** Compare
the entry seed and every action's actor, target, damage, queue and CRT state,
then both reward and field-reset checkpoints. Internal replay and seed sweeps
show consistency, not equivalence to the game. Other commands remain available
in `Battle::new`, but the full-callback constructor rejects unsupported skills,
items, Wait and weapon changes. Longer fights and altered loadouts have not been
validated by this routine audit.
