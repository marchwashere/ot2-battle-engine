# Carried-party first-menu Escape

`FieldEscapeMenu::with_players(entry, situation, players, levels, enemies,
history)` accepts one to four actual carried characters in their party order.
Supported identities are Throné, Osvald, Agnea, Partitio, and Hikari. It retains
the supplied speed, HP/SP, latent points, equipment, and learned source IDs;
it does not execute those learned skills. The API projects only the first
daytime player menu and direct Escape, with optional boost-cancel inputs.

The entry boundary is after the field selector's player timing draws. Ordinary
enemy initialization, current-order shuffle, first-player promotion,
battle-start speaker/voice, conditional Study Foe, next-order shuffle, and
first order voice use the shared scalar rules. Study Foe runs only when Osvald
is actually in the main party. Carried HP determines the current actor's own
pinch voice. No action has occurred at this first menu, so no previous-action
ally notice is synthesized.

Normal and Preemptive starts are supported; Surprise is rejected. Every supplied
member must be living and have explicit resources. Persistent combat statuses
and BP-start support abilities are outside this bounded profile. Native
`BattlePlayerBP.Initialize` initializes Normal/Preemptive BP to one, so only
zero-ending `[1,0]` cycles are valid before Escape. The saved field BP does not
change this. Each imported actor's level-one boost voice consumes exactly one
CRT draw, while cancellation consumes none. `boost_cancel_cycle()` can advance
a reusable menu cursor; clone it before `attempt()` to discard failed trials.

Ordinary voice data is mapped by identity, independently of party slot. Imported
latent caps are Throné80, Osvald100, Agnea110, Partitio100, and Hikari125. This
projection never changes latent points or other character resources. Inputs
and incoming enemy history are immutable. A failed Escape stops at the check;
no enemy action or continuation is fabricated.

## Canalbrine Bridge water data

The import contains all twelve `ENG_FLD_SEA_1_3_WTR_DAY_*` formations used by
Canalbrine Bridge's three progress tiers, including the five-enemy formation340.
Ordinary enemies are `ENE_NML_WTR_280` and `ENE_NML_FLD_SEA_150/160/170/220`.
Harbor Octopuff is `ENE_NML_WTR_550`. All use their source idle-frame/sync/BP
initialization. Every water group permits Escape, has no battle-start event,
and has no fixed speaker. Source inspection found no water-specific CRT startup
adjustment; map/encounter selection and canoe movement remain the caller's work.

The legacy level-only constructors keep their fixed party interpretation and
output shape. Compatibility tests compare their complete Escape attempts to
the new carried constructor across all one-, two-, and three-member parties.
Additional tests cover every water group, all four first actors, preemptive
guarantees, failed normal checks, identity-based Study Foe, carried speed/pinch,
latent caps, and incremental cursor equivalence.

Regenerate with `node tools/import-field-escape.cjs` followed by
`node tools/import-field-carried.cjs`. Source rows and Blueprint initialization/
voice functions are retained in `data/source/field_escape.json` and
`data/source/field_carried.json`. New four-member water endpoints are
source-derived and await a retail trace comparison.
