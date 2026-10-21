# Bewildering Grace voice audit — 2026-10-17

Audited all 39 lottery children, both nested summons, the parent sequence,
LibAilment/player callbacks, and 170 reaction candidates for Throné, Osvald,
Agnea, Partitio, Hikari and Temenos. Reproduce source checks with
`node tools/audit-bg-voices.cjs`; evidence and all individual effect records
are in `data/source/bg-voice-audit.json`. This is a source audit with regression
coverage, not a live trace of every effect.

## Confirmed corrections

### Follow-up: Silence and latent recovery, seed 0x7B4971B0 (2026-10-22)

The original inputs reproduce the reported sequence exactly: mist/Silence
(024), party latent depletion (034), latent +30 (032), then an additional
end-of-turn action (038). The old simulator matched the first three effects
but incorrectly predicted EXP100 as the fourth.

The error was a misidentified disease enum in the voice gate. Native enum
registration at `0x143d80e60` identifies Silence as **1**. The Blueprint
`BattlePlayerBP.CanPlayVoiceWhileDisease` checks **4, 101, 85, 53, 6**:
Sleep, Sleep Doom, Frozen, Stone and Stun. Silence is absent. Those conditions
block voice types 4/11/13/26, but Silence does not. Its skill prohibition is
separate and remains enforced. `OnTakeDisease` also does not need an exception
for newly inflicted Silence; the earlier interpretation of that branch was
wrong for the same reason.

The third effect shuffles its recovery speaker and selects Partitio. His
recovery voice consumes **8 draws**, even while Silenced. The fourth lottery
therefore occurs at route position **3019**, not **3011**, producing 038 rather
than 002. Earlier lotteries remain at 2911, 3005 and 3006, and every menu and
boost input through the BG cast remains unchanged. This is a missing voice
traversal, not extra RNG consumed by latent depletion or a camera correction.

The source audit now reads and verifies the native enum registration, extracts
the exact disease IDs from the Blueprint, and asserts Silence is not included.
`data/source/bg-chubby-7b4971b0.json` preserves the reconstructed original
party/inputs and user-reported effects, not a live RNG trace. Its regression
checks all four effects, the eight-draw Partitio voice, unchanged earlier menus,
and rejection for missing EXP100. Reaction tests compare Silenced and healthy
characters for damage, avoidance, recovery and later debuffs; menu decondition
voices and skill blocking remain covered separately.

### Follow-up: Reverse Buffs, Chubby Cait seed 0x2135708B (2026-10-21)

The original boost inputs reproduce the user's four reported effects exactly:
buffs turn into debuffs (035), mist/Silence (024), party fire damage (004),
then 1,000 leaves (036). The old prediction was 035, enemy-buffing summon
(017), EXP100 (002), no effect (031).

The missing calls are inside `LibAilment.SortDiseaseListForReverse`, invoked
by `Battle_ExecReverseDisease` before applying each target's reversed buffs.
It filters permanent/non-resistible/non-turn-counted diseases, then uses an
insertion sort by descending duration. A longer duration inserts without RNG;
otherwise it calls `RandomBool` before testing whether durations are equal.
Consequently even an unequal shorter duration consumes a comparison roll.
Native `FUN_1430d6c80` draws one CRT value and implements
`RandomInteger(2) == 1`, not the weighted-boolean threshold helper.

For this party the four buff lists consume **10 additional draws** before
the second lottery. The first lottery remains at route position 2998; the
second is 3018 rather than 3008. No Guards, camera, or pre-BG menu changes
are needed. The same-input full pre-fix fight was identical at `0804390`,
`8f1e678`, and the working tree with the shared camera refactor. The omitted
sorting therefore predates both camera changes. This does not resolve the
separate inconsistent fourth-Guards-hit report for seed `0x0FF630A8`.

`data/source/bg-chubby-2135708b.json` records the original reconstructed
party/inputs and user-reported effects; it is not a live RNG trace. Regression
tests preserve every menu through BG, reproduce all four observed effects,
and reject the old candidate for missing EXP100. Primitive tests cover equal
and unequal durations, insertion order, and permanent-buff exclusion. The
source audit now includes the previously omitted sorting helper.

### Follow-up: Chubby Cait seed 0x3899B25A (2026-10-20)

The user's original inputs now reproduce all four reported effects:
EXP ×100, lightning hitting the surviving Dreadwing once, items scattered,
then full party HP/SP/BP recovery. Two source-backed errors caused the old
prediction to accept this unusable item-finisher route:

- `RandomSelectAbilityCommand.GetAbilityTarget` filters dead/stealthed actors
  before selecting a random target. In the enemy branch, PushExecutionFlow
  targets 1502 then 1746 execute in reverse: filter 1746, then random selection
  1502. The old model selected a dead enemy and skipped the lightning's damage
  and notification draws. Party-single uses the same stack ordering.
- Agnea's damage reactions include Dulcinea dance/song conditions. VoiceRate
  rolls precede those conditions in `BP_BattleVoiceComponent.PlayBattleVoice`.
  `BattlePlayerBP.PlayBattleVoiceForDamage` gets the conditions from the
  damaging command's `IsMusicalCommand`; the scatter child has neither flag.
  The imported voice profiles omitted those flags and incorrectly selected
  the dance reaction, skipping two candidate rolls. The corrected traversal
  rolls counter, debuff, dance, song, then small damage.

`data/source/bg-chubby-3899b25a.json` preserves the original route party and
boost inputs. These are locally reproduced inputs, not a live RNG-call trace.
The regression checks the reported effects, surviving Dreadwing, retained
EXP ×100, and rejection because item lock prevents the soulstone finisher.
The source-data audit now validates song/dance flags in every voice profile.

### Earlier corrections

1. **Recovery suppression belongs to the whole dance.** RandomSelectAbilityCommand
   creates children with `SetSummonAbility(true, true, ..., self)`. Both
   `BP_ActionCommandBase.GetSettledPlayOnce_OnRecovered` and its setter delegate
   to `m_cSummonOwnerAbility`. LibAilment shuffles recovery recipients once and
   settles that owner flag. Later recovery operations/children disable recovery
   voices without another shuffle. Self-healing also settles the flag. Each new
   dance gets a fresh flag. Revival voices remain separate.
2. **Three-hit light retains prepared targets.** `Battle_ExecDamageHP` tests
   object validity, not remaining HP. Later hits process earlier-killed targets;
   the final hit can request the aggregate kill voice. The old alive check
   skipped these callbacks and that voice.
3. **Direct shield damage has no break voice.** `Battle_ExecDamageSP` retains
   break UI/camera callbacks but does not request type 10. Neither its command
   nor player OnHitDamageSP callbacks request a voice. Parent event descriptors
   are BlueprintImplementableEvent (`0x0c020800`, no native implementation).
   Removed the fabricated break voice.

## Reported Shipwreck failure

Seed `0x1E10AA9C`, original Shipwreck entry 10012, Temenos menu 10221.
Old instructions reach pre-cast **10241 / `0x4FBF56EF`**. Correct execution:

| Lottery position | Effect |
| --- | --- |
| 10241 | Party BP fully restored |
| 10260 | Party latent +30 |
| 10261 | Party latent fully restored |
| 10262 | Three light hits against all enemies |

This matches the user's four reported effects. Previously the second recovery
child added a four-draw shuffle and eight voice-candidate draws, shifting the
third lottery incorrectly to 10273 and producing the false JP×100 prediction.
`bg-shipwreck-1e10aa9c.json` preserves the route inputs and reported effects;
its RNG checkpoints are predictions, not newly captured live calls.

The corrected bounded search finds an all-alive JP×100 victory from the same
entry, with pre-cast **10261 / `0x95A612B3`**. Do not mix those boost instructions
with a newly searched upstream route.

## Effect coverage

IDs are suffixes of `ABI_DAN_MAKA_`. Parent cast voice follows the first lottery.
Children do not replay it. The only authored child voice is 007/type 0 (NONE),
which has no matching voice row. Actual reaction calls come from the effects.

| IDs | Effect | Voice/callback rules checked |
| --- | --- | --- |
| 001, 009, 018 | JP multipliers | No effect voice or recipient shuffle |
| 002, 010, 019 | EXP multipliers | No effect voice or recipient shuffle |
| 003 | Transformation | Replacement initialization; no added caster voice |
| 004, 023 | Party fire damage | Damage/death voice and UI; settled-damage gate |
| 005 | Revival, HP/SP/BP, cleanse | Shared recovery setup; separate type 14 revival; display before type 13 recovery; cleanse silent |
| 006 | Damaging summon | Summon's voice profile and damage enforcer |
| 007 | Party HP to one | Silent type 0; actual damage reaction/UI, no critical/variance |
| 008 | Instant death | Death voice only on actual death; caster excluded |
| 011 | Three light hits | Prepared targets; aggregate weakness/break/final kill |
| 012, 021 | Party BP recovery | Owner gate; display before type 13 |
| 013 | Magic reflection | No debuff voice |
| 014 | Poison | Debuff type 4; later poison tick has UI, no damage voice |
| 015 | Items scattered, physical damage | No item-disable voice; normal hit/miss/damage/death reactions |
| 016 | BP depletion | UI callback, no recovery voice |
| 017 | Enemy-buffing summon | Summon profile; positive statuses do not trigger debuff voices |
| 020 | Party SP recovery | Owner gate; display before type 13 |
| 022 | Lightning | Outcome-dependent weakness/break/kill |
| 024 | Silence | Status application reacts normally; Silence blocks skills but does not suppress reaction/recovery voices |
| 025 | Caster SP depletion | UI, no recovery voice |
| 026 | Attack buff | No debuff voice |
| 027 | Caster HP recovery | UI; no self-recovery voice; still settles owner flag |
| 028 | Party wind damage, delay | Damage/death reactions; order shift silent |
| 029 | Direct shield damage | Break UI/camera; no break voice |
| 030 | Defense debuffs | Interleaved application/type 4; shared settled-damage gate |
| 031 | No effect | No voice |
| 032, 033 | Latent recovery | Owner gate; no scalar UI timer or extra voice on reaching cap |
| 034 | Latent depletion | No scalar UI timer or recovery voice |
| 035 | Reverse buffs | Randomized duration insertion sort per target, then debuff reactions subject to the settled-damage gate |
| 036, 037 | Money | No voice |
| 038 | Additional action | No immediate effect voice |
| 039 | Repeated random fire hits | Target/outcome reactions; party damage/death suppression |

## Timing and tests

Parent voice is authored at tick 4287, child dispatch at 100380: approximately
2.00 seconds apart at 2×, before the child's own timeline. Imported English BG
clips last 1.621 seconds (Agnea) and 1.439 seconds (Temenos), so they finish before
child damage reactions. No blanket battle-speed RNG adjustment was introduced.

Regressions cover the reported sequence, corrected JP search, all 49 ordered
pairs of recovery effects and reset between dances, early light kills, direct
shield breaks, all 39 individual effects, revival/death, Silence and debuffs.
Source checks verify candidate completeness across all six modeled characters.
Existing live Guards, Chubby Cait and Gigantes fixtures remain valid. Distinguish
this source-derived coverage from live-confirmed checkpoints.
