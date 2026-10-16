# Advanced Magic and Fire Storm

Implemented battle commands: `ABI_SET_SCH_070` (Advanced Magic), and
`ABI_SET_SCH_011` (Fire Storm). While Advanced Magic is active, a learned
`ABI_SET_SCH_010` command resolves to Fire Storm automatically. Fire Storm
cannot be cast without an upgrade charge. Learning remains caller-controlled;
adding definitions to the catalog does not grant a character either skill.

Advanced Magic targets a living ally, including the caster, costs 15 SP, and
adds 2/3/4/5 uses at BP0/1/2/3 (stacking up to nine). The status count is not
reduced at round transitions. `BP_ActionCommandBase.ExecPayPostCost` consumes
one use after the entire upgraded action, including a battle-winning cast.
Fire Storm costs 14 SP, hits every prepared enemy twice, uses invocation105,
and has per-hit AbilityRatio85/150/215/280. Each hit has independent damage
rolls. The shared CommonCommand executor processes one whole target pass per
authored effect, including the final hit on prepared targets reduced to zero
HP by the first hit. Ability-camera gating suppresses broken-hit slow timers;
new breaks retain their shared delayed timer.

`tools/import-scholar-magic.cjs` imports all eight AbilityData rows, the
AbilitySetData upgrade link, ADD_SUPER_MAGIC's 20 disease slots, SUPER_MAGIC's
non-turn-based metadata, both authored sequences, and cast voices/durations
for the six currently supported route identities. It also saves decompiled
Blueprint evidence for menu replacement, post-action use consumption, disease
counter updates and disease reactions in `data/source/scholar-magic.json`.

Both cast voices are unconditional non-PlayOnce rows (one RNG check), imported
separately to preserve existing route profiles' 64-bit voice masks. The importer
checks that Fire Storm's cast voice ends before the first damage event. Existing
reaction and next-command-menu voice processing remains shared. Advanced Magic
uses the status pipeline: application check, first slot duration draw, disease
application, then the other nineteen slot draws. No buff-received voice is
requested by this source disease category.

Validation: tests cover all BP tiers, exact cast/status draw counts, charge
persistence and consumption, automatic upgrade/reversion, target/hit ordering,
shared break timer, final-hit kills, deterministic replay, silence/invalid-command
atomicity, and the six cast-voice profiles. These are source-derived tests;
no retail capture of these two skills has been supplied yet.

Scope: other advanced spells (Icewind/Lightning Bolt upgrades) are not included;
the engine rejects those commands while Advanced Magic is active instead of
silently executing their unupgraded variants. This change adds battle capability,
not new actions to the currently agreed route routines.
