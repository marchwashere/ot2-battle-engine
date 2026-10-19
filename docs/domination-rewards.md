# Domination and A Step Ahead

`BattleManagerBP.AddBattleFirstActionCharacter` registers an A Step Ahead user
only when `EnableAddFirstAction` and `BattleStartType == 0` (Normal). A
preemptive opening does not add the bonus turn. `BeginTurn` skips
`AddBattleTurnCount` only while `m_bIsFirstActionCharacter` is set.

`CalcBonusReward` tests `GetBattleTurnCount() <= RESULT_BONUS_ONE_TURN.Min`.
The source parameter has `Min=1`, `Param=0.1`. `CalcGetReward` applies BG's
multiplier first; `CalcBonusReward` receives that multiplied JP reward.

Thus engine round 2 qualifies when round 1 was A Step Ahead's bonus turn on a
normal start. Round 2 of a preemptive start does not qualify. Round 1 qualifies
in either case. Equipping A Step Ahead alone is not sufficient.

Seed 0x3054BA63 (Pirro first, 154 HP) exposed this in Nameless Isle: Ebony Eel
x3 provides 111 base JP. The selected preemptive two-turn JP100 routine awards
11,100, not 12,210. The previous reward code incorrectly used `round <= 2`
without checking whether the bonus turn actually existed. It did not alter
the already-correct battle turn queues or RNG consumption.

Evidence: `tools/speedrun_audit/Battle__BP__BattleManagerBP.txt` in the source
workspace, functions AddBattleFirstActionCharacter (@000e), BeginTurn (@21d2),
CalcGetReward and CalcBonusReward; `tools/exact_BattleParamDefineTable.json`.
Regression: `nameless_routine::tests::domination_counts_preemptive_turns_but_excludes_the_step_ahead_turn`.
