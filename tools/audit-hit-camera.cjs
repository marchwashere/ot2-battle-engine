'use strict';
// Verify camera inputs against cooked AbilityData and the actual Blueprint gates.
const fs=require('node:fs');
const assert=require('node:assert/strict');
const {table}=require('./source.cjs');
const {blueprint}=require('./audit-pirro.cjs');
const {auditHitTimer}=require('./audit-hit-timer.cjs');
const native=table('Ability/Database/AbilityData');
const rows=new Map(native.rows.map(r=>[r.id,r]));
const abilities=[];
for(const name of ['bg-effects','scholar-magic']) {
  const imported=JSON.parse(fs.readFileSync(`data/source/${name}.json`));
  for(const row of imported.abilities) {
    assert.equal(typeof row.UseAbilityCamera,'boolean',row.id);
    assert.equal(row.UseAbilityCamera,rows.get(row.id).UseAbilityCamera,row.id);
    abilities.push({id:row.id,use_ability_camera:row.UseAbilityCamera});
  }
}
const ailment=blueprint('Library/LibAilment',['Battle_ExecDamageHP']);
const body=ailment.functions.Battle_ExecDamageHP;
assert(body.includes('LocalVariable JustBreak'));
assert(body.includes('LocalVirtualFunction PlayBreakCamera'));
assert(body.includes('LocalVirtualFunction IsCharacterBreak'));
assert(body.includes('LocalVirtualFunction PlayFinishBlow'));
const manager=blueprint('Battle/BP/BattleManagerBP',['ExecuteUbergraph_BattleManagerBP']);
const managerBody=manager.functions.ExecuteUbergraph_BattleManagerBP;
assert(managerBody.includes('PushExecutionFlow target=33312'));
assert(managerBody.includes('LocalVirtualFunction StartBreakCamera'));
assert(/JumpIfNot target=33266\n[^\n]+LocalVariable CallFunc_IsPlaying_playing_1\n[^\n]+PopExecutionFlow/.test(managerBody));
assert(managerBody.includes('NameConst BREAK_CAMERA_SLOW'));
// BG's parent uses a specialized PreActionCommand rather than the base ability
// camera setup. Do not propagate its true flag onto its camera-free children.
const random=blueprint('Battle/BP/BattleAction/RandomSelectAbility/RandomSelectAbilityCommand',
  ['PreActionCommand','ExecuteUbergraph_RandomSelectAbilityCommand']);
assert(random.functions.PreActionCommand.includes('1546'));
assert(random.functions.ExecuteUbergraph_RandomSelectAbilityCommand.includes('NameConst INVADE_START_ACTION_DELAY'));
assert(!random.functions.ExecuteUbergraph_RandomSelectAbilityCommand.includes('StartAbilityCamera'));
for(const id of ['ABI_MER_070_D_1','ABI_MER_070_E_1']) assert.equal(rows.get(id).UseAbilityCamera,false);
fs.writeFileSync('data/source/hit-camera.json',JSON.stringify({
  ability_source:{source:native.source,sha256:native.sha256},abilities,
  damage_source:{asset:'Library/LibAilment',sha256:ailment.sha256,function:'Battle_ExecDamageHP'},
  break_camera:{sha256:manager.sha256,notes:'The Playing gate skips StartBreakCamera only; pushed continuation33312 still schedules BREAK_CAMERA_SLOW.'},
  bg_parent:{sha256:random.sha256,notes:'RandomSelectAbilityCommand overrides PreActionCommand; do not inherit the parent camera flag into child timelines.'},
  ...auditHitTimer(),
  scope:'Shared scalar camera gate for supported ordinary attacks, scripted skills/items and BG/route child commands. Ability cameras span one command through post-action; overlapping camera-enabled jobs are rejected. No graphics or fixed RNG offsets.'
},null,2)+'\n');
console.log(`Verified all ${abilities.length} imported command camera flags and Blueprint gates.`);
