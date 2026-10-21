# Route voice audit — 2026-10-16

Scope: implemented manipulation routes, including prologue/Pirro, mansion and
field actions, Cape Cold Mug, Hikari/Temenos recruitment, Gravell EXP100,
Lord of the Sands, Gigantes, and all three JP100 variants. This is not an audit
of the unimplemented final Galdera battle or arbitrary enemy AI.

Run `node tools/audit-route-voices.cjs` for the checked-in source-data report.
It checks 14 CRT profiles and every imported row's source parameters, plus
explicit caster/ability and reaction contracts. It separately checks all 39 BG
child IDs for Agnea and Temenos: a missing cast row is allowed only when the
source has no such row. Runtime tests remain necessary: complete data alone
cannot prove callbacks execute.

## Confirmed defects corrected

- Osvald Guards/Donate BP: source rows omitted by the old Merchant/Hikari-only
  importer. Covered by the unchanged-input retail checkpoint regression.
- BG debuff reactions: OnTakeDisease requires damage-type voice selection with
  DECONDITION. Four two-draw reactions reproduce 5184 before the next menu and
  5203 at Osvald's menu; field-return 5212 is now user-confirmed.
- Ordinary carried field Defend: `carried_field_item_crt` lacked type17 entirely.
  This affects ordinary pre-Cultured-Cait Defends, not only ASA battle Defends.
- Temenos identity was absent from recovery/revival enforcer and ally-break
  mappings, preventing character-specific reaction candidates from matching.
- Command-menu condition always used `decondition=false`. Poison/Silence now
  select status-aware voices, with source precedence over own low-HP condition.
  A preceding ally break overrides those own conditions.
- Buff reversal directly invokes AddDisease and therefore also needs the
  debuff reaction callback. It now shares the same settled-reaction gate.
- Correction, 2026-10-22: Silence does **not** block damage/avoid/recovered
  voices. The previous interpretation misidentified the numeric disease IDs.
  The native enum resolves the gate to Sleep, Sleep Doom, Frozen, Stone and
  Stun; Silence is ID1 and absent. See `bg-voice-audit.md` for the source-backed
  correction and seed7B4971B0's eight missing recovery draws.

## Source callback coverage

| Actions | Voice path checked |
|---|---|
| Normal attacks, scripted prologue skills, Pirro and Mug | Authored cast/attack events; damage, avoid, death, break, weakness, kill and victory callbacks |
| Defend and boost adjustments | Guard/boost candidates and source rates |
| Soulstones and Poms | Item voice, recipient recovery identity, aggregate target reactions |
| Donate BP, Stimulate, Springy Boots | Ability-ID mapping and imported caster rows |
| Guards/Beastlings | Owner casting voice plus each summoned actor's source voice |
| Fireball, Slash, Barrage, Holy Light, Critical Scope | Correct caster profile and source ability aliases, including conditional Barrage rows |
| BG 39 outcomes | Parent cast; child cast-row presence; damage/death, recovery/revival, debuff/reversal, native disease voice gates and next-menu conditions |

Evidence: `BattlePlayerBP.OnTakeDisease` (script 0x7525b),
`CanPlayVoiceWhileDisease` (0x7a160), `IsCharacterDecondition` (0x71431),
`BeginCharacterTurn_OLD`; `BattleCharacterBaseBP.AddDisease`;
`BP_BattleVoiceComponent.PlayBattleVoice`, `CheckOnTakeDamageVoice`,
`ResetVoiceFlagForPreAction`; source BattleVoiceSet and authored action timelines.

## Remaining confirmed gap

Ally-low-HP command reactions are not represented. Player OnDamaged records
`SetPrevNoticeCharacter_Pinch` when crossing into low HP; BeginCharacterTurn
queries that manager notice and can overwrite even the ally-break condition.
The simulator currently handles only the acting character's own low HP.
BG damage/HP-one outcomes can reach this path. Before fixing it, audit the native
manager notice's reset/consumption lifetime and eligibility (including death and
self-recipient); do not invent a persistent last-damaged actor or add a draw offset.
Thus the audit does not certify all BG outcomes as fully RNG-aligned, despite
the passing regressions and the confirmed 1E10AA9C case.
