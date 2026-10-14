# Gigantes and the revised Sands routine

`gigantes::predict` consumes carried Throne/Osvald/Temenos/Partitio stats,
resources and weakness history. It requires both merchants' Boost-Start and uses
four A Step Ahead members. The caller validates supports, learned skills and gear.

Gigantes, night:

- T1: Throne and Osvald Defend. Partitio Defends only when preceding both;
  otherwise he attacks with his lance. Temenos attacks with his staff.
- T2: Throne +2 sword, Osvald +3 Elemental Barrage, Partitio +3 Beastlings.
  Temenos Defends if his action is reached. Reject an enemy turn, unbroken
  Beastlings target, insufficient BP/SP, or incomplete victory.

Later Defends precede earlier Defends. Both breakers precede Partitio on T2.
Three sword hits plus the minimum six Barrage hits break nine shields before
Beastlings. Night talents last through T2.

With the assistant's level70 entry fixture and manual loadout, the conservative
floor is sword1356 + Barrage17454 + Beastlings32696 = **51506** versus50000 HP.
This excludes both T1 attacks, every critical, every fire-potency bonus and all
extra Barrage hits; sword/Barrage damage is counted entirely unbroken. The
assistant evaluates the floor from the actual carried stats before accepting.

`predict_sands` implements the approved two-turn Sands routine: T1 Scope/Holy
Light/Osvald Defend, conditional Partitio Defend; T2 +3 Slash and +3 Beastlings.
Finisher boost cycles preserve the preceding prefix.

Import order: `import-bg-effects.cjs`, `import-lord-sands.cjs`, then
`import-gigantes.cjs`. The latter imports raw ability rows and sequence markers.
Random count initialization shuffles the native minimum-hit array. Runtime
branches select Barrage elements between hit callbacks; skipped branches do not
draw RNG. Break timers remain on elapsed timeline time across jumps. An initial
counter increment and the terminal Finish section are retained: six to eight
actual damaging hits. Ability voices use base IDs.

Equipment fire potency applies at the native outgoing-damage stage, before
break, ability ratio, buffs and growth. Raw nut doping is added before character
and secondary-job revisions, preserving float32 rounding through level changes.
Nuts do not increase the level-based damage-growth multiplier.

Validation:256 Gigantes seeds cover all three Barrage counts and both Partitio
branches;128 Sands seeds cover six-hit Slash and finisher-prefix preservation.
All66 core unit tests pass. Newly implemented battles still need retail RNG
checkpoint comparison.
