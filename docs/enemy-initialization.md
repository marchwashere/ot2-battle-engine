# Enemy initialization CRT

The simulator imports scalar initialization metadata from cooked game records.
No animation/audio runtime is executed. There is no Mug-specific RNG branch.

## Source control flow

1. BattleEnemyBP.InitializeEnemy spawns KSBattleCharacterEnemy and calls
   SetSyncAnimationCharacter(MainEnemy, DeadType == 7). Enum7 is eDESTRUCTION:
   MainEnemy synchronizes children; destruction enemies synchronize to a parent.
2. SetupActionTable initializes the resource action table with direction3 (right).
   On this fresh actor, GetIdleType returns the default BAT_IDLE_ENEMY, ordinal130.
3. FinishCallbackLoaded (ubergraph entry11598) resets NowActionID, plays that idle,
   and checks !(IsSyncFrameForChilds || IsSyncFrameForParent). If synchronized,
   it returns without randomizing the starting frame.
4. Otherwise it calls RandomInteger(Sprite.GetFlipbookLengthInFrames()).
   Native FUN_1430d73b0 calls CRT rand only when the maximum is greater than zero.
   Even a one-frame animation consumes one draw. Zero frames consumes none.
5. The BP integer-range draw and initial timing draw still occur afterwards.

## Why this NPC consumes no starting-frame draw

NPCData NPC_Twn_Snw_1_2_A_TALK_1100 selects ActionID SnwSm001_Mle.
Its action130 entries are None in every direction. PlayActionNPC clears the old
Sprite flipbook before attempting to resolve the requested action; that absent
slot leaves no frames. GetFlipbookLengthInFrames (FUN_14091ff00) returns0 for a
null flipbook. Thus the callback reaches RandomInteger(0), which does not call
CRT rand. The synchronization flags are both false for this man.

This is not a blanket rule for NPC fights. An actor with a valid initial animation
and neither synchronization flag consumes the frame draw, regardless of fight
name. The NPC later uses a weapon-specific combat idle; it does not retroactively
randomize this initial frame.

## Implementation and evidence

- tools/enemy-initialization.cjs resolves the resource/action table, sums imported
  flipbook KeyFrames.FrameRun values, and imports the two synchronization gates.
  NPC overrides require the effective NPC action label; unresolved records fail.
- CombatProfile.enemy_initialization is required for OT2 enemy definitions;
  missing metadata is rejected instead of assuming one draw. The field Escape
  projection uses the same initializer and imported metadata.
- src/opening_crt.rs uses CrtRng.index(frames), preserving RandomInteger(0).
- data/source/enemy-initialization-audit.json retains decoded source, table hashes,
  NPC selection and native implementations. The live Mug fixture verifies the
  two enemy initialization draws and menu6192. Other battle live fixtures remain
  unchanged. Unit coverage checks zero/one/multiple frames and both sync flags.

Scope: fresh supported actors with the default initial idle and direction.
Reused actors, alternate initial idles, mode changes and asynchronous resource
failures require their actual effective metadata; no general rendering lifecycle
is being emulated.
