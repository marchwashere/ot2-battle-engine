# Bewildering Grace voice audit — 2026-10-17

Audited all 39 lottery children, both nested summons, the parent sequence,
LibAilment/player callbacks, and 170 reaction candidates for Throné, Osvald,
Agnea, Partitio, Hikari and Temenos. Reproduce source checks with
`node tools/audit-bg-voices.cjs`; evidence and all individual effect records
are in `data/source/bg-voice-audit.json`. This is a source audit with regression
coverage, not a live trace of every effect.

## Confirmed corrections

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
| 024 | Silence | First new Silence can react; later blocked reactions draw nothing |
| 025 | Caster SP depletion | UI, no recovery voice |
| 026 | Attack buff | No debuff voice |
| 027 | Caster HP recovery | UI; no self-recovery voice; still settles owner flag |
| 028 | Party wind damage, delay | Damage/death reactions; order shift silent |
| 029 | Direct shield damage | Break UI/camera; no break voice |
| 030 | Defense debuffs | Interleaved application/type 4; shared settled-damage gate |
| 031 | No effect | No voice |
| 032, 033 | Latent recovery | Owner gate; no scalar UI timer or extra voice on reaching cap |
| 034 | Latent depletion | No scalar UI timer or recovery voice |
| 035 | Reverse buffs | Debuff reactions subject to settled/silence gates |
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
