# Throné: second Chapter 1 sewer fight

`ENG_EVE_THI_C01_020` runs independently in the Rust battle core. The default
configuration uses all three characters at level 2, their original equipment,
full HP/SP after leveling, and 1 starting BP. Throné's night talent is still
disabled by story progression. The right and left Pursuers have 185 HP and one
shield; the Pursuer Leader has 375 HP and three shields (axe/dark weaknesses).

## Routine

Commands follow actual turn order; the routine does not rearrange actors.

| Character | Round 1, 0 BP | Round 2, 2 BP |
|---|---|---|
| Throné | Darkest Night → right grunt | Darkest Night → leader |
| Pirro | Pierce Through → leader | Slice → left grunt |
| Scaracci | Axe → left grunt | Axe → right grunt |

Pirro's percentage skill is **Pierce Through**, English text `ABN_GST_0190`,
ability set `ABI_SET_GST_0190`. The earlier development name “Shred” was wrong.
Event command 8900 in `MS_TOU_10_0600` learns it through
`FUN_140f34b30(15,18,2,1)`.

Its `HP_RATIO_DAMAGE` effect uses maximum HP × 70/80/90/100% at 0/1/2/3 BP,
integer division, capped at current HP, followed by the common minimum of one.
It does not apply weakness/break damage multipliers or critical/variance rolls.
It still checks physical accuracy and dagger weakness for shields. Thus the
routine's unboosted hit on the leader deals 262, leaving 113 HP and three shields.
This is not modeled as healing the user; the row contains only HP_RATIO_DAMAGE.

Normal enemy AI supports charged attacks and the low-HP/no-repeat branch.
`BattleAI_NormalEnemy.AIInitialize` sets ordinary Attack to slot 1, charged
Hissatu to slot 0 and low-HP Okunote to slot 7. The leader's ordinary Attack
has one hit; charged Double Thrust has two authored damage callbacks. All hits, drops, BP,
SP, breaks and turn-order changes are simulated. A routine failure is exposed,
never converted into a forced victory.

## Interface and boundaries

```sh
cargo run --offline --release -- throne-second 0x12345678
cargo run --offline --release -- throne-second-sweep 0 4096
```

`BattleConfig::throne_second(seed)` and `Battle::new_throne_second` accept CRT
state immediately before enemy initialization. `ThroneSecondRoutine` supplies
the actions above. `BattleConfig::throne_second_at_level(seed, 1)` provides an
explicit level-one variant for openings lacking the overkill EXP bonus.

The trace finishes after victory voice and enemy drop lotteries. This profile
does **not** consume field-return/story-counter resets. `opening_crt.rewards`
records this boundary; the opening encounter's legacy `after_battle` field is
absent. The assistant owns the preceding cutscene, party progression and route
offsets, with no dependency from this crate back to the assistant.

## Evidence and remaining validation

`tools/import-throne-second.cjs` imports the enemy/ability/voice tables,
level-two stat deltas, English labels and scalar CRT-affecting sequence keys.
Hashes and raw relevant records are in `data/source/throne_second.json`.
No animation or audio runtime is executed.

Regression coverage includes all 4,096 starting states 0–4095: every simulation
wins in round 2 using all six specified party actions. Trace/non-trace results
agree, trace spans cover every modeled CRT draw, and prior captured opening
battle tests still pass. These are simulator checks, not live-game validation.

The 2026-09-27 live trace is stored in `data/source/live-second-5120.json`.
At seed 0 + 5120, the engine now matches all **144 CRT calls**, their native
caller categories, all six menus, and one leader hit on Pirro. Drops finish at
**5264 / 0x41C7DB10**. Player menus are:

| Round | Character | Advances from seed 0 |
| --- | --- | ---: |
| 1 | Throné | 5170 |
| 1 | Scaracci | 5178 |
| 1 | Pirro | 5195 |
| 2 | Pirro | 5213 |
| 2 | Throné | 5237 |
| 2 | Scaracci | 5247 |

Three source corrections explain the former 5270 prediction. This group has
a BattleStartEvent, so `BattleManagerBP.Start` omits the random starting speaker
and voice (the authored OpenTalk is deterministic). The AI slots above replace
an erroneous two-hit normal attack. Darkest Night permits CRT-consuming break
and kill reactions; normal-attack suppression cannot be reused for this skill.
Changing the intervening states also changes conditional shuffle lengths.

`SoundManagerBP.RequestPlayBattleVoiceByLabel` returns cooked cue `Duration`;
`BP_BattleVoiceComponent.ReceiveTick` decrements `PrevRemainingTime` using
**game DeltaSeconds**. Darkest Night's hit follows its voice by 1.69883 game
seconds, exceeding its 1.21-second timer, even though at 2x the real clip can
still be playing. Slice and Pierce Through hit after 0.3333/0.3 game seconds,
before their 0.79/1.293 timers expire. The engine imports these scalar bounds;
it runs no audio and samples no CRI RNG. The live trace confirms the two
Darkest Night reaction draws at 5173 and 5243.

`tools/audit-second-crt.cjs` checks the relevant Blueprint branches and hashes;
the importer also preserves cue metadata and authored hit timing. The capture
test checks every caller, not just the endpoint. Other seeds are source-derived
predictions; other languages, speeds and arbitrary skill routines are outside
this validated profile.
