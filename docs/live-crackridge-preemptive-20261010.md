# Western Crackridge preemptive opening, 2026-10-10

Retail replay starts at field RNG `0x38FB928B`. Encounter selection consumes
eight calls, selects Wild Mole IV twice and Armor Eater, and gives Preemptive.
Enemy initialization starts at `0x0B6B5B33`. Party order is Throné, Hikari,
Agnea, Partitio; levels are 16/14/14/14 and speeds 130/103/133/92.

The recorded first menu is `0x316CAE5A`, 49 calls after enemy initialization:
9 enemy initialization, 6 current-order shuffle, 1 start speaker, 1 start voice,
13 next-order shuffle, 19 Agnea menu voice. Current order is Agnea, Partitio,
Hikari, Throné. Preview is Throné, Agnea, Armor Eater, Mole, Hikari, Partitio,
Mole. Duplicate Mole identity is not distinguishable from the reported UI.

`BattleManagerBP.AddBattleFirstActionCharacter` checks EnableAddFirstAction
and BattleStartType == 0 before adding the support user. Otherwise it returns
false. Thus A Step Ahead adds no bonus turn on a Preemptive or Surprise start.
`BeginTurn` calls `SetBattleFirstActionTurn` only when
m_bIsFirstActionCharacter is set. Ordinary UpdateNextOrder includes enemies.

The old core incorrectly rebuilt/shuffled four players again (5 calls), and
then filtered enemies out of the next preview (another 5 calls). Retail makes
one seven-actor preview shuffle (13 calls). This accounts for the three-call
error and both incorrect orders; it is not a constant offset to apply.

`tests/live_crackridge_preemptive.rs` reproduces both orders and exact menu RNG,
and ensures safe Defends never cross into the following enemy turn. Normal
starts retain the separately shuffled A Step Ahead turn.

An independent EnemyDB audit confirms UseCatDamageType=true for all ten
Octopuff variants WTR510..600 and all three Cait variants OTR130..150; every
imported field-break profile matches. Unbroken, neutral Shadow Soulstone M
damage is 1100/100 = 11. A 5-HP Octopuff legitimately meets the >=2x remaining
HP Overkill threshold, whereas an 8-HP Queen does not. This replay's reported
levels and speeds already match the progression ledger.
