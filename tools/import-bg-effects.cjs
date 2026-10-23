'use strict';
// Preserve the entire reachable child graph, not just the parent lottery rows.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const s=require('./source.cjs'),{audit}=require('./audit-sequence.cjs'),{blueprint}=require('./audit-pirro.cjs');
const ability=s.table('Ability/Database/AbilityData'), ailments=s.table('Ability/Database/AilmentData'), invade=s.table('Ability/Database/InvadeData');
const tiers=JSON.parse(fs.readFileSync(path.join(__dirname,'../data/gravell_battle.json'))).tiers;
const roots=tiers[3].entries.map(e=>e.ability);assert.equal(new Set(roots).size,39);
const summons=invade.rows.filter(r=>/^PC_ABI_MAKA_00[12]_INVADE$/.test(r.id));
assert.equal(summons.length,2);
const ids=[...new Set([...roots,...summons.flatMap(s=>s.AbilityList.filter(id=>id!=='None')),'ABI_MER_YOHEI_04_01'])];
const rows=ids.map(id=>{const r=ability.rows.find(r=>r.id===id);assert(r,id);return r;});
const used=new Set(rows.flatMap(r=>r.Ailment.map(a=>a.AilmentName)));
const aRows=ailments.rows.filter(r=>used.has(r.id));
const enemies=s.table('Character/Database/EnemyDB').rows.filter(r=>rows.find(a=>a.id==='ABI_DAN_MAKA_003').Ailment.some(a=>a.AilmentName===r.id));
assert.equal(enemies.length,6);
const sequences={};
const steps={};
for(const r of rows) {
 const asset=r.ActionCommandSequencer.asset.text.split('.')[0].replace(/^\/Game\//,'');
 sequences[r.id]=audit(asset);
 steps[r.id]=sequences[r.id].flatMap(e=>{
  if(/LocalVirtualFunction ExecPlayDiseaseMissUI\b/.test(e.body))return [{kind:'disease_miss_ui',tick:e.time}];
  if(/LocalVirtualFunction ExecStepCounter\b/.test(e.body))return [{kind:'random_target',tick:e.time}];
  if(r.id==='ABI_MER_YOHEI_04_01'&&/LocalVirtualFunction SequenceEvent_0\b/.test(e.body)) {
   const director=blueprint(asset);assert(/ExecAilmentWithAbilityHit_Implementation/.test(director.functions.ExecuteUbergraph_SequenceDirector));
   const flags=[...e.body.matchAll(/\b(True|False)\b/g)].map(m=>m[1]==='True');
   const index=/IntConst (\d+)/.exec(e.body);assert(index&&flags.length===3);
   return [{kind:'effects',tick:e.time,index:flags[0]?null:Number(index[1]),last:flags[1],on_hit:flags[2]}];
  }
  if(/LocalVirtualFunction ExecAilment\b/.test(e.body)) {
   const flags=[...e.body.matchAll(/\b(True|False)\b/g)].map(m=>m[1]==='True');
   const index=/IntConst (\d+)/.exec(e.body);assert(index&&flags.length===2,r.id);
   return [{kind:'effects',tick:e.time,index:flags[0]?null:Number(index[1]),last:flags[1]}];
  }
  if(/LocalVirtualFunction PlayVoice\b/.test(e.body)) {
   const type=/ByteConst (\d+)/.exec(e.body);assert(type,r.id);
   return [{kind:'voice',tick:e.time,voice_type:Number(type[1])}];
  }
  return [];
 });
}
const commands={};
for(const asset of ['Battle/BP/BattleAction/RandomSelectAbility/RandomSelectAbilityCommand','Battle/BP/BattleAction/TransformEnemy/TransformEnemyCommand','Battle/BP/BattleAction/SummonAbility/SummonAbilityCommand','Battle/BP/BattleAction/RandomCountAttack/RandomCountAttackCommand']) commands[asset]=blueprint(asset);
const diseases=s.table('Ability/Database/DiseaseData').rows.filter(r=>aRows.some(a=>a.AddDisease.some(d=>d.DiseaseName===r.id)));
const recovery_ui=blueprint('UserInterface/Battle/Effect/BP/BattleDamageNumber',['PlayRecoveryHP','PlayRecoveryMP','PlayRecoveryBP','DisplayTimer']);
const params=s.table('GameParam/BattleParamDefineTable');
const resources=s.table('Character/Database/CharacterResourceDB'),voice=s.table('Character/Database/BattleVoiceSet');
const summonProfiles=[...summons,...invade.rows.filter(r=>/^PC_ABI_MER_YOHEI_04_0[1-4]_INVADE$/.test(r.id))].map(r=>{
 const resource=resources.rows.find(x=>x.id===r.ResourceLabel);assert(resource,r.id);
 const voiceRows=resource.BattleVoiceCharacterID==='None'?[]:voice.rows.filter(v=>v.id.startsWith(resource.BattleVoiceCharacterID+'_')&&!v.id.endsWith('VOICE_DATA_START'));
 assert(voiceRows.every(v=>v.BattleVoiceType==='EBATTLE_VOICE_TYPE::ePLAY_ABILITY'&&v.VoiceRate===100&&v.AbilityID==='ABI_MER_YOHEI_04_01'));
 return {id:r.id,parameter:r.Parameter,weapon:r.WeaponType,ability_voice:voiceRows.length===1,
  ...require('./enemy-initialization.cjs').initialization(r,null,2)};
});
const transform_resists=Object.fromEntries(s.table('Character/Database/EnemyDB').rows.map(r=>[r.id,r.DiseaseResist[8]!=='EATTRIBUTE_RESIST::eNONE']));
const out={transform_resists,sources:[ability,ailments,invade,params,resources,voice].map(({source,sha256})=>({source,sha256})),abilities:rows,ailments:aRows,diseases,summons,summon_profiles:summonProfiles,enemies,sequences,steps,commands,recovery_ui,params:params.rows.filter(r=>['DISEASE_POISON_DAMAGE','DISEASE_PARANETER_RATIO','DISEASE_TURN_COUNT_MAX'].includes(r.id))};
fs.writeFileSync(path.join(__dirname,'../data/source/bg-effects.json'),JSON.stringify(out,null,2)+'\n');
console.log(`Imported ${roots.length} BG outcomes, ${summons.length} summon children, ${enemies.length} transformations, ${aRows.length} effect handlers.`);
