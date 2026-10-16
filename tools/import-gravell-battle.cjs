'use strict';
// Build-time source import only. The runtime uses the checked-in compact data.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const s=require('./source.cjs'),{audit}=require('./audit-sequence.cjs');
const {objectHeader}=require('../../OT2BattleEngine/tools/properties.cjs');
const ability=s.table('Ability/Database/AbilityData'),invade=s.table('Ability/Database/InvadeData');
const find=id=>{const r=ability.rows.find(r=>r.id===id);assert(r,id);return r;};
const tiers=[1,2,3,4].map(n=>{
 const row=find('ABI_DAN_070_0'+n);
 const entries=row.Ailment.filter(a=>a.AilmentName!=='None').map(a=>({ability:a.AilmentName,weight:a.InvocationValue}));
 assert(entries.every(a=>Number.isInteger(a.weight)&&a.weight>=0));
 return {ability:row.id,count:row.BaseCount,total:entries.reduce((n,r)=>n+r.weight,0),entries};
});
assert.deepEqual(tiers.map(t=>t.total),[890,880,950,997]);
assert.deepEqual(tiers.map(t=>t.count),[1,2,3,4]);
const ids=[...new Set(tiers.flatMap(t=>t.entries.map(e=>e.ability)))];
const rows=ids.map(find);
const summon=invade.rows.filter(r=>/^PC_ABI_MER_YOHEI_04_0[1-4]_INVADE$/.test(r.id));
assert.equal(summon.length,4);
const guards=find('ABI_MER_YOHEI_04_01');
const sourceRows=[...rows,find('ABI_DAN_060_01'),...[1,2,3,4].map(n=>find('ABI_MER_050_0'+n)),find('ABI_INV_020_01'),find('ABI_MER_070_D_1'),find('ABI_SCH_010_01'),guards];
const sequences={};
for(const row of sourceRows){
 const asset=row.ActionCommandSequencer?.asset?.text?.split('.')[0]?.replace(/^\/Game\//,'');
 if(asset&&!sequences[asset])sequences[asset]=audit(asset);
}
const guardRow=find('ABI_MER_070_D_1');
const guardVariants=guardRow.Ailment.filter(r=>r.AilmentName!=='None').map(r=>({id:r.AilmentName,weight:r.InvocationValue}));
assert.equal(guardVariants.length,4);assert(guardVariants.every(v=>v.weight>0));
assert.equal(guardRow.CostType,'EABILITY_COST_TYPE::eMONEY');
fs.writeFileSync(path.join(__dirname,'../data/gravell_battle.json'),JSON.stringify({tiers,guard_variants:guardVariants,guards_cost:guardRow.CostValue},null,2)+'\n');
const crt=JSON.parse(fs.readFileSync(path.join(__dirname,'../data/carried_field_item_crt.json')));
crt.ability_voice_ids??={};
const voiceTable=s.table('Character/Database/BattleVoiceSet');
const voiceTypes={eBATTLE_START:1,eBATTLE_VICTORY:2,eORDER_START:3,eTAKE_DAMAGE:4,eON_DEAD:5,eNORMAL_ATTACK:6,eNORMAL_ATTACK_LAST_HIT:7,eON_BOOST:8,eOPEN_WEAKNESS:9,eBREAK_ENEMY:10,eON_AVOID:11,eKILL_ENEMY:12,eON_RECOVERED:13,eON_REVIVED:14,eUSE_ITEM_TO_PARTY:15,eUSE_ITEM_TO_ENEMY:16,ePLAY_GUARD:17,ePLAY_ABILITY:18};
const abilities=['ABI_DAN_060_01','ABI_DAN_070_01','ABI_INV_020_01','ABI_MER_070_01','ABI_SCH_010_01'];
const specs=[['THIEF','BattleTOU'],['PROFESSOR','BattleGAK'],['DANCER','BattleODO'],['MERCHANT','BattleSHO'],['FENCER','BattleKEN'],['PRIEST','BattleSIN']];
const durations=[];
// Galdera uses Osvald as Merchant: retain his Donate BP and Guards cast voices.
// His unused Dancer abilities are omitted from this route-specific 64-row voice mask.
crt.voices=specs.map(([prefix,folder],index)=>{
 const a=voiceTable.rows.findIndex(r=>r.id===prefix+'_VOICE_DATA_START'),b=voiceTable.rows.findIndex((r,j)=>j>a&&r.id.endsWith('_VOICE_DATA_START'));
 const rows=voiceTable.rows.slice(a+1,b).filter(r=>voiceTypes[r.BattleVoiceType.split('::').at(-1)]&&r.EnemyGroupID==='None'&&r.EventFlag==='None'&&!(r.BattleVoiceType.endsWith('::eBATTLE_START')&&(r.BossFlag||r.FieldCommandFlag))&&(!r.BattleVoiceType.endsWith('::ePLAY_ABILITY')||((abilities.includes(r.AbilityID)&&(r.AbilityID!=='ABI_SCH_010_01'||prefix==='PROFESSOR')&&!(prefix==='PROFESSOR'&&['ABI_DAN_060_01','ABI_DAN_070_01'].includes(r.AbilityID)))||(['MERCHANT','FENCER','PROFESSOR'].includes(prefix)&&['ABI_MER_070_D_1','ABI_MER_050_01'].includes(r.AbilityID))))).sort((a,b)=>b.Priority-a.Priority);
 assert(rows.length<=64,`${prefix}: ${rows.length} voice rows`);
 if(['MERCHANT','FENCER','PROFESSOR'].includes(prefix))for(const id of ['ABI_MER_070_D_1','ABI_MER_050_01'])assert(rows.some(r=>r.AbilityID===id),`${prefix}: missing ${id} voice`);
 durations[index]={};
 for(const r of rows.filter(r=>r.BattleVoiceType.endsWith('::ePLAY_ABILITY')||r.BattleVoiceType.endsWith('::eUSE_ITEM_TO_PARTY'))){
  const asset='Sound/Resource/VO/'+folder+'/en/'+r.VoiceLabel.toLowerCase(),pkg=s.assets.loadPackage(path.join(s.assets.CONTENT,asset+'.uasset'));
  durations[index][r.AbilityID==='None'?'party_item':r.AbilityID]=Math.ceil(s.fields(objectHeader(pkg,pkg.exports[0]).properties).Duration*24000);
 }
 return rows.map(r=>({id:r.id,type:voiceTypes[r.BattleVoiceType.split('::').at(-1)],rate:r.VoiceRate,once:r.PlayOnce,border:r.Border,self_only:r.TargetSelf,target:r.TargetID==='None'?null:r.TargetID,counter:r.CounterFlag,decondition:r.DiseaseCategory.endsWith('::eDECONDITION'),pinch:r.PinchFlag,just_break:r.JustBreakFlag,meat_only:r.MeatFlag,ability:r.AbilityID==='None'?null:r.AbilityID}));
});
const donate=[1,2,3,4].map(n=>find('ABI_MER_050_0'+n));
const skills=[
 {id:'donate_bp',name:'Donate BP',target:'other_ally',sp_cost:donate[0].CostValue,hit:{kind:'guaranteed'},effects:[{kind:'restore',resource:'bp',recovery:{kind:'fixed',amount:donate[0].Ailment[0].InvocationValue}}],boosted:donate.slice(1).map(row=>({effects:[{kind:'restore',resource:'bp',recovery:{kind:'fixed',amount:row.Ailment[0].InvocationValue}}],hits:1}))},
 ...[['bewildering_grace','Bewildering Grace','ABI_DAN_070_01'],['hired_help_guards','Hired Help: Guards','ABI_MER_070_01']].map(([id,name,source])=>({id,name,target:'self_only',sp_cost:id==='bewildering_grace'?find(source).CostValue:0,hit:{kind:'guaranteed'},effects:[{kind:'no_effect'}],boosted:[1,2,3].map(()=>({effects:[{kind:'no_effect'}],hits:1}))})),
 {id:'stimulate',name:'Stimulate',target:'other_ally',sp_cost:5,hit:{kind:'guaranteed'},effects:[{kind:'order_shift',positions:-1}]},
 {id:'springy_boots',name:'Springy Boots',target:'ally',sp_cost:0,hit:{kind:'guaranteed'},effects:[{kind:'status',status:{kind:'act_first',turns:5}}]}
];
const items=s.table('Item/Database/ItemDB');
for(const [id,row,voice] of [
 ['donate_bp',find('ABI_MER_050_01'),'ABI_MER_050_01'],
 ['stimulate',find('ABI_DAN_060_01'),'ABI_DAN_060_01'],
 ['springy_boots',find('ABI_INV_020_01'),'ABI_INV_020_01'],
 ['ABI_SET_SCH_010',find('ABI_SCH_010_01'),'ABI_SCH_010_01'],
 ['energizing_pomegranate',items.rows.find(r=>r.id==='ITM_CSM_0070'),'party_item'],
 ['energizing_pomegranate_m',items.rows.find(r=>r.id==='ITM_CSM_0080'),'party_item'],
 ['thunder_soulstone_l',items.rows.find(r=>r.id==='ITM_CSM_0370'),null]
]) {
 const asset=row.ActionCommandSequencer.asset.text.split('.')[0].replace(/^\/Game\//,'');
 const events=sequences[asset]??(sequences[asset]=audit(asset));
 crt.skills[id]=events.flatMap(e=>{
  if(/LocalVirtualFunction PlayVoice\b/.test(e.body))return [{kind:'voice',tick:e.time,type:Number(/ByteConst (\d+)/.exec(e.body)[1])}];
  if(/LocalVirtualFunction ExecAilment\b/.test(e.body)){const flags=[...e.body.matchAll(/\b(True|False)\b/g)].map(m=>m[1]==='True');return [{kind:'damage',tick:e.time,last:flags[1],effect:flags[0]?null:Number(/IntConst (\d+)/.exec(e.body)[1])}];}
  return [];
 });
 assert(crt.skills[id].some(e=>e.kind==='damage'),id);
 crt.skill_uses_ability_camera[id]=row.UseAbilityCamera??false;
 if(voice&&voice!=='party_item')crt.ability_voice_ids[id]=voice;
 for(let i=0;i<specs.length;i++)crt.skill_voice_duration_ticks[id+':'+i]=voice?(durations[i][voice]??0):crt.skill_voice_duration_ticks['shadow_soulstone_m:'+i];
}
fs.writeFileSync(path.join(__dirname,'../data/gravell_crt.json'),JSON.stringify(crt,null,2)+'\n');
fs.writeFileSync(path.join(__dirname,'../data/gravell_skills.json'),JSON.stringify(skills,null,2)+'\n');
fs.writeFileSync(path.join(__dirname,'../data/source/gravell_battle.json'),JSON.stringify({sources:[ability,invade].map(({source,sha256})=>({source,sha256})),abilities:sourceRows,summons:summon,sequences},null,2)+'\n');
console.log('Imported BG tiers, all child effects, Guards summons, and authored sequences.');
