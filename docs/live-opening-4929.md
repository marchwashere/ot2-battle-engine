# Live opening crosscheck: seed 0, 4929 advances

This is the historical routine: the second party actor boosted on enemy 2;
the first and third attacked enemy 1 unboosted. The current predictor instead
boosts the first actor on enemy 1. Tests preserve this capture by replaying its
explicit actions, so the historical final state below is not the expected final
state for the revised routine.

User report on 2026-09-25; preserved in
`data/source/live-opening-4929.json`. The originally reported 4927 advances was
corrected to 4929. Tracer attachment independently read `0x0EA79703`, exactly
seed zero advanced 4929 times. Do not add an initialization offset.

| Checkpoint | Game report | Simulator after source corrections |
| --- | --- | --- |
| Pirro's first command menu | `0xF485D421` | `0xF485D421` |
| First-round party | Pirro, Throné, Scaracci | Pirro, Throné, Scaracci |
| Second-round party | Scaracci, Throné, Pirro | Scaracci, Throné, Pirro |
| After first field reset | `0x0EC00F7B` | `0x0EC00F7B` |
| After post-battle event reset | `0x194611CD` | `0x194611CD` |

The first menu consumes 30 calls in both game and simulation. The live replay
captured 154 consecutive CRT calls, all now matched by the simulator in state,
order and native caller category. Raw evidence is checked in at
`data/source/live-opening-4929-trace.json`. Game PID 31532, game thread 41824;
the tracer restored its debug registers and detached. No RNG writes or extra
reseeds occurred inside the capture. Debugger timings are diagnostic, not
measurements of uninstrumented game performance.

The previous prediction was `0x62B6954A` after 145 calls. Source-backed fixes
explain the difference: two delayed break timers, five finish-blow timers,
one round-start BP timer, one fewer shuffle call on the corrected RNG stream,
and two calls for the second post-battle reset: `2 + 5 + 1 - 1 + 2 = 9`.

| Boundary | Calls since entry |
| --- | ---: |
| Pirro's first command menu | 30 |
| Throné's first command menu, before boost | 52 |
| Scaracci's first command menu | 65 |
| Scaracci's second-round command menu | 80 |
| Throné's Defend menu | 110 |
| Pirro's second-round command menu, before boost | 135 |
| Last attack complete | 148 |
| Drops complete | 150 |
| First field reset | 152 |
| Post-battle event reset | 154 |

The user subsequently reported Pirro 31 and Throné 31/50, all using default
weapons. The opening configuration incorrectly enabled the three-turn night
talent, applying a 1.5 attack multiplier and moving party queue entries forward.
With `night_buff_turns: 0`, the prediction is now:

| Action | Reported | Predicted |
| --- | --- | --- |
| R1 Pirro | 31 | 31 |
| R1 Throné | 31 / 50 | **33 / 50** |
| R1 Scaracci | not supplied | 49 |
| R2 Scaracci | not supplied | 49 / 49 / 50 |
| R2 Throné | Defend | Defend |
| R2 Pirro | not supplied | 48 / 47 / 47 |

Throné's first hit remains an unresolved two-point discrepancy. Do not alter
the observed 31 or introduce an empirical multiplier to force it. Her selected
default dagger, its +6 ATK, base ATK 88, target DEF 80 and the captured variance
draw currently produce 33. Her second hit uses the same inputs with Break and
matches 50. The shared Blueprint formula and native float RNG were rechecked.

The full first-round display now matches Pirro, Throné, grunt, grunt, Scaracci.
Both enemies are broken before their scheduled turns. All 154 captured CRT calls
and every menu/final boundary still match after disabling the talent.

## Source corrections

- `BattleActionOrderBP.InitializeActionOrder` calls `SetupFirstTurnPlayer` after
  shuffling. It removes the first party row and inserts it at index zero. The
  previous simulator omitted this promotion. No RNG is involved.
- `CalcNextTurnWaitTime` reads cached `BaseWaitTime`. The first preview is built
  before the first `BeginBattleTurn`; adding the night talent does not refresh
  that cache. Using live buffed AGI for that preview was incorrect.
- `RI_MS_TOU_10_0200` passes fixed-first-player ID zero, so there is no source
  basis for forcing Pirro to act first.
- `CheckConditionSpecialAction` checks the talent's BanFlag as well as time of
  day. `SpecialAction.ePARTY_BUFF_IN_NIGHT.BanFlag` is 652, named
  `BAN_TOU_SPECIAL_ACTION` in the native enum. Source confirmation:
  `Content/Event/json/MS_TOU_10_0100` command 23 (line 25) sets flag 652 to 1
  before the opening fight; `MS_TOU_10_1600` command 11 (line 13) clears it
  later in Chapter 1. The importer validates both commands and stores them
  with their source hashes. The disabled opening talent is source-confirmed,
  independently of the matching damage/order.
- The native shuffle `FUN_140e47360` was checked against the executable decomp;
  its conditional calls and permutation match `shuffle_order`.

Boost voice selection was already modeled: each press calls
`PlayBattleVoice_Boost`, which passes the new boost level as `Border` to type 8.
Candidate probability checks precede eligibility. With this party's imported
100% candidates, pressing from level 0 to 1 costs one draw, and from 1 to 2
costs two. The replay confirms those counts and their positions in the stream.

## Timer and reset provenance

- `LibAilment.Battle_ExecDamageHP` calls `PlayBreakCamera` on JustBreak,
  otherwise `PlayFinishBlow` for a broken target.
- `BattleManagerBP` ubergraph @33312 delays by BREAK_CAMERA_SLOW.Min, then
  @7371 schedules the delegate. @33548 checks the busy flag before @7487
  schedules the finish-blow delegate. The native timer helper consumes CRT
  even with zero start variance (return RVA `0x2C52E57`).
- The normal break sequence lasts 30000 authored ticks at 24000 ticks/second;
  the timer delay is 2400 ticks. This places Throné's last-hit voice before the
  delayed break timer, while suppressing the finish-blow timer on her second
  hit. These are scalar conditions, with no presentation runtime.
- `BattleEnemyBP.SetDamageForEnemy` @2096 finalizes death on LastHit; the
  death branch clears Break before the final hit's finish-blow condition.
- `BattleManagerBP.ChangeBattleTurn` @2276 calls BeginAddBoostPoint after
  actor revisions. Ubergraph @34944 schedules its timer before next-order
  generation.
- `EventList.MS_TOU_10_0300.EncountReset=true`; `EventManagerBP.FinishPre` @3342
  tests that flag and @3470 resets the counter. The capture's second reset
  occurs 2.288 seconds after the first. The tracer identifies the counter
  function directly; attribution to event completion also uses this source
  flag. It does not directly label the on-screen popup.

Run all entry, party-order, per-action, per-call and final-state comparisons:

```sh
cargo test --offline --test live_opening
```

Live comparisons cover the full first queue, party orders, two matching damage
values and the complete captured CRT sequence. Throné's first hit remains
unresolved; these checks do not establish all damage or other seeds.
