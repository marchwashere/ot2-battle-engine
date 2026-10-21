# Shared damage-camera RNG gate

`battle/hit_camera.rs` projects the camera state that gates main-CRT timer
creation. It does not render graphics or simulate audio. Ordinary attacks,
scripted skills/items, and the shared BG/route-command executor use it.

## Source conditions

- `LibAilment.Battle_ExecDamageHP` handles `JustBreak` with `PlayBreakCamera`.
  Otherwise a successfully damaged target that remains broken requests
  `PlayFinishBlow`. Final-hit death cleanup can clear the broken state first.
- `BattleManagerBP.PlayBreakCamera` starts a break camera only when the camera
  is not already Playing. Its pushed continuation still schedules the separate
  break timer when an ability camera prevents starting a break camera.
- `PlayFinishBlow` returns during enemy-death slow, then requests chromatic
  aberration and checks `BattleCamera.IsPlaying`. If Playing, it returns before
  the timer. Thus suppressing the timer does not imply suppressing every visual.
- `BP_ActionCommandBase.Setup Action Command` reads `UseAbilityCamera`.
  Supported ordinary skill commands start that camera in pre-action and finish
  it in post-action. Their entire damage timeline runs under that camera.
- At a break camera's exact endpoint, action events precede the appended
  camera sequence's OnStop. Keep the gate inclusive at that endpoint; the
  captured Guards trace validates this ordering.
- The `FINISH_BLOW_SLOW` data is Min=0, Max=1, Init=1. Native
  `FUN_140ef9780` uses Max as a multiplier of the base battle speed: it adds no
  slowdown. Its cleanup timer still consumes CRT in `FUN_142c52d90`, where
  `rand()` is evaluated even when the initial-delay variance is zero.

Run `node tools/audit-hit-camera.cjs` to compare every imported command's flag
with cooked AbilityData and check the Blueprint branches. The hashed evidence
is in `data/source/hit-camera.json`. The flag is required on deserialization;
missing data must not silently become a camera-free action.

## Scope and correction

The shared executor previously special-cased Fire Storm and max-boost Barrage
by ID. It now retains the imported flag for all 56 command definitions. This
also covers Aggressive Slash and single-hit Holy Light against a broken target.
It preserves the separate new-break timer, including while an ability camera
is active, and does not start a fictitious nested break camera.

Hired Help parents and their Guards/Beastlings children have the flag false.
Their overlapping children share the break-camera state and retain legitimate
finish-blow timers. BG is not a reason to inherit a parent flag: its specialized
RandomSelectAbilityCommand overrides PreActionCommand, and its imported child
commands and nested summons have camera-free timelines.

The projection covers the currently supported sequential player actions and
overlapping camera-free summons. Overlapping camera-enabled jobs are rejected
until their ownership/completion callbacks are audited. Debug camera overrides,
eight-character enemy commands, and arbitrary concurrent death cameras are not
newly supported by this change. Current enemy-death callbacks occur separately
at finalization; retain their existing trace tests. No skill implementation such
as Sixfold Strike is added merely by making the camera gate shared.

## Regression coverage

- New-break versus already-broken hits; shared break cameras do not restart;
  exact completion-tick ordering; no nested camera beneath an ability camera.
- Both supported Aggressive Slash boost tiers breaking on hit one, retaining
  the break timer and suppressing subsequent finish-blow timers.
- Single-hit Holy Light against a living, already-broken target.
- All 56 imported flags compared to cooked data; missing flag rejected.
- Existing full Guards, Gigantes, Sands, opening/Pirro and field traces, plus
  Fire Storm, BG, EXP100 and JP100 routine regressions. Preserve the two Gigantes
  VOD fixtures and the independent 361-draw captured battle without updating
  their expected results.

## Validation run, 2026-10-21

- Source audit: all 56 imported flags match cooked AbilityData.
- Core: 99 library tests and 123 integration tests across all 22 integration
  suites pass. The integration executables were freshly built with the changed
  production code; the two new skill tests were then rerun after correcting
  their test-party fixture. No existing expected RNG values were changed.
- Assistant: 202 pass, 7 fail, 12 already ignored, 2 explicitly excluded.
  A separate clean archive of core `8f1e6786504ab333d83b7742a6c8a4675d33def3`
  and assistant `f92011ae26d2a6121205d8f2449d529d3265302f` produces the identical
  pass/fail/ignored result for every listed test. These seven failures predate
  this change; the full assistant suite is not claimed green.
- Windows assistant tests need `RUST_MIN_STACK=33554432`; the default stack
  overflowed in an existing Conning Creek test. This is a test-run setting,
  not a change to runtime search limits.

The pre-existing failures are the Flamechurch, Temenos and Wellgrove fixtures
that try to learn Collect after Osvald has already changed jobs; the Clockbank
outlier cost benchmark; the Gravell continuation's old 5550-draw expectation
(both builds return5554); the direct Inventor fixture missing Elemental
Augmentation; and the winter-voyage fixture missing the Coast Merchant.

Two long search regressions were stopped during the broader run and excluded
from both comparison runs:
`flats_escape::tests::cait_fork_replays_joint_targets_and_freezes_the_incoming_encounter`
and `snows_escape::tests::cached_cost_layers_preserve_complete_uncached_routes_and_timing_overrides`.
The completed comparison takes about22 seconds. Logs are under the core
`target/camera-*-tests.log`, assistant `target/camera-route-regression-tests.log`,
and core `target/camera-baseline/OT2SpeedrunAssistant/baseline-route-tests.log`.
