# Throné opening cutscenes: CRT audit

Source inspection on 2026-09-25, followed by a live capture on 2026-09-26.
The title-to-battle advance count is now verified for the captured replay.
Full pre-battle simulation is not yet implemented. The viewer provides a
manual `Recreate shared level` toggle: off adds zero calls, on adds one to the
entered advance count. Other pre-battle calls must already be included in that
count. Leave the toggle off when the input already includes this creation.

## Live capture: title screen to Pirro's first menu

The user started a new game as Throné, skipped both opening cutscenes, and
confirmed arrival at Pirro's first command menu. The tracer recorded 43 consecutive main-thread CRT calls,
no reseeds, and no state discontinuities. Its separately read final state
matches all 43 advances. It restored the debug registers and detached.

| Boundary | Position from seed zero | CRT state |
| --- | ---: | --- |
| Title screen, before starting new game | 4916 | `0x137EA674` |
| Battle entry | 4929 | `0x0EA79703` |
| Pirro's first command menu | 4959 | `0xF485D421` |

There are **13 calls before battle initialization**, followed by **30 battle
initialization calls**. The latter match the first 30 calls of the previous
full battle capture in both CRT state and native caller. The 4916 title
position predates attachment; these thousands of positions must not be
attributed to the opening cutscenes.

| Pre-battle consumer | Draws | Evidence/purpose |
| --- | ---: | --- |
| Native timer helper | 1 | Return RVA `0x2C52E57`; the particular timer was not attributed |
| Streamed-level color initialization | 6 | `FUN_140ec60c0` calls `FUN_1411c0100`, return RVA `0x11C010F`, assigning a random level color |
| Field NPC animation start | 1 | Captured VM function `BPC_LevelManager_Others.SpawnNPC`; NPC `NPC_Dng_Cty_3_1_0100_N000`; random starting frame 0..15 |
| Encounter-counter calculations | 4 | Two pairs in `CalcEncountStepCount`, each B then A with maximum 400 |
| Global Black Market lottery | 1 | RVA `0xEA7749`; live RDI metadata matches `GMK_BLACKMARKET_CONTROL` |

The flag lottery runs outside the Black Market's map: its row has no begin/end
flag restriction and uses the time-zone-change trigger. The captured fields
identify it; it is not inferred just from the native function address.

Call order was timer, six colors, NPC frame, first counter pair, flag lottery,
then second counter pair. Source-level event reset flags alone did not explain
the nine other pre-battle draws. Color and animation setup really do consume
the main CRT stream here. A headless model can preserve those draws without
rendering anything.

Evidence: `data/source/live-throne-title-to-pirro-20260926-trace.json`, with
caller annotations and the matched flag row in
`data/source/live-throne-title-to-pirro-20260926.json`. The live regression is
`captured_title_to_pirro_transition_preserves_every_crt_call` in
`tests/live_opening.rs`.

## Repeat after returning to title: 12 draws

A second live comparison on 2026-09-26 confirms the user's report of one fewer
pre-battle draw. The repeat interval contains **12 pre-battle calls plus 30
battle-initialization calls**. Both cutscenes were skipped.

Only the streamed-level color count changed: **six in the first capture, five
in the repeat**. The timer (1), NPC frame (1), counter calls (4), and Black
Market lottery (1) all still executed. Do not explain this discrepancy by
omitting the lottery or by suppressing a counter reset.

The native loader `FUN_140ed4030` looks for the requested entry in its existing
level map. An existing entry bypasses the new-object path and the virtual
initialization call at vtable +0x270. The initialization implementation
`FUN_140ed8170` creates the primary level and requested sublevels through
`FUN_140ec60c0`, which assigns the random level color via `FUN_1411c0100`.
Reusing a level entry therefore bypasses its color draw. Further native-source
inspection identifies the retained entry as **`Btl_SinExpose`**, as detailed
below. The traces did not record level names, so this attribution comes from
the load/unload conditions and asset inventory, checked against both captures.
Do not promote 12 or 13 into a universal offset based on these two runs alone.

The full second capture includes a preceding battle and return to title.
The user-confirmed title marker was after record 201 at `0xEA4F203D`
(position 5131). Before record 202, state changed to `0x5D3D7B67` (position
4917) without a captured `srand`. The tracer did not write that state, and
the cause of the change was not captured. This discontinuity must remain
visible in the evidence. Records 202 through 243 are then consecutive,
ending at the independently read `0xF485D421`; the pre-battle boundary is
again `0x0EA79703`. The 12-call conclusion comes from actual call records,
not subtracting the earlier title-marker position.

The debugger restored its original registers and detached cleanly. Raw evidence
is `data/source/live-throne-repeat-20260926-trace.json`; interval metadata is
in `data/source/live-throne-repeat-20260926.json`.

## Native explanation: retained `Btl_SinExpose`

The extra level is `/Game/Level/Level/Other/Btl_SinExpose`, held by the battle
loader separately from the current field's battle scenery. This is established
from the native code and local assets, not from interpreting the asset's name:

1. The small initializer at preferred VA `0x14065C1B0` constructs the FName at
   `DAT_144f96358` from ASCII `Btl_SinExpose` at `0x143CAD478`, using
   `FUN_14124f110`. This initializer is missing from the text decomp; its
   instructions were read directly from the same executable. Bytes and decoded
   targets are preserved in `data/source/opening-level-loading.json`.
2. In `FUN_140ed98a0` (decomp line 2543104), entering the load state queues the
   next field and its battle-map list. It then separately checks
   `FUN_140ed0fa0(battle_loader, DAT_144f96358)`. Only a false result queues a
   load of `Btl_SinExpose`. The predicate looks up the existing level set and
   checks its loading state through `FUN_140ed1330`.
3. In `FUN_140ed9bc0` (line 2543233), leaving a field queues hide/unload for
   that field and its `BattleMapNameList`. It does **not** unload
   `Btl_SinExpose`. `FUN_140eda7e0` can show/hide the separate level without
   unloading it.
4. Creating a fresh entry goes through `FUN_140ed4030` -> `FUN_140ed8170` ->
   `FUN_140ec60c0` -> `FUN_1411c0100` -> main CRT `rand()`. Retaining the entry
   bypasses this construction and its one color draw.

The `Dng_Cty_3_1` row in `LevelTable` lists
`Btl_Dng_CastleSewer_Land_A` as its sole battle map. The requested sublevels
come from `FUN_140ed31b0`; `FUN_140ed8170` creates only existing assets.
The local asset inventory gives this construction count:

| Streamed level | First captured transition | Repeat with retained shared level |
| --- | ---: | ---: |
| `Dng_Cty_3_1` | 1 | 1 |
| `Dng_Cty_3_1_Art` | 1 | 1 |
| `Dng_Cty_3_1_Coll` | 1 | 1 |
| `Dng_Cty_3_1_Sound` | 1 | 1 |
| `Btl_Dng_CastleSewer_Land_A` | 1 | 1 |
| `Btl_SinExpose` | 1 | 0 |
| **CRT color draws** | **6** | **5** |

There is no `Dng_Cty_3_1_GD` asset and neither battle map has these sublevels.
This also matches the first trace's stack pattern: primary, three sublevels,
primary, primary; the repeat lacks the final primary creation. Together with
the seven unchanged calls, the totals are 13 and 12.

For a future headless pre-battle model, preserve whether this shared level
entry is already initialized, in addition to the CRT state. The relevant
condition is existing loader state, not simply a first-run/repeat switch:
some other earlier transition could also initialize the entry. No level
rendering or color calculation is needed; only its conditional CRT draw.
The viewer's manual toggle projects this one draw into the battle starting
state. It does not automatically track loader state across runs or implement
the complete title-to-battle sequence. Exports keep the input count, recreation
choice and total `battle_entry_advances` separately; checkpoint totals include
the extra call while battle draw counts continue to start at zero.

## Script flow

The cooked event scripts have no file extension:

- `Content/Event/json/MS_TOU_10_0100` starts and waits for
  `RI_MS_TOU_10_0100` (commands 1500/1501). It also sets story flags,
  including the night-talent ban, and branches to `MS_TOU_10_0200` via
  command 1000.
- `Content/Event/json/MS_TOU_10_0200` adds guests 15 and 16, then starts
  and waits for `RI_MS_TOU_10_0200`.
- The second sequence calls `StartBattle` at authored tick 1990000, with
  encounter `EVM_EVE_THI_C01_010`, finish event `MS_TOU_10_0300`, event
  battle true, and single battle false. Its wrapper passes fixed-first-player
  zero. This authored tick is not a measured wall-clock timestamp.

The sequence assets live under
`Content/Event/RichEvent/01_Main/70_TOU/ep1/`. Their event channels contain
57 and 100 keys respectively. These are scripted event counts, **not RNG
counts**. Neither event script requests `CreateRandomNumber`, and neither
decoded sequence contains a direct random-number call.

`BP_RichEventCommandBase.StartBattleFunction` forwards the event-battle
request through `StartEventBattle`. Its character-spawn handlers call
`RichEventCharacter.InitCharacter`; the inspected rich-character preview
flipbook setup uses `PlayFromStart`, not a random starting frame. Do not
apply the separately documented field `SpawnNPC` random-frame draw to
every rich-event character without tracing the actual call path.

## Confirmed conditional CRT consumption

Both `Event/Database/EventList` rows have `EncountReset=true` and
`EventSkip=1`.

`EventManagerBP.FinishPre` checks `PlayEventData.EncountReset` at Blueprint
offset 3342, requires `TheaterStatus == 0` and a valid encounter manager,
then calls `ResetEncountStep` at offset 3470, followed by
`RefleshEncountVolume`. `FinishEvent` invokes `FinishPre` when necessary.

`EncountManagerBP.ResetEncountStep` loads the current map's parameters and
calls `CalcEncountStepCount`. With normal debug settings this consumes
**two main-CRT draws**, B before A. For `Dng_Cty_3_1` both are
`RandomInteger(400)`. The counter is
`trunc((1000 + A + B) * ratio)`, where ratio is 0.5 while `FirstEncount`
is true and 1 otherwise. Resetting the encounter counter advances the
existing CRT stream; it does not reseed it.

Source listings in the parent research workspace:

- `tools/speedrun_audit/Event__BP__EventManagerBP.txt`: `FinishPre`,
  `FinishEvent`.
- `tools/speedrun_audit/Battle__BP__EncountManagerBP.txt`:
  `ResetEncountStep`, `CalcEncountStepCount`.

The two reset flags alone do **not** establish a fixed four-draw pre-battle
offset. The second event initiates battle from inside its sequence; cleanup
ordering relative to battle entry must be established, and other called
functions are not exhaustively audited for CRT consumption.

## Remaining boundary limitations

The captured title-to-battle interval with both cutscenes skipped is verified
as a whole. Neither cutscene
boundary was separately marked, so the two counter pairs are not individually
assigned to particular event completions. Watching versus skipping equivalence
and timing independence are not established by this one capture. The specific
timer's originating Blueprint is also not captured by this tracer mode.

The verified live battle capture begins at seed zero + 4929 advances
(`0x0EA79703`). That input already includes preceding activity; do not add
cutscene advances to it. The simulator currently accepts this battle-entry
state rather than predicting it from the beginning of the opening events.

An earlier-entry model must preserve only the CRT-relevant conditions above.
The 13-draw route is a verified capture, not a claim of a universal offset
across arbitrary input sequences. No visual or audio runtime is required.
Unreal global and CRI RNG remain outside scope.
