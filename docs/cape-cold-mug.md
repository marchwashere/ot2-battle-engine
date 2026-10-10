# Cape Cold valuables Mug

Source-derived, not live-trace validated. `mug::predict` runs the user's fixed
routine as level-one Osvald with his original staff/clothes: Icewind, Fireball
at 2 BP, then unboosted staff only if necessary. The independent core starts
before actor initialization and ends after victory/drop callbacks. Field
time change, menu/dialogue and return callbacks belong to the assistant.

`ENG_NPC_Twn_Snw_1_2_A_1100` contains the man with Heavy Coin Pouch, Gold Pocket
Watch and Old Locket. He has 280 HP, one shield, 85 elemental defense; sword,
dagger, staff and ice weaknesses. Osvald has 200 HP, 60 SP, elemental attack
104+4 staff, growth80. Character revision already includes his base job.
Throné is not a battle participant, so her night talent does not apply.

Icewind breaks on R1, doing52–54. Fireball's 2 BP coefficient is280%, doing
225–234 against the broken enemy. Surviving requires a total below280 and
leaves1–3 HP. Recovery gives the man priority in R3: NPC Pattern's first actual
action is Stand Ground, applying two separate two-turn defense buffs (21 CRT
draws each). He does not use NormalEnemy's charged-action reroll. Staff then
deals11, with100% accuracy and0% crit, and wins. Longer NPC combat is rejected.
DEF_UP/MDEF_UP use the imported DISEASE_PARANETER_RATIO minimum0.667 in the
final defense multiplier, not an inflated defense stat.

Preserve Study Foe's one random unopened-weakness selection, Osvald's19
order-voice candidate checks, individual boost increases, source skill voice
durations/camera flags, break/damage timers and the30% Herb of Clarity drop
check. Drops outside the usable item catalog are returned as reward labels.
The Mug-specific importer is `tools/import-cape-cold-mug.cjs`; field callback
audit is the assistant's `tools/audit-cape-cold-mug.cjs`.

The two damage variance draws are29 CRT calls apart. The odds example counts
all2^31 distinguishable CRT states with affine LCG interval arithmetic:
**367149429 / 2147483648 =17.096727574%** survive R2, assuming a uniform initial
CRT state and this unchanged routine. Prior route searches can bias this
distribution. Once a particular state is supplied, the result is deterministic.

User-confirmed input: seed0 +6154, daytime Cape Cold, original-gear Lv1 Osvald.
Assistant source projection: night6163; preview6164; Mug opening-line lottery
6165; player menus6192/6218/6289; damage53/226/11; victory/drops6297; field return6299
(0xDC86178D). Retail replay validates only initialization through first menu6192.
The Mug NPC draws BP and timing, without the ordinary enemy sprite starting
frame. Study Foe and all19 order-voice checks are captured. The compact fixture
`data/source/cape-cold-mug-first-menu-live.json` retains callsites and source hash;
remaining combat and field return are model predictions.

EXP/JP coverage remains incomplete: existing assistant modules calculate
Throné's route EXP/level, but no general per-character EXP/JP ledger exists.
This profile does not silently fabricate a ledger or level-up state.

Initialization is now resource-derived, not a Mug exception: the initial
BAT_IDLE_ENEMY slot is absent, so RandomInteger(0) consumes no CRT draw.
See the core docs/enemy-initialization.md for the complete branch and scope.

## Retail completion correction

The completion replay confirms menus6192/6218/6289 and field control6299
(0xDC86178D). Prior6297 endpoint omitted kill voice at6293 and the break
slow timer at6294. Osvald normal attack voice lasts5689 ticks; damage occurs
at9600, so the kill reaction is eligible. JustBreak remains latched even when
the same hit kills and death clears the persistent break state. Drop is at6296,
battle exit6297; field reset draws6297/6298. Loot dismissal adds no extra reset.
The compact completion fixture retains all134 calls from battle initialization
to field return, plus the raw capture hash. This supersedes earlier source-only
completion values above.
