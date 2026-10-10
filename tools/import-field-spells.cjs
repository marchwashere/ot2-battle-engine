// Reuse the source-audited Mug Fireball/Icewind callback timelines for field spells.
// Preserve field voice order, appending missing guard and supported ability rows.
const fs=require('node:fs'),path=require('node:path');
const root=path.join(__dirname,'../data');
const mug=JSON.parse(fs.readFileSync(path.join(root,'cape_cold_mug_crt.json')));
for(const file of ['field_party_crt.json','field_break_crt.json']) {
 const target=path.join(root,file),data=JSON.parse(fs.readFileSync(target));
 for(const key of ['skills','skill_voice_duration_ticks','skill_uses_ability_camera','ability_voice_ids']) {
  data[key]??={};
  for(const id of ['ABI_SET_SCH_010','ABI_SET_SCH_020']) data[key][id]=mug[key][id];
 }
 const thief=JSON.parse(fs.readFileSync(path.join(root,'throne_pirro_crt.json'))).voices[0];
 for(const [actor,rows] of [[0,thief],...(file==='field_party_crt.json'?[[1,mug.voices[0]]]:[])]) {
  for(const row of rows.filter(r=>r.type===17 || (r.type===18 && (actor===0?['ABI_THI_020_01']:['ABI_SCH_010_01','ABI_SCH_020_01']).includes(r.ability)))) {
   if(!data.voices[actor].some(r=>r.id===row.id)) data.voices[actor].push(row);
  }
 }
 fs.writeFileSync(target,JSON.stringify(data));
}
