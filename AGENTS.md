- Roque JP100 profile: carried Throne/Osvald/Temenos/Partitio, all ASA,
  Partitio Boost-Start2BP, native Temenos night debuffs. Partitio then Temenos
  Defend -> Temenos first/Partitio second next turn; small Pom supplies3BP.
  Osvald staff rightmost; full BG -> Guards simulation, reject disrupted
  order, insufficient BP, silence, dead recipients or incomplete victory.
  Search both boost paths in total input order; rewards share native logic.
  Profile requires caller-audited skills/supports/inventory. Small Pom item
  and timeline source-imported; no battle-state offsets. Retail trace pending.

# Active project

- Quicksand post-boss land Escape imports include ENG_DNG_DST_2_2_L_120 and
  ENE_NML_ENC_SUB_01 (repeat boss, Escape enabled), plus ordinary land formations.
  Lord Sands predictor exposes finisher boost cycles before Partitio's T3
  Beastlings; unchanged first seven actions, actual boost voice/RNG execution.

- Galdera Lord of the Sands: source-backed eight-action routine, both night talents,
  partial ASA, Slash/Scope/Beastlings timelines and rewards. Refuse enemy actions.
  Defend promotes ahead of earlier Defends/Boots, after break recovery; it does
  not persist in fresh previews. See docs/turn-order-controls.md and its audit.
  Regenerate import-lord-sands after BG import; see docs/lord-sands.md.

- Galdera after Temenos: field Escape supports actual Step Ahead membership
  [0,1,3] via with_step_ahead_members; level1 Temenos without support does not
  enter the Normal bonus turn. Preemptive remains all-player. Import Temenos
  voice identity5/latent110, Warrior job2 and Temenos job revisions. Existing
  all-party callers pass None and keep their ordering/RNG. New route ends at
  Quicksand entry; world controls live in assistant docs/galdera-quicksand.md.

- Temenos Galdera recruitment uses temenos_recruitment::predict: source normal
  attack AbilityRatio80, +1 BP, first A Step Ahead actor; boost/cancel search
  secures a real first-action kill, including Osvald/Partitio critical requirements.
  Separate exact staff/spear/dagger timelines; preserve all other profiles. See
  docs/temenos-recruitment.md. Caller handles source scenes and Temenos/Agnea swap.

- import-highroad-return includes every Eastern Wellgrove Trail land group
  (FST_2_2 DAY/NGT, all progression tiers), native enemy initialization, combat
  and rewards. Assistant carries real players/supports/weakness history for the
  planned post-Champion Seeker steal encounter, demands Normal/Preemptive and
  Escape100. Western Sai menu boosts, theft and field travel stay in assistant.

- import-highroad-return now also imports every Southern Sai Sands land group
  (DST_2_3 DAY/NGT, all progression tiers) and its native enemy initialization,
  combat and reward records. The assistant models the planned first encounter
  using carried level70 party/supports/history and validates native Escape100%.
  Map loading, ambush avoidance and field restoration remain in the assistant.

- gravell_routine exposes finisher_menu/replay_finisher for the assistant's
  Galdera coast extension. Reconstruct the accepted routine with the exact
  earlier actor/actions/inputs frozen, then use actual zero-ending boost inputs
  before Thunder L. Replay victory/EXP100/all alive and native callbacks. Do not
  grant BP to an empty finisher menu. World travel/counter search stays in the
  assistant; Any% and existing EXP100 search are unchanged.

- Cat fixed elemental damage must use EnemyDB.UseCatDamageType: integer divide
  post-resistance damage by100, minimum1. SSM deals11 to Cultured Cait8HP,
  so the coast pack does NOT grant Overkill EXP. Missing this awarded129 extra
  EXP and made Agnea Lv15 instead of14, reversing Highroad menu order.
  Source audit data/source/cat-fixed-damage.json (core).

- carried_shadow_soulstone_m supports a first-menu one-item field victory with
  the actual ordered one-to-four-character party, original/carried gear and
  resources, known weaknesses and optional night talent. It uses the common
  Battle initializer, identity-mapped ordinary/item/victory voices and English
  item duration, aggregate AoE reaction and actual drops. Duplicate enemy IDs
  are allowed only in this bounded field profile and Hikari recruitment.
  Unsupported surviving packs are rejected; no enemy AI is fabricated.
  Night first-menu actor/RNG must match FieldEscapeMenu::with_players: the
  talent moves later player rows but neither changes the promoted first player
  nor refreshes the initial order-speed cache. New coast profile: Sea Scorpion
  865HP, SSM1100 neutral dark damage, Cultured Cait8HP. Caller applies actual
  party rewards and field resets; benched Osvald neither studies nor earns EXP.
  Import tools/import-carried-field-item.cjs; coast source endpoints await retail.

- FieldEscapeMenu::with_players accepts one to four carried route characters
  in actual order, including [Throné,Hikari,Agnea,Partitio]. Ordinary voices
  map by identity; Study Foe requires Osvald in the main party. Source water
  formations include five-enemy Canalbrine groups and Harbor Octopuff WTR550.
  Entry follows field-selector player timing draws. Normal/Preemptive first
  menu only; no enemy actions or failed-Escape continuation. Initial BP1,
  no BP-start support/statuses; carried HP controls own pinch voice. Hikari
  latent cap is125. Existing fixed-party APIs/JSON remain unchanged. See
  docs/carried-field-escape.md. Recruitment predict_with_boost_inputs accepts
  zero-ending paths at both player menus; second-menu-only preserves all
  earlier actions. Each [1,0] cycle consumes one CRT, with no BP spent.

- Scholar assignment uses jobs::change_secondary_job on carried stats and
  jobs::apply_growth for subsequent levels. Apply the source job revision to
  CharacterGrow after character revision, preserving flat equipment bonuses.
  Native integer-product/binary32/floor arithmetic is imported and audited.
  Empty newly permitted staff slots receive ITM_EQP_ROD_010 with zero stats;
  no inventory award or best-weapon selection. HP/SP clamp without healing.
  Caller owns licenses, learned source ability IDs, JP and support equipment.
  The helper adds normal attack_staff; imported Scholar spell tiers do not
  imply action execution support. Original growth without nuts only; see
  docs/scholar-jobs.md. No CRT draws or presentation execution.

- Hikari recruitment uses `hikari_recruitment::predict` and the common Battle
  core with carried [Throné, Osvald, Agnea, Partitio] before Hikari joins. The
  scripted ENM volume -> ENC record -> ENG_EVE_WAR_C00_010 selection is fixed;
  shared initialization still draws player timing. First player uses regular
  Ice Soulstone (260/200/200), second uses its unboosted default weapon on the
  remaining20HP leader. Simulate reachable leader AI; reject grunt turns or
  a third player command. Preserve actual resources/history and apply the
  four released latent-power caps80/100/110/100 on damage. Hikari ID1 has
  imported original starter/growth. Caller owns event/field callbacks, skip
  initialization, item ledger and EXP distribution. Source-derived endpoints
  await retail validation; see docs/hikari-recruitment.md. Voice bitset vectors
  stay length3 for old profiles (identical JSON), length4 only here.

- progression supports newly recruited Partitio ID4 with imported original
  lance/bow/equipment and character growth. route_player(4,EXP) builds the
  corresponding ledger character; recruitment/events/map loading remain in
  the assistant. Four-character combat is currently restricted to the Hikari
  recruitment profile, not the first-menu field Escape adapters.

- FieldEscapeMenu supports the ordered route party Throné / Osvald / Agnea
  (IDs7,6,8), with carried levels, starting equipment and persistent enemy
  knowledge. Three members use imported Agnea ordinary voice rows; Osvald's
  Study Foe runs with both two- and three-member parties. Eastern Cropdale land
  enemy formations are imported for first-menu Escape; no enemy actions or
  failed-Escape continuation are invented by this bounded API.


- EnemyHistory is persistent saved weakness icons plus Study Foe preparation.
  History-aware Escape/break/rare/Mug APIs and BattleConfig receive it; selected
  outcomes export icons discovered in combat. First preparation reveals one,
  repeat reveals two (bounded by closed icons); duplicates do not redraw.
  Western capture regression validates every draw6581..6630, menu6630 and
  failed unadjusted Escape. Never compensate with a fixed two-draw offset.

- Field rare item reactions aggregate target flags before one kill/break/weakness
  voice selection. Do not emit a kill voice per target. Cultured Cait live fixture
  at data/source/live-field-rare-cultured.json validates menu6563 -> drops6572;
  assistant field B/A return reaches6574. No arbitrary CRT offset.
- progression.rs imports per-character CharacterGrow/ParameterRevision and
  actual EnemyDB formation EXP/JP/money. Applying growth preserves equipment
  contributions. Rare item profiles accept carried levels rather than forcing5/1.

- Assistant route hard rule: every planned encounter must avoid Surprise.
  Core field-action APIs reject surprised starts; upstream manipulation is
  the assistant's responsibility. A rare kill does not stop onward travel;
  Evasive Maneuvers affects future encounter movement only after it is equipped.

- Field rare API supports the first-menu Shadow Soulstone (M) clear with
  original Lv5 Throné/Lv1 Osvald. Reject Surprise and formations with any
  survivor; no enemy-AI/second-action fallback is fabricated. Imported CRT
  item schedule, both item/victory voice sets and source drop records supply
  callbacks. AoE item voice plays once for the command, not once per enemy.
  Return core trace and base EXP/JP/money; assistant owns field reset, skill
  learning and Evasive Maneuvers. Added imported snow profiles include Western
  Winterbloom and Cultured Cait/Octopuff Traveler.

- Mansion break simulation must import the full ordinary Throné voice set,
  including normal attack6, final hit7, weakness9, break10 and kill12. The old
  Escape-only import omitted these despite later adding dagger-break support.
  Retail seed0/title4916/shared recreation: enemy-init boundary5525, R1 menu5549,
  +1 dagger break, R2 menu5582. Each authored attack/final-hit voice consumes
  one CRT rate draw. Regression tests assert both spans and the exact R2 state.

- Field Escape imports Southern Cape Cold Snows low-tier day/night formations
  and snow Octopuff. opening_break_menu_with_party supports solo Lv5 Throné or
  original-gear Lv5 Throné/Lv1 Osvald with the real current menu actor and night
  talent. Only one enemy, a successful weapon break, and a following player
  menu are accepted. Osvald staff schedule/duration share the audited Mug asset;
  full ordinary professor voices and Study Foe are replayed. Returned break
  metadata identifies attack/escape actors and actual round. Tests compare
  battle initialization to the independent Escape projection and CRT continuity.
  Source-derived new paths, not live-validated. Assistant owns strategy/cost.

- field_escape::opening_break_menu returns a cloneable real FieldBreakMenu at
  the next player turn after a successful 0/1BP opening sword/dagger break.
  attempt replays next-menu boost inputs ending0 then guaranteed Escape, with
  continuous trace and cumulative CRT offsets. Preserve battle/BP/voice state;
  unboosted initial attack yields2BP, 1BP attack suppresses regeneration to0.
  first_action_break remains the zero-adjustment compatibility wrapper.
  FirstEscapeBattle::break_menu/attempt_broken provides equivalent mansion
  snapshots. Time cost/scoring/search policies belong in the assistant, not here.
  Tests validate actual R2 voice draws, BP legality, immutability and source
  trace continuity. New break routes remain source-derived pending live replay.

- field_escape::FieldEscapeMenu::new_party supports original-gear Throné and
  Osvald first daytime Normal/Preemptive menus. Field selector consumes both
  player timing rolls. Imported Crackridge enemies and Osvald growth/ordinary
  voices; actual shuffled first traveler determines order/boost voice. Study
  Foe draws once per distinct new enemy CharacterID and copies to duplicates
  without RNG (audit-field-party.cjs). Escape uses both levels in existing
  sum/4 formula. No enemy actions/failed Escape continuation or dual-party
  break fallback. Assumes no previously revealed regional weaknesses and
  healthy/non-pinch actors; source-derived, retail trace pending. Solo API
  remains compatible with existing live fixtures.

- Mug completion fixed and live-validated to6299/0xDC86178D. R3 staff hit
  triggers kill voice6293 and delayed JustBreak timer6294 even on a kill.
  Drop6296 -> battle end6297 -> field B/A6299. Osvald attack voice duration
  is5689 ticks, shorter than the9600-tick hit. Tracer detached/registers restored.
  Fixture cape-cold-mug-completion-live.json retains134 continuous calls.

- Enemy initialization now uses imported initial_idle_frames + sync flags, not
  a Mug exception. NPC SnwSm001_Mle lacks BAT_IDLE_ENEMY130; RandomInteger(0)
  makes no CRT call. See OT2BattleEngineRust/docs/enemy-initialization.md.

- Cape Cold Mug: `mug::predict`, imported NPC_Twn_Snw_1_2_A_1100, original-gear
  Lv1 Osvald. Icewind -> 2 BP Fireball -> unboosted staff if needed. Headless
  scalar projection includes Study Foe's one weakness draw, Osvald voice rows,
  and Pattern AI's Stand Ground with two independent status applications.
  NPC Pattern does not use NormalEnemy's charged BP reroll. DEF_UP/MDEF_UP
  final ratio is 0.667. Longer combat is rejected. End after victory/drop;
  field callbacks remain in the assistant. Retail validates initialization
  from6165 to first menu6192: Mug NPC skips ordinary sprite-frame draw,
  retains BP/timing. Later source prediction ends6295, damage53/226/11. See
  docs/cape-cold-mug.md. General per-character EXP/JP ledger still absent.

This is the new standalone Rust simulation core. The sibling C++ project is
research/reference material, not a runtime or build dependency.

- The simulation core is HEADLESS: no game graphics, UI, animations, particles
  or audio playback. The result viewer, route orchestration and field encounter
  predictor live in the sibling `OT2SpeedrunAssistant` repository, which imports
  this crate. Never add a reverse dependency on the assistant. This repository
  must build and run battle manipulation sweeps independently.
- Track only the main CRT rand stream. Ignore Unreal/CRI RNG unless it changes
  battle outcomes or correct CRT advancement. Preserve relevant conditional draws.
- Prioritize a running small battle and explicit supported behavior over general
  Unreal compatibility. Do not port the old interpreter/object system.
- Use structs/enums, small mutable state and shared immutable definitions.
- Reject unsupported data. Do not silently substitute guessed OT2 behavior.
- The opening Throné encounter runs under source-derived `ot2_opening_v1` rules.
  Keep `scaffold_v1` for synthetic tests. The mechanics and imported records are
  tested, but full live-game CRT alignment is not established; see
  docs/throne-opening.md and docs/opening-routine.md for coverage and the exact
  seed/exit boundary. Never claim manipulation
  accuracy from reproducible simulator output alone.
- Keep the existing workspace and old engine unchanged. Work in this repository.
- The second Chapter 1 sewer fight is ENG_EVE_THI_C01_020; use the level-two
  configuration by default. Pirro's ABI_SET_GST_0190 is Pierce Through, not
  Shred. The fixed second routine uses 2 BP on all three round-two actions.
  Its CRT boundary ends after drops, without field resets. The 2026-09-27 live
  capture matches all 144 CRT calls and six player menus
  for seed 0 + 5120, ending at 5264. Keep BattleStartEvent startup gating,
  normal enemy slot 1 / charged slot 0, and scalar game-time skill reaction
  gates. Other seeds remain source-derived; see docs/throne-second.md.
- escape::FirstEscapeBattle is a deliberately bounded independent profile for
  solo level-two Throné versus the first mansion Diamante Guard I. Entry follows
  the field selector's party timing roll; skip that player draw in this profile.
  It exposes initialization, any pre-menu ordinary enemy actions, boost input
  replay and the first Escape check only. Do not expose its private guard config
  for later combat. Failed attempts stop at the check; no field reset or invented
  continuation. Successful attempts return Escaped before field callbacks.
  Escape uses max(sum(levels)/4,1) versus area danger; first preemptive turn
  bypasses the RNG check. Night talent is enabled here. Main CRT only, including
  conditional voice rolls; no graphics/audio execution. See docs/mansion-escape.md.
  Live alignment for this new stage remains unverified.

- Pirro Chapter 1 support: Battle::new_pirro and pirro::PirroRoutine run standalone.
  Solo level-two original-gear Throne, 3 BP for both max-boost steps. Branch at R7
  using R8 preview; latent gives two consecutive actions, not BP recovery.
  Import all boss records/schedules, fixed soulstones, hold-out/mercy and latent
  tutorial. Carry actual resources/inventory from callers. Never force victory:
  early criticals can trigger the second phase and defeat the supplied routine.
  Charged FIRST/LAST multi-order paths outside this fixed routine stop explicitly;
  do not approximate them with a normal appended turn. Boss CRT has partial
  live coverage; see docs/throne-pirro.md and the captured loss below.
  PirroRoutine accepts exact boost paths; preview_boost_inputs exposes actual
  conditional draw costs without mutation. Pirro's 50% special Lv3 voice means
  2->3 consumes 3 or 4 CRT calls, not the earlier fights' fixed three.
  Pirro's TalkPlayerID=eTHIEF selects Throné directly: no random speaker draw,
  but the battle-start voice-rate draw remains. The user-reported seed-0
  5595 pre-cutscene -> 5631 first menu matches (+36); this single endpoint
  does not validate later actions. Preserve the random fallback for groups
  without an available designated speaker, even in a solo party.

- 2026-09-28 Pirro capture: seed 0 +5678 -> 5881, all 203
  calls and menus 5714/5752/5783/5813/5860/5870 match; R4 critical38 and R7
  defeat reproduce. Track discovered weaknesses for the conditional type-9
  reaction (kill/break take priority), and the one-time phase-mode CRT frame
  draw after action/round callbacks. GetActionCommandForItem overrides the
  generic USE_ITEM row with UseAbilityCamera=true. Import that scalar flag for
  items and the data flag for skills: PlayFinishBlow skips its timer while
  Playing; new-break timers and normal-attack broken-hit timers still draw.
  R6 Shadow Soulstone now reaches round timing at5872 and defeat ends at5881.
  Use schema-based decoding and reject missing requested functions; the older
  exploratory helper silently omitted the camera ubergraph/item builder.
  No final offsets or blanket item exceptions. Only this captured loss is
  fully CRT-validated; other winning/boost-search paths need live coverage.
  Entry5595 R7 menu report5798 is also covered: never apply NormalEnemy's
  BoostActionCheck/reset/reroll to Pirro when BP fills. His boss AI overrides
  do not call it. See tools/audit-pirro-boost.cjs and source/pirro-boost-gate.json.
  Latent unlock is55/80, imported from PotentialityActionDB. The sequence
  director releases the player directly; do not use the manager's full-gauge
  helper. Entry5595 reaches R8 with55 points; another break gives25 and fills
  it. In the no-grape branch, check actual readiness at R8: if unavailable,
  use max-boost swords without latent, then R9 latent -> Wind -> Ice and R10
  sword. User confirmed this sequence. Keep the ready-R8 and grape branches;
  never grant a full gauge or force unavailable activation. Expose the delayed
  branch in routine output so the assistant can show only the changed turn.
  R10 Pierce Through (穿心), ABI_BOS_THI_C01_010_008, uses HP_RATIO_DAMAGE
  (CalcType 2). JudgeSpecialHitRate gates ATTACK_HIT_FAILED only on CalcType 0;
  never force the percent-HP effect to miss. Preserve both 21-call status
  applications, its accuracy draw, and successful damage-reaction traversal.
  At entry5595, reported R9 menus5832/5836 and R10 menu5892 match; damage is
  floor(261*1/100)=2, exit5900 including the death timer below. The previous5889 menu/5896 exit skipped one
  accuracy and substituted one avoid voice for three damage voice checks.
  See audit-pirro-mercy.cjs. These are reported endpoints, not a per-call capture.

- 2026-09-29 winning replay supersedes the endpoint-only caveat for entry5595:
  all305 CRT calls match through drops at5900 / 0x533B5D0C, including R10menu5892.
  EnemyData.DeadEffectSlow schedules FinishEnemyDeadSlow via K2_SetTimerDelegate
  after final damage UI, before victory voice/drop; retain that one CRT call.
  Import the gate; never add a final offset or execute graphics. The phase-frame
  callback follows order voice (5796voice,5797frame); the loss's next enemy voice
  consumes zero. Both captured paths remain tested by state and native caller.
  See audit-enemy-death-slow.cjs and data/source/live-pirro-5595-victory.json.
  Other seeds/boost plans still need live coverage; this is not universal validation.

- field_escape::FieldEscapeMenu is a bounded healthy solo first-menu projection
  for ordinary daytime highroad formations. Import enemies/growth/equipment with
  tools/import-field-escape.cjs; share enemy initialization, shuffle, voices and
  Escape probability instead of introducing final RNG offsets. Field selector
  already consumed player timing. Support Normal/Preemptive only; reject Surprise
  and unknown enemies explicitly. No enemy actions or failed-Escape continuation.
  New highroad endpoints remain source-derived, not live-validated. Route/map/Al
  orchestration and boost search remain exclusively in the assistant.
  preview_boost_inputs_after_latent shares actual latent activation with execution;
  R8 early latent activates before the path, late latent uses the path without
  activation. Both finish R8 with 3 BP swords. Preserve all prior live fixtures.
  first_action_break additionally supports a lone highroad enemy and the unchanged
  level-five loadout. Reuse actual normal-attack accuracy/damage/weakness/shield
  callbacks and imported ordinary reaction voices. Try 0 BP then 1 BP, dagger
  then sword at equal BP, and accept only a living enemy broken on that action.
  Return guaranteed Escape at the next menu while it remains broken. Reject
  enemy actions explicitly; its internal AI template is never executed. Preserve
  menu/trace continuity with the independent first-Escape projection. No offset
  compensation and no invented enemy or failed-Escape continuation.

- Boosted field spell breaks: permit +1 Fireball/Icewind/Darkest Night at the opening menu and price the actual boost path (no mandatory final cancel). Replay the real BP damage tier and resources; reject victory or any surviving unbroken enemy. Physical zero-BP policies remain unchanged. Snows/Eastern searches must evaluate odd cost layers. Regression compares [1] Fireball with [1,0] on the retail two-mole fixture: both survive/break, RNG endpoints match, and boosted cast saves one input.

- carried_soulstone generalizes carried_shadow_soulstone_m to Wind Soulstone L, 2700 fixed wind. Uses actual source ItemDB sequence (damage tick68800), common enemy-item voice, shared battle damage/victory/drop mechanics and actual carried party. import-carried-field-item imports both sequences; import-highroad-return also imports reward rows. Route caller must apply full rewards/resources and level-ups for this additional ordinary fight victory.

- 2026-10-10: Gravell BG/Guards pending status superseded. All39 max-BP effects execute in battle/grace.rs; complete minimum-input fight search/replay and real victory/all-alive/EXP100 acceptance in gravell_routine. Assistant integrates rewards/progression/field return and boost instructions. Unplanned enemy continuation is rejected, not fabricated. See core docs/gravell-search.md; new flow still awaits end-to-end retail validation.

- Gravell routine revised: T1 earlier merchant +1 Donate BP to later merchant, later merchant +3 Guards, Agnea Defend, Throne Pom M to Agnea; T2 Agnea +3 BG then next player Thunder L. Both merchants learn Donate BP. Strict propagation freezes first Guards state with sufficient damage for Thunder L, then searches BG; victory/EXP100/all-alive unchanged. Supersedes previous Agnea-first/Stimulate/Boots branching routine.

- Larger battle order queues: remove old eight-combatant restriction only with full native introsort port (combat::sort_order). Native midpoint partition, binary32 natural-log depth, selection leaves and heap fallback including ties. 325 original-instruction oracle fixtures cover 0..64 actors; step-ahead/Defend tests cover four players plus five/six enemies. Source C++ native_order_sort.hpp and generated native instruction fixture; do not replace with standard stable/unstable sort.

- Field enemy imports must include all GameParamDefineTable RARE_ENEMY_* replacements, not only base group members. import-highroad-return now derives all13 unique Cait/Octopuff IDs, their initialization/combat data and rewards. ENE_NML_WTR_580 was missing on Wildlands search candidates. Regression covers ordinary and A Step Ahead Escape plus safe Defend for every rare variant.
