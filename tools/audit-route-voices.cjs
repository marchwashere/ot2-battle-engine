'use strict';
// Source-data audit; callback behavior is covered separately by Rust regressions.
const fs=require('fs'),assert=require('assert/strict'),s=require('./source.cjs');
const source=s.table('Character/Database/BattleVoiceSet');
const byId=new Map(source.rows.map(r=>[r.id,r]));
const report={source:{path:source.source,sha256:source.sha256},profiles:[],contracts:[]};
for(const file of fs.readdirSync('data').filter(f=>f.endsWith('_crt.json'))){
 const data=JSON.parse(fs.readFileSync('data/'+file));
 let count=0;
 for(const rows of data.voices||[]){
  assert(rows.length<=64,file);
  assert.equal(new Set(rows.map(r=>r.id)).size,rows.length,file+' duplicate voice');
  for(const row of rows){
   const raw=byId.get(row.id);assert(raw,file+': '+row.id);
   for(const [a,b] of [['rate','VoiceRate'],['once','PlayOnce'],['border','Border'],['counter','CounterFlag'],['self_only','TargetSelf'],['pinch','PinchFlag'],['just_break','JustBreakFlag']])
    assert.equal(row[a],raw[b],`${file} ${row.id} ${a}`);
   assert.equal(row.decondition,raw.DiseaseCategory.endsWith('::eDECONDITION'));
   assert.equal(row.song??false,raw.SongFlag,`${file} ${row.id} song`);
   assert.equal(row.dance??false,raw.DanceFlag,`${file} ${row.id} dance`);
   count++;
  }
 }
 report.profiles.push({file,rows:count});
}
function contract(file,index,prefix,abilities=[],types=[]){
 const data=JSON.parse(fs.readFileSync('data/'+file+'.json'));
 const start=source.rows.findIndex(r=>r.id===prefix+'_VOICE_DATA_START');
 const end=source.rows.findIndex((r,i)=>i>start&&r.id.endsWith('_VOICE_DATA_START'));
 assert(start>=0&&end>start,prefix);
 const expected=source.rows.slice(start+1,end).filter(r=>r.EnemyGroupID==='None'&&r.EventFlag==='None'&&
  (abilities.includes(r.AbilityID)||types.includes(r.BattleVoiceType.split('::').at(-1))));
 for(const row of expected)assert(data.voices[index].some(r=>r.id===row.id),`${file}: missing ${row.id}`);
 for(const ability of abilities)assert(expected.some(r=>r.AbilityID===ability),`${prefix}: source voice absent for ${ability}`);
 report.contracts.push({file,index,prefix,abilities,types,checked:expected.map(r=>r.id)});
}
for(const [i,prefix] of ['THIEF','PROFESSOR','DANCER','MERCHANT','FENCER','PRIEST'].entries()){
 contract('carried_field_item_crt',i,prefix,[],['ePLAY_GUARD','eUSE_ITEM_TO_PARTY','eUSE_ITEM_TO_ENEMY']);
 contract('gravell_crt',i,prefix,[],['ePLAY_GUARD','eON_RECOVERED','eON_REVIVED','eTAKE_DAMAGE','eON_AVOID','eON_DEAD']);
}
for(const [i,prefix] of [[1,'PROFESSOR'],[3,'MERCHANT'],[4,'FENCER']])contract('gravell_crt',i,prefix,['ABI_MER_050_01','ABI_MER_070_D_1']);
for(const [i,prefix] of [[2,'DANCER'],[5,'PRIEST']])contract('gravell_crt',i,prefix,['ABI_DAN_070_01']);
contract('gravell_crt',0,'THIEF',['ABI_INV_020_01']);
contract('gravell_crt',1,'PROFESSOR',['ABI_SCH_010_01']);
contract('lord_sands_crt',0,'THIEF',['ABI_INV_030_01']);
contract('lord_sands_crt',1,'PROFESSOR',['ABI_WAR_010_01','ABI_SCH_060_01']);
contract('lord_sands_crt',3,'MERCHANT',['ABI_MER_070_E_1']);
contract('lord_sands_crt',5,'PRIEST',['ABI_CLE_020_01']);
contract('cape_cold_mug_crt',0,'PROFESSOR',['ABI_SCH_010_01','ABI_SCH_020_01']);
contract('field_party_crt',1,'PROFESSOR',['ABI_SCH_010_01','ABI_SCH_020_01']);
contract('field_party_crt',0,'THIEF',['ABI_THI_020_01']);
contract('throne_pirro_crt',0,'THIEF',['ABI_THI_020_01']);
// BG children normally have no additional player cast line. Verify against
// source rather than assuming that a zero-draw child voice is intentional.
report.bg_child_voices=[];
const bg=JSON.parse(fs.readFileSync('data/source/bg-effects.json'));
for(const prefix of ['DANCER','PRIEST'])for(const ability of bg.abilities.filter(r=>/^ABI_DAN_MAKA_\d+$/.test(r.id))){
 const rows=source.rows.filter(r=>r.id.startsWith(prefix+'_')&&r.AbilityID===ability.id);
 for(const row of rows)assert(JSON.parse(fs.readFileSync('data/gravell_crt.json')).voices[prefix==='DANCER'?2:5].some(r=>r.id===row.id),`missing BG child voice ${row.id}`);
 report.bg_child_voices.push({prefix,ability:ability.id,rows:rows.map(r=>r.id)});
}
fs.writeFileSync('data/source/route-voice-audit.json',JSON.stringify(report,null,2)+'\n');
console.log(`Audited ${report.profiles.length} profiles, ${report.profiles.reduce((n,p)=>n+p.rows,0)} imported rows, ${report.contracts.length} route contracts.`);
