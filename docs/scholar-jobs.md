# Scholar assignment on carried characters

`jobs::change_secondary_job(player, character, level, previous, next)` applies
or removes Scholar (job 6) on a carried `PlayerCharacter`. The supplied previous
job must match the caller's saved ledger. The helper preserves current HP/SP,
BP, latent power, original equipment, and learned ability IDs; current HP/SP
are only clamped to the resulting maxima. The returned `EquipmentChange`
records stat deltas and default equipment added or removed. Rejected changes
leave the input unchanged.

`jobs::apply_growth(player, character, from, to, secondary)` applies level
growth with the same revision order. The caller retains responsibility for
level-up healing, licenses, inventory, JP, ability purchases, and support
equipment. Neither helper consumes RNG or executes presentation callbacks.

## Source calculation and equipment

The imported `LibCharacter.Calc Player Parameter Internal` obtains
CharacterGrow, adds stat-boosting nuts, and applies the character's parameter
revision. `Calc Player Parameter` then applies the secondary job revision.
`GetPlayerParameterByID` adds flat support corrections and caps parameters;
equipment contributes separately. This bounded API supports the route's
original growth without stat-boosting nuts and preserves carried flat bonuses.

Scholar's imported `JobData` row revises MP to 113%, MATK to 110%, and MDEF to
105%; the other modeled stats remain at 100%. Each revision uses the native
`Multiply_CharacterParam_RevisionParam` helper (`FUN_140fcaf50`, called by
`FUN_140fcd1a0`): multiply integers, convert to binary32, multiply by 0.01,
then floor. The importer checks the existing character progression records
against this native arithmetic at every supported level. Weapon bonuses are
not multiplied by the job revision.

`GetPlayerProperEquipment` unions main-job and secondary-job permissions.
`RefreshWeaponEquipmentState` fills a newly permitted empty weapon slot via
`ItemFunction.GetFirstEquipmentID`. For staff this is `ITM_EQP_ROD_010`, with
zero attack and magic bonuses. This is direct equipment assignment, not an
inventory reward or a selection of the best owned staff. Existing equipped
weapons are retained. Removing Scholar removes a staff that the main job
does not permit. The helper rejects removal of a currently selected main
weapon because the carried model does not yet implement its reselection.

`SetPlayerRoundingHPAndMP` clamps raw saved HP/SP to the new maxima; it does
not heal or preserve the percentage filled. For the route's level-15 Throné,
assignment adds 7 maximum SP, 10 magic attack, and 4 magic defense.

## Abilities and scope

`jobs::scholar_abilities()` exposes the source ability-set IDs, names, costs,
default-on-assignment flags, and all four unmodified AbilityData tiers:

| Source ID | Ability | SP | Default |
| --- | --- | ---: | --- |
| `ABI_SET_SCH_010` | Fireball | 14 | Yes |
| `ABI_SET_SCH_020` | Icewind | 14 | Yes |
| `ABI_SET_SCH_030` | Lightning Bolt | 14 | No |
| `ABI_SET_SCH_060` | Elemental Barrage | 10 | No |

The caller maintains those source IDs in its learned-ability ledger. The job
helper adds the ordinary `attack_staff` command when staff becomes permitted.
Imported spell definitions do not claim support for executing those source
IDs in a battle profile; Elemental Barrage execution is outside this change.

The supported secondary jobs are none and Scholar, for the five imported route
characters. Source evidence is in `data/source/jobs.json`; runtime metadata
is in `data/jobs.json`. Regenerate both with `node tools/import-jobs.cjs`.
The importer retains source hashes, relevant Blueprint functions, and the
native multiplication wrapper/helper bodies.

`tests/jobs.rs` covers exact route stat deltas, resource preservation,
equipment defaults, idempotence, removal, all supported level transitions,
ability metadata, and atomic rejection. These are source-derived regressions;
no retail job-menu capture is claimed.
