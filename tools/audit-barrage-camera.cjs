'use strict';
const fs=require('node:fs');
const assert=require('node:assert/strict');
const {table}=require('./source.cjs');
const {auditHitTimer}=require('./audit-hit-timer.cjs');
const {blueprint}=require('./audit-pirro.cjs');
const ability=table('Ability/Database/AbilityData');
const barrage=ability.rows.find(r=>r.id==='ABI_SCH_060_04');
assert.equal(barrage.UseAbilityCamera,true);
const manager=blueprint('Battle/BP/BattleManagerBP',['PlayerOrder']);
const order=manager.functions.PlayerOrder;
assert(order.indexOf('LocalVirtualFunction BeginCharacterOrder')>=0);
assert(order.indexOf('LocalVirtualFunction BeginCharacterOrder')<order.indexOf('LocalVirtualFunction Open'));
const player=blueprint('Battle/BP/BattlePlayerBP',['BeginCharacterTurn_OLD']);
assert(player.functions.BeginCharacterTurn_OLD.includes('PlayBattleVoice'));
const result={
  ability:{source:ability.source,sha256:ability.sha256,id:barrage.id,UseAbilityCamera:barrage.UseAbilityCamera},
  ...auditHitTimer(),
  command_voice_order:{manager_sha256:manager.sha256,player_sha256:player.sha256,notes:'PlayerOrder calls BeginCharacterOrder before opening the command menu. BeginCharacterTurn_OLD requests type-3 PlayBattleVoice using JustBreakFlag. Immediate boost input cannot retroactively skip that selection; audible playback is separate.'},
  scope:'Max-boost Elemental Barrage: retain the separate new-break timer; suppress PlayFinishBlow timers while its ability camera is Playing. No change to the break or command voice selector.'
};
fs.writeFileSync('data/source/barrage-camera-gate.json',JSON.stringify(result,null,2)+'\n');
console.log('Verified Barrage ability-camera flag and shared PlayFinishBlow gate.');
