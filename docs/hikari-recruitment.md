# Hikari recruitment battle

`hikari_recruitment::predict(before, players, history)` runs the source-derived
two-command recruitment fight through the common battle engine. It accepts the
ordered carried party Throné, Osvald, Agnea, Partitio, including actual stats,
equipment, HP/SP/latent points and saved enemy weakness history. Hikari is not
a combatant; the caller handles his later event-based recruitment.

The routine uses one **regular Ice Soulstone** at the first player menu, then an
unboosted default-weapon attack against the surviving leader at the next player
menu. It simulates any leader turn between those commands. An unsupported
continuation or a failure to win returns an error, without mutating the inputs.

`predict_with_boost_inputs(before, players, history, first, second)` additionally
replays boost-button paths before either fixed command. Both paths must return
to zero. `BattlePlayerBP.Initialize` resets BP to `DEFAULT_BP_VALUE.Init = 1`
for this Normal event battle, regardless of the saved field BP; BP-start support
abilities are outside this profile. Thus the legal cycle is `[1, 0]`, repeated
as needed. Each rise consumes one ordinary boost-voice CRT draw for every
possible actor; cancellation consumes none. Second-menu-only inputs preserve
all earlier battle steps. A nonempty path appears on the corresponding output
menu as `boost_inputs`; `predict` retains its empty-plan output shape. Literal
input resources remain in `players_before`, while `battle_config` records BP1.

The entry boundary is immediately before shared battle actor initialization.
Event command 600's `ENM_EVE_WAR_C00_010` volume selects encounter
`ENC_EVE_WAR_C00_010`, whose first group is `ENG_EVE_WAR_C00_010`. This is a fixed
event lookup, with no field formation or battle-situation lottery. Normal
startup still initializes all four players and all three enemies, generates
both turn queues, selects a battle-start speaker and runs Osvald's Study Foe.
Duplicate grunt types share Study Foe history.

The leader `ENE_EVE_WAR_DST_090` has 280 HP and ice weakness. Each of the two
`ENE_EVE_WAR_DST_080` grunts has 180 HP. The item deals 260/200/200, leaving only
the leader with 20 HP. The current EXP values 1530/1467/0/0 give levels
15/15/1/1. All four default attacks have guaranteed accuracy and minimum damage
of at least 20 against that remaining leader. Tests also bound the leader's
maximum first-round damage below every full-health route member's HP.

The 1,024-seed regression covers every traveler at both player menus, including
Agnea and Partitio, and leader actions between the menus. A separate lower-level
5/1/1/1 corpus includes correctly rejected outcomes: Lv1 Osvald's neutral staff
attack need not deal 20 damage. A 30/30/12/12 corpus succeeds. These source-derived
CRT endpoints have not yet been compared to a retail capture.

The output includes the input characters, bounded battle configuration, command
menus, action reports, complete CRT spans, final resources, saved enemy history,
base rewards, applied bonus rewards and drops using ItemDB labels. Base rewards
are 61 EXP, 60 JP and 750 leaves. First-round victory earns 66 JP; overkill and
damage-free bonuses depend on the actual actions. The three 100% drop checks
still draw CRT: one `ITM_CSM_0060` and two `ITM_CSM_0290`. No field reset or event
continuation is included.

All four Chapter 1 latent release flags must already be enabled. Damage charges
surviving players using the source binary32 expression and caps at 80/100/110/100
points. Point bookkeeping and reaching the cap consume no CRT. No latent-gain
support ability is modeled. The caller owns reward distribution and subsequent
character growth. `route_player(1, exp)` additionally provides Hikari's imported
original sword/lance/equipment and character-specific growth; Chapter 1 skip
initialization remains the caller's responsibility.

Only the specified commands are exposed. Grunts always die before an enemy
action, so their AI is an unreachable scaffold and the core rejects grunt turns.
The leader's ordinary, charged and low-HP attack definitions are imported. The
lance's marker jump is resolved for one unboosted hit, including its authored
accuracy check. English voice durations are scalar timing gates only; no audio,
graphics or Unreal runtime executes.

Regenerate the checked-in source projections with:

```text
node tools/import-hikari-recruitment.cjs
node tools/import-progression.cjs
```

The source audit at `data/source/hikari_recruitment.json` records the volume,
encounter/group, enemy and item rows, action schedules, all four voice sets,
latent-charge functions and source hashes. Run a standalone projection with:

```text
cargo run --example hikari_recruitment -- 0x0
cargo test --test hikari_recruitment
```
