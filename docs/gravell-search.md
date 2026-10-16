# Gravell battle search implementation status

Implemented (2026-10-10):

- `field_escape::carried_step_ahead`: separate all-party bonus-turn profile,
  retaining the ordinary preview and actual carried stats/resources/knowledge.
  It assumes the caller has equipped A Step Ahead on every supplied traveler.
- Source sequences and voice rows for Stimulate, Springy Boots, Pomegranate M,
  Thunder Soulstone L and Defend. Stimulate shifts current unacted and next rows;
  Boots applies BeTheFirst and promotes the next row.
- `gravell_routine::OpeningPlan`: all 24 opening orders, including Stimulate's
  reorder and Thunder L before Agnea when she acts last.
- `bewildering_grace::Dance`: all four source weighted tables and a resumable
  select/complete-child protocol. A pending child prevents another selection.
  Caster death or enemy elimination stops further selections.
- `hired_help::prepare_guards`: four weighted variants without replacement,
  including the fourth CRT draw even for +2 BP. Summon count is boost+1.
- `fight_search::BoostFrontier`: minimum-input search of unique RNG/selected-BP/
  voice states from a cloned command menu, with resumable input budgets. Actual
  supported actions are replayed before returning a candidate. Trace accumulation
  is disabled on intermediate search nodes; selected replays retain it.
- Ordinary damage now applies EnemyDB.UseCatDamageType before multipliers,
  independently of the existing fixed-damage /100 path.

## Complete prescribed fight search

`Battle::grace_step` executes all 39 +3 BP children, both nested summons,
transformation, and the overlapping Guards timelines. Each lottery runs only
after the preceding effect completes. Negative outcomes are executed, not
blacklisted: revival, resource changes, silence, poison, scattered items,
buffs/debuffs, transformations and altered order all affect continuation.

`gravell_routine::search` advances minimum-input BG and Guards frontiers.
The selected candidate is replayed from the original battle. Acceptance requires
actual victory, EXP100, and all four travelers alive at rewards. A blocked action,
an unplanned enemy turn, or surviving enemies after the prescribed routine is
rejected. This bounded routine does not simulate arbitrary additional enemy AI
or invent recovery actions to rescue a failed candidate.

`gravell_routine::rewards` calculates final enemy rewards (including transformed
enemies), native float reward multipliers, battle bonuses, drop RNG, and the
Guards cost. The assistant carries HP/SP, EXP, JP, growth and enemy knowledge into
the field return. Its existing world model does not maintain a money inventory;
the cost is exposed in the fight reward result.

The assistant now shows the real BG/Guards boost paths and command-menu ordinals.
Detailed log contains the complete CRT callback trace and post-victory field
checkpoint. No placeholder or assumed EXP100 victory remains.

Source details preserved:
- Mushroom buffs enemy ATK/MATK/DEF/MDEF/AGI for five turns.
- Silence uses 0..2 additive duration in all status slots; Poison starts at nine.
- eALL includes both sides; party-single selection precedes dead filtering.
- Transform shuffles all enemy slots and checks DiseaseResist[8]; Chubby is immune.
- Guards prepare four variants even when only three are summoned. Child voices,
  damage and order delay are merged by authored time across overlapping summons.
- Rewards retain the maximum requested EXP/JP multiplier, not their product.

## Evidence and verification

Importer: `tools/import-gravell-battle.cjs`; source dump:
`data/source/gravell_battle.json`. Source blueprints:

- BattleActionOrderBP.SetBattleFirstActionTurn and BattleManagerBP turn flow.
- LibAilment.Battle_OrderCountUp and Battle_ExecAddDisease.
- RandomSelectAbilityCommand.SelectRandomAbility and its child completion flow.
- SummonAbilityCommand.SetupSummonData (loop0..3, then SummonCountMax=boost+1).
- CalcAilmentDamageHP's Cait base `(Attack-Defense)/96+2`.

Automated tests cover all 24 routine orders, setup action resources/order changes,
bonus-to-normal BP transition, exact clone/replay search behavior, lottery source
positions (seed0 EXP100 at3286 and JP100 at306), early termination, and exhaustive
32,768-variance Guard/Cait damage (31..32 without criticals). New initialization
and setup callbacks are source-derived and still require retail validation.

Full core suite and winning search/replay tests pass. These new Gravell callbacks
are source-derived; an end-to-end retail capture has not yet validated them.

Validation run: 150 core tests pass; the assistant full-Gravell continuation
fixture passes and the live UI/API route at title advance19284723 completes.
The broad assistant library sweep completed132 of133 checks, then was stopped
after more than five minutes in a legacy route-search test; it is not reported
as a fully passing assistant suite. No assertion failure was reported.

## Revised fixed routine (2026-10-10)

T1 is A Step Ahead: earlier merchant +1 Donate BP to the later merchant; later
merchant +3 Guards; Agnea Defend; Throné Pomegranate M to Agnea. T2 Agnea +3 BG,
then the next player Thunder L. Both merchants learn Donate BP (6 SP; +1 BP
grants2 BP), with JP and source ability voice/sequence tracked. No Stimulate or
Springy Boots branches remain in this routine.

Search uses strict propagation: choose the earliest Guards boost state leaving
a living formation that Thunder L can finish, freeze that result, then search
BG at the next controllable menu. This minimizes each local adjustment, not
the combined global input total. Every BG candidate is fully executed and
negative effects can invalidate the otherwise sufficient finisher.


Donate BP retail checkpoint (2026-10-10): seed 0, Hikari menu 19290014,
+1 BP Donate BP to Partitio, Partitio menu 19290035. Capture
`data/source/live-donate-bp-20261010.json` verifies boost +1, ability voice +1,
recovery display timer +1, recovery voice +1, next menu voice +17.
BattlePlayerBP ExecuteUbergraph BP recovery at offsets11907..12843 executes
the recovery UI before OnRecovered; LibAilment Battle_ExecRecoveryBP reaches
this path. The shared scalar recovery handler formerly reversed timer and
voice, producing eight recipient voice rolls here instead of one. No fixed
advance offset is applied. HP/SP/BP share display-before-voice sequencing;
latent recovery has no scalar display timer.


Guards retail checkpoint (2026-10-10): Partitio menu19290035, +3 BP
Guards, Throné menu19290104. `data/source/live-guards-20261010.json`
retains the 69 actual CRT calls for the replay segment (the original capture
also contained earlier actions and a manual reset, excluded here).
LibAilment.Battle_ExecDamageHP loops prepared targets with IsValid, without
an HP/alive skip. CheckHit returns true for IsCharacterDisableAction, so
already-killed targets still consume critical/variance/display calls without
an evasion roll. Each Guard is the damage enforcer, not the hiring merchant;
Throné must not select a Partitio-specific ally-break menu response. The old
simulation skipped two dead targets (five draws here), and wrongly allowed
the ally-break response (12 fewer draws here). The regression checks each
span against captured call sites and CRT states, not just the final count.

## Cost search refinement (2026-10-10)

`search_target` now searches Guards/BG jointly in total input-cost order instead
of permanently freezing the first finisher-ready Guards roll. Optional
`search_target_windowed` bounds Guards alternatives and accepts a shared work
counter for interactive route refinement. Exhausting work means incomplete search,
not impossibility. Callers retain a validated baseline and must not negative-cache
that result. All accepted candidates replay the original battle with full tracing.

Boost search merges only equal full mutable BG states across Guards histories.
Target deadlines use the exact 32-bit CRT distance (`CrtRng::distance_to`), not
estimated action draw counts. Probe trace filtering changes recorded diagnostics
only; accepted replay restores every span. Retail Donate/Guards regressions and
normal attack/action execution remain unchanged.

## Osvald Merchant voice regression (2026-10-16)

The Gravell voice importer previously restricted Donate BP and Guards cast voices
to MERCHANT/FENCER. Galdera's Osvald also requires the source BattleVoiceSet rows
`PROFESSOR_ePLAY_ABILITY_ABI_MER_050_01` and
`PROFESSOR_ePLAY_ABILITY_ABI_MER_070_D_1`. The missing Guards voice skipped one
draw, changing subsequent hit outcomes and producing a net five-draw discrepancy.
Osvald's unused Dancer ability voices are omitted to retain the route profile's
64-row mask limit.

`osvald_guards_live_1e10aa9c` replays the unchanged reported inputs: Osvald's
menu is 4772, Guards hits Chubby Cait four times and kills it, and Throné's menu
is 4844. Subsequent BG effects remain self-heal, EXP x100, JP x2, party defense
down. The fixture records player-observed checkpoints, not a retail per-call
trace. No RNG offset or changed hit formula is used.

Follow-up retail checkpoints exposed another omission in BG status callbacks:
BattlePlayerBP.OnTakeDisease (script 0x7525b) requests eTAKE_DAMAGE with
DECONDITION for debuffs/deconditions. BP_BattleVoiceComponent checks the counter
row then selects the debuff row (two draws); its settled-damage flag suppresses
the second defense-down reaction in the same action. Apply this inside slot zero's
AddDisease callback, before the remaining None-slot duration rolls. Four party
reactions add eight draws. The unchanged-input regression now matches Agnea's
menu 4889, pre-BG 5002, effects complete 5184, and Osvald's menu 5203. The resulting
field-return prediction is 5212; this endpoint is simulated, not yet retail-verified.
