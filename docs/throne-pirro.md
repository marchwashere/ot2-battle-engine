# Throné Chapter 1: Pirro

`BattleConfig::throne_pirro`, `Battle::new_pirro` and `pirro::PirroRoutine`
run independently of the assistant. Entry is before actor initialization;
exit is after victory/drop callbacks, before the next story event or field
counter reset. Main CRT only. No rendering/audio runtime.

The configuration imports ENE/ENG_BOS_THI_C01_010: 2100 HP, five shields,
sword/light/dark weaknesses. Solo level-two Throné uses her original gear,
three turns of night talent, initial 1 BP. The standalone example supplies
one grape and one of each required small soulstone. The assistant replaces
these with the route's carried resources and inventory.

## Fixed routine

R1/R2 Darkest Night, R3 3 BP Darkest Night, R4/R5 sword, R6 Shadow Soulstone.
At the R7 player menu, inspect the R8 preview. If Pirro precedes Throné, use a
grape now and delay the remaining actions one round. Otherwise begin them now:
Light Soulstone; latent + 3 BP sword then Wind Soulstone in the consecutive
extra action; Ice Soulstone; sword. Unspecified boosts are zero.

In the no-grape branch, test latent readiness at the R8 command menu. If it
is not full, use 3 BP swords without latent on R8; on R9 activate latent for
Wind Soulstone followed immediately by Ice Soulstone; R10 remains sword.
Activation still requires a full gauge on R9; no artificial charge is added.
The routine exposes `latent_delayed` and the pending second item in its state.

`PirroRoutine::step_with_boost_inputs` accepts a complete button path in place
of automatic boosting; it must end at the action's BP. `Battle::preview_boost_inputs`
returns an uncommitted state for a partial path, including real voice draws.
The Pirro-specific max-boost voice has a 50% rate and precedes the generic one:
raising 2->3 consumes either three or four CRT calls. Both are non-PlayOnce.
The assistant uses this API to search R3 inputs for a winning no-grape branch;
the core remains independent and never forces an outcome or turn order.

The policy does not force victory. An R4 critical can cross Pirro's 75% phase
threshold early and bring forward his stronger attacks. A regression at CRT
entry state `0xAA7C3306` (seed 0 + 5678) loses in R7 before the planned heal. A 1024-entry-state sweep
covers both healing branches, misses, criticals, victories and defeats; it is
not an estimate of route-conditioned odds or live-game validation.

## Port and provenance

`tools/audit-pirro.cjs` decodes BattleAI_Bos_Thi_C01_010 and the rich event.
`tools/import-pirro.cjs` imports all fourteen boss skill records, scalar skill
key schedules, battle voice candidates for both phases and item sequences.
Run the normal second-battle importer first, then this importer. The ordinary
item importer also preserves the four small soulstones.

AI initialization shuffles five tables, including an unused one: 12 CRT calls.
Fixed-first shuffles exclude index zero but still draw for the final one-entry
range. Phase changes use the original HP comparisons and working queues.
The group's `TalkPlayerID=eTHIEF` fixes the battle-start speaker to Throné.
`PlayPlayerSideVoice_BattleStart` skips `RandomInteger` when that designated
player is alive; only the missing/dead/unspecified-speaker fallback draws.
Voice selection itself still draws once. The importer preserves the resolved
actor and decoded branch in `data/source/pirro-startup.json`.
The exact 75% equality takes the exhausted-table branch. The scripted hold-out
keeps Pirro at one HP without clamping displayed damage; the mercy action
removes it. Status additions preserve the check and all 20 slot draws.

Its `ABI_BOS_THI_C01_010_008` action is **Pierce Through / 穿心**: 1% of
maximum HP, integer division, so 261 max HP gives **2 damage**. It applies
`ATTACK_HIT_FAILED_NO_ICON`, but `JudgeSpecialHitRate` checks that status only
for CalcType 0. The percent-HP effect is CalcType 2 and still checks accuracy.
The earlier unconditional forced miss was wrong. Preserve both scripted status
applications and the successful damage-reaction voice checks. Source and
localization assertions: `node tools/audit-pirro-mercy.cjs`, recorded in
`data/source/pirro-mercy.json`.

The status is an explicit self-targeted effect of this ability, executed at
sequence time zero and reapplied by the final all-effects key. The ability
table has no other `ADD_THIS_ATTACK_HIT_FAILED` user. Pirro selects this action
once when Throné is below 34% HP in AI states 6/8 (state10), or as an overriding
condition when Pirro is at exactly 1 HP after his phase change and has not yet
used it (state20). Both share `UsedAbility_State10`. The latter causes the
entry5595 route's R10 use. This describes the status's application, separately
from the CalcType gate controlling its effect on hit resolution.

The user confirmed the entry5595 route's R9 menus at **5832 / 5836** and R10
at **5892**, with Pierce Through dealing a small hit. The corrected model
matches those endpoints: the restored accuracy draw adds one and three
damage-voice candidate checks replace one avoidance check, adding two.
After drops it reaches **5900**. The 2026-09-29 replay now verifies all 305
calls from entry5595 through this winning endpoint; see below.

The battle event below 75% HP unlocks Throné's latent power at **55/80**.
The sequence director calls the player's ReleasePotentialityAction directly,
which sets ActionPointInit from PotentialityActionDB. The manager's similarly
named full-gauge helper is a different path. See audit-pirro-latent.cjs and
data/source/pirro-latent-unlock.json. Activation consumes the voice-selection CRT rolls, spends the gauge and
inserts a second consecutive player action. It does not refill BP. Invalid
activation/commands roll back atomically. Damage and breaking add source-derived
latent points after unlocking. Grape self-recovery suppresses recovery voice;
its scalar notification timer still draws.

Small soulstones use fixed 200 elemental damage: 260 against an unbroken
weakness, 400 against a broken target. They have no accuracy/critical/variance
rolls; item voices, hit notifications and break/kill reactions remain in CRT.

## Limits and verification

On 2026-09-28 the user reported seed 0, pre-cutscene advance **5595** and first
command-menu advance **5631**. Startup now matches those 36 calls: four actor
initialization, twelve AI shuffles, two order shuffles, one battle-start voice,
and seventeen command-menu voice checks. The old 37-call model incorrectly
randomized the explicitly selected speaker. This is an observed endpoint
regression. The subsequent live capture also confirms the 36-call startup.

The 2026-09-28 capture at seed 0 + **5678** contains 203 calls, ending at **5881**
(`0x1FB4FF1B`) on defeat. See `data/source/live-pirro-5678.json`, which includes
the original capture SHA-256 and all recorded calls. The regression matches
**all 203 calls** by CRT state and native caller, plus all six command menus:
**5714, 5752, 5783, 5813, 5860, 5870**. The model reproduces the user-reported
R4 sword critical for **38** and predicts defeat in R7.

This exposed two omitted conditional calls and one extra timer, now fixed:
- First Darkest Night reveals a new dark weakness. `CheckResistance` detects
  that before `SetWeakVisible`; `Battle_ExecDamageHP` selects type-9 voice after
  kill/break priority and the existing voice-busy gate. Later dark hits do not
  rediscover it. Successful normal attacks also record discoveries even when
  their reaction voice is suppressed.
- Pirro's one-time `ChangeActionMode` invokes `ApplyChangeActionMode` after the
  current action/round callbacks. Enemy reinitialization draws a new idle frame
  after the next order voice and before command input. Preserve this CRT draw
  without rendering anything; the winning capture places voice at5796 and the
  phase frame at5797. The loss has no intervening next-enemy voice draw.

- `GetActionCommandForItem` explicitly sets `UseAbilityCamera=true`, overriding
  the generic `USE_ITEM` row. `Setup Action Command` copies it, and the pre-action
  callback sets the scalar camera `Playing` flag until post-action completion.
  `PlayFinishBlow` exits while that flag is set, before its timer's CRT draw.
  The simulator now imports this gate for items and the corresponding data flag
  for skills. A newly inflicted break retains its separate timer, and ordinary
  attacks retain their broken-hit draws. No graphics are executed. Reproduce
  the source chain with `node tools/audit-hit-timer.cjs`; excerpts and asset
  hashes are in `data/source/hit-timer-gates.json`.

After R6 Shadow Soulstone, both retail and the model now begin round timing at
**5872** and finish this defeat at **5881 / 0x1FB4FF1B**. The earlier 5882 endpoint
was wrong. The exploratory decoder had silently omitted the camera ubergraph
and item-command builder; the audit now uses explicit schema script offsets
and rejects missing requested functions. Other winning paths and boost-search
results remain source-derived outside the capture below. English voices and 2x timing
are the supported scalar profile.

The later user report at entry **5595** places the R7 menu at **5798**, not
5799. The extra draw was an incorrectly shared normal-enemy BP threshold
reroll. `BattleAI_NormalEnemy.UpdateThink` calls `BoostActionCheck`, which
resets full BP through `SetBoostPoint_OLD(0, true)` and rerolls `BoostMax`.
Pirro does not inherit that AI or call that helper; his charged actions are
scripted separately. Removed that reset from the boss path. Initialization
and the phase-change frame draw remain. `node tools/audit-pirro-boost.cjs`
rechecks the hierarchy and call sites; the endpoint regression also asserts
BP stays full. This endpoint report is not a per-call capture of the win.

The 2026-09-29 replay at entry5595 now covers that win call by call:
`data/source/live-pirro-5595-victory.json` records all **305** calls, ending at
**5900 / 0x533B5D0C**. The missing final call was `EnemyData.DeadEffectSlow`:
`PlayEnemyDeadCamera` schedules `FinishEnemyDeadSlow` with K2_SetTimerDelegate
even with zero timer variance, consuming CRT at5897 after damage UI and before
victory voice/drop. `audit-enemy-death-slow.cjs` preserves the source gate and
timer continuation. It applies on the boss's actual death, never on hold-out
at 1 HP or a player defeat. The separate assistant ending matches62 calls to5962.

This profile supports the supplied routine and its ordinary/mercy phase paths.
If a different or prolonged policy enters the charged multi-order phase, it
returns an explicit unsupported error before committing that enemy action.
The source requests FIRST/LAST orders through PostSignOfBoost; appending a
normal enemy turn would be wrong, so no approximate continuation is exposed.
The full records remain imported for that future extension. Post-victory
1900/2000 scenes are outside this battle boundary.

Run `cargo test --all-targets`, or inspect a standalone replay with:

```powershell
cargo run --example pirro -- 0 5595
```

The complete captured loss matches the live evidence above. Earlier defeat
examples/odds must be recomputed
after these corrections and must not be presented as verified retail odds.
