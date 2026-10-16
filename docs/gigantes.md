# Gigantes and the revised Sands routine

## Retail Nameless capture, 2026-10-16

The third attempt in `data/source/gigantes-live-20261016.json` is user-confirmed:
seed 1E10AA9C, island start 9216, correct wild Defends and Throné sword selection.
Core boss entry follows four player initialization draws at 9597. The opening,
T1 actions and Osvald T2 command menu at 9825 match. Elemental Barrage previously
allowed weakness voices during Osvald's cast clip. Its English SoundWave duration
is 42697 ticks at 24000 Hz, imported from BattleGAK/vo_gak_pc_ab_04050. Blocking
reactions while that clip plays matches Throné's menu at 9891. The full regression
now matches all 361 captured RNG draws through the post-drop boundary at 9958,
including callback categories for the sword and Hired Help suffix. Partitio's T2
menu is at 9926; his mandatory boosts finish at 9932.

The field counter reset then reaches9960. The automatic Georges rescue scene
finishes at9963 (state0x98D520A9), before the player's manual switch to day.
Do not present the intermediate field-return checkpoint as the end of that scene.


`gigantes::predict` consumes carried Throne/Osvald/Temenos/Partitio stats,
resources and weakness history. It requires both merchants' Boost-Start and uses
four A Step Ahead members. The caller validates supports, learned skills and gear.

Gigantes, night:

- T1: Throne and Osvald Defend. Partitio Defends only when preceding both;
  otherwise he attacks with his lance. Temenos attacks with his staff.
- T2: Throne +2 sword, Osvald +3 Elemental Barrage, Partitio +3 Beastlings.
  Temenos Defends if his action is reached. Reject an enemy turn, unbroken
  Beastlings target, insufficient BP/SP, or incomplete victory.

Later Defends precede earlier Defends. Both breakers precede Partitio on T2.
Three sword hits plus the minimum six Barrage hits break nine shields before
Beastlings. Night talents last through T2.

With the assistant's level70 entry fixture and manual loadout, the conservative
floor is sword1356 + Barrage17454 + Beastlings32696 = **51506** versus50000 HP.
This excludes both T1 attacks, every critical, every fire-potency bonus and all
extra Barrage hits; sword/Barrage damage is counted entirely unbroken. The
assistant evaluates the floor from the actual carried stats before accepting.

`predict_sands` implements the approved two-turn Sands routine: T1 Scope/Holy
Light/Osvald Defend, conditional Partitio Defend; T2 +3 Slash and +3 Beastlings.
Finisher boost cycles preserve the preceding prefix.

Import order: `import-bg-effects.cjs`, `import-lord-sands.cjs`, then
`import-gigantes.cjs`. The latter imports raw ability rows and sequence markers.
Random count initialization shuffles the native minimum-hit array. Runtime
branches select Barrage elements between hit callbacks; skipped branches do not
draw RNG. Break timers remain on elapsed timeline time across jumps. An initial
counter increment and the terminal Finish section are retained: six to eight
actual damaging hits. Ability voices use base IDs.

Equipment fire potency applies at the native outgoing-damage stage, before
break, ability ratio, buffs and growth. Raw nut doping is added before character
and secondary-job revisions, preserving float32 rounding through level changes.
Nuts do not increase the level-based damage-growth multiplier.

Validation:256 Gigantes seeds cover all three Barrage counts and both Partitio
branches;128 Sands seeds cover six-hit Slash and finisher-prefix preservation.
Both bosses also have captured retail RNG regressions.

## 2026-10-16 retail Lord of the Sands correction

Both bosses initialize two authored action slots (NORMAL and DELAY), not one.
AIInitialize shuffles action tables through BP_BattleAIUtility.CreateActionTable:
Lord of the Sands consumes 11 shuffle draws; Gigantes consumes 15. Native
Array_Shuffle includes a draw for the final singleton. ExecSettingNextOrder then
inserts two rows, consuming two more draws. The second slot uses delay ratio 1.3
for Sands and 0.6 for Gigantes. NORMAL ignores its stored delay ratio. Order sorting
retains each slot identity through the unstable native sort and triple shuffles.
These profiles still reject enemy actions; this does not implement arbitrary boss AI.

RandomCountAttackCommand.ExecStepCounter calls CheckAttackHit before damage.
Aggressive Slash therefore caches a hit roll even when Critical Scope guarantees
its critical. MinimumCount is zero. Holy Light's Temenos cast clip lasts
2.428999900817871 seconds, suppressing the immediate weakness reaction while it
plays; its duration is imported from the English voice asset.

Evidence: data/source/route-boss-ai.json contains the decompiled AI/order and
RandomCountAttack methods with asset hashes. data/source/lord-sands-live-20261016.json
contains the captured RNG calls and carried party fixture. Its regression matches
the observed T1 queue, initial T2 preview, all eight action boundaries and the
post-victory/drop RNG state. BreakActionOrder removes all remaining boss rows and
inserts one hidden BREAK row, matching the existing break implementation.

Validation after the complete Gigantes correction: all core unit and integration
tests pass, including the earlier Guards/Chubby, Temenos and Sands captures;
assistant Quicksand (3) and Nameless Isle (8) tests pass.

## Boss-profile safety and source inventory

### Completed retail sword and summon timing audit (2026-10-16)

User confirmed the final captured attempt breaks Gigantes on Throné's first
sword hit after eight Elemental Barrage hits, at 2x battle speed. The regression
asserts eight Barrage damage events with no break and the matched menu at 9891.
The sword starts after boosts at 9894. Its third-hit timer is draw9908, Partitio's
menu is 9926, his boosts end at 9932, and victory/drop ends at 9958. All now match
the final capture without inserting RNG offsets.

Static audit: TickManualSequencerUpdate (140e481a0) advances each registered
sequence by its playback rate times DeltaSeconds. Break-camera sequences omit
TickResolution and inherit the native MovieScene default of 60000/1, unlike the
normal attack's explicit 24000/1. Read-only inspection of Default__MovieScene and
four loaded break cameras confirmed that default; native getter1408e9330 reads
it at MovieScene+0x108. The camera's upper bound30000 is therefore 0.5 seconds,
or12000 action ticks. The old import treated it as1.25 seconds and suppressed
the third sword hit's timer. import-opening-crt now converts the clocks explicitly.

Normal sword accuracy uses AfterSpawn, while its same-tick voice event uses the
native AtEndOfEvaluation default. Sequence import now preserves that ordering,
and execution chooses the actual command weapon rather than the fixture weapon.

Hired Help children share the BattleCamera.Playing guard. At the exact camera
endpoint, the action runs before the appended camera's OnStop; use a strict
comparison for broken-hit timers. This preserves the earlier retail Guards
capture. CheckLastHit uses the parent's m_bIsSummonLastHit, set only on the final
summon. SetDamageForEnemy consequently retains Break after an early lethal hit
until the final summon, accounting for Beastlings' timer at9952. The fixture
checks every draw's state and relevant native callback, not only the endpoint.

Run node tools/audit-boss-ai.cjs to regenerate the 96 boss/SubBoss AI asset inventory (127 enemy identities). It records full AIInitialize bodies, source hashes and RNG/order-related method call sites. 94 assets contain RNG-related methods and 54 contain explicit order-changing methods; these are static call-site counts, not fixed draw totals or action counts. Phase branches, inherited calls, table shuffles, random action selection, additional actions and break recovery must be traced for each boss before implementing it.

new_route_boss accepts only the audited Sands/Gigantes profiles. Ordinary field constructors reject registered boss-AI enemies, including repeat-boss encounters, instead of silently giving them one slot/no AI initialization. Pirro retains its separate audited implementation. This does not implement combat for the other bosses.
