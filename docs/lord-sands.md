# Lord of the Sands Galdera routine

Source importer: tools/import-lord-sands.cjs (run after import-bg-effects.cjs).
Dedicated config/CRT data keep original route profiles unchanged. The player
fixture comes from the saved Galdera 32-observation route at Quicksand entry.

Prescribed routine: three-member A Step Ahead; T1 Throne +1 dagger, Osvald/Partitio
Defend; T2 Throne Critical Scope, Osvald +2 Aggressive Slash, Partitio Defend,
Temenos staff; T3 Partitio +3 Beastlings. All eight actions execute in actual order.
No fallback enemy AI is permitted: an unexpected actor/order or surviving boss is
an error. Enemy AI in the config is unused. Dungeon traversal/event loading are
owned by the assistant; user confirms no intervening encounter.

Import preserves the first-fight enemy ENE_NML_FLD_DST_280 (not repeat-encounter
ENE_NML_ENC_SUB_01), 7 shields/40,000 HP/270DEF. Boss voice rows are imported for
this profile, including actual action voices. Temenos night debuffs use direct
AddDisease:3 turns DEF/MDEF down,1 blind; no status lottery. Critical Scope uses
ADD_FORCE_CRITICAL's check and20 duration slots. CheckCritical returns true at
source offset1676 before any critical lottery when FORCE_CRITICAL is present.

Aggressive Slash uses its actual +2BP row, count5 and authored Finish marker,
random-count setup plus five-entry minimum-hit shuffle and target selections.
It switches to sword, spends9SP and handles native breaks/reactions. Beastlings
prepare four weighted variants without replacement, use identical400ATK summon
stats, actual voice callbacks and four child start times from the parent sequence.
SP_DAMAGE sets additive break damage rather than directly executing a shield hit;
this bounded routine only accepts the already-broken boss. No new shield or shield
RNG is synthesized. Summons ignore the hiring character's attack buffs.

Defend moves to the front of the next queue, after recovering broken enemies.
Earlier Defends and Springy Boots do not block it. The old interpretation of
CheckOverTakeEnable's order type6 was wrong: that is BREAK_STYLE, not GUARD3.
Partitio's T2 Defend precedes Osvald on T3 regardless of the lingering Guard
status. See `turn-order-controls.md` for the corrected shared implementation.

Predictions include all action reports and continuous absolute RNG spans,
party resources/history, Break/Overkill EXP,37 base JP (no Domination on T3),
Untouched leaves,15,000-leaf cost and actual3% Jam drop. Defeat flag and native
field return live in assistant. No damage rolls are clamped artificially beyond
the game's9,999 cap. Tests exercise128 seeds, real capped Beastlings damage,
actual order, and all existing core regression fixtures. Retail validation pending.
