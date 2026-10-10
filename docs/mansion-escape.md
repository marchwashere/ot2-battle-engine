The independent `escape::FirstEscapeBattle` profile supports the fixed first
mansion encounter, one Diamante Guard I, solo level-two Throné with original
equipment. It begins AFTER encounter selection and player timing initialization.
The caller supplies HP/SP/latent; starting BP is one. Event 1600 has unlocked the
night talent. Normal/preemptive/surprise first queues are modeled, and a surprise
can execute guard attacks before the first player menu. The wrapper permits only
boost selection/cancellation and the first Escape check. It never attacks or
breaks the enemy. Other formations and post-failure continuation are outside this
profile; the private guard configuration must not be exposed as a general battle.

`tools/import-mansion-escape.cjs` imports the actual enemy stats, normal attack,
voice candidates and group metadata. `tools/audit-escape.cjs` decodes the source
Blueprint functions into the parent workspace's `outputs/mansion-escape-audit`.
The shared core handles initialization, shuffle, enemy actions and CRT voices.
Graphics/audio playback remain absent. Voice predicates that consume main CRT
remain modeled.

Source: `LibBattle.CheckEscapeSuccess` sums party levels, divides by **four** and
clamps to at least one. Subtract the area's danger level, clamp -10..10 and index
the 21-value rate table. Add attempt bonuses [0,15,30,100] from
`ESCAPE_SUCCESS_RATE`, and any support bonus. Use RandomBoolWithWeight on the
binary32 percentage. Solo level 2 versus danger 3 has a 55% first check.
`BattleManagerBP.Escape` bypasses the check on a preemptive first turn; this
consumes **zero** check draws. Escape success does not select a battle voice or
roll victory drops. The route owns the field return's counter reset.

`InitBattleVoice` filters BossFlag/FieldCommandFlag startup candidates before
their lotteries. The mansion start uses the ordinary startup speaker selection
and voice, whereas the preceding scripted fights skip this through their event.
Player timing must not be drawn again at the field-to-core boundary.

Failed `Attempt` ends at the check and is not a simulated failed turn/field return.
Successful attempts end at `Outcome::Escaped`, before field callbacks. Trace spans
are local to the composition boundary. Source-derived; no live capture of this
new stage yet. Existing opening/second live fixtures remain separate evidence.
