'use strict';
const fs=require('fs'),path=require('path'),assert=require('assert/strict'),s=require('./source.cjs');
const {audit}=require('./audit-sequence.cjs'),{initialization}=require('./enemy-initialization.cjs'),{objectHeader}=require('../../OT2BattleEngine/tools/properties.cjs');
const table=n=>s.table(n).rows;
const enemy=table('Character/Database/EnemyDB').find(r=>r.id==='ENE_NML_FLD_CTY_260');
assert.equal(enemy.Param.HP,50000);
const config=JSON.parse(fs.readFileSync('data/lord_sands.json')),e=config.enemies[0],p=enemy.Param;
const sword=JSON.parse(fs.readFileSync('data/field_break.json')).config.skills.find(s=>s.id==='attack_sword');
config.skills.push(sword);
e.id=enemy.id;e.name='Gigantes';e.stats={max_hp:p.HP,max_sp:p.MP,speed:p.AGI};e.resources={hp:p.HP,sp:p.MP,bp:enemy.FirstBP,latent:0};
Object.assign(e.combat,{attack:p.ATK,defense:p.DEF,magic:p.MATK,magic_defense:p.MDEF,accuracy:p.ACC,evasion:p.EVA,critical:p.CON,growth_attack:Math.round((enemy.DamageRatio-.5)*128),growth_magic:Math.round((enemy.DamageRatio-.5)*128),shields:p.SP,enemy_bp_base:p.BP,enemy_initialization:initialization(enemy),weapon_weaknesses:enemy.WeaponResist.flatMap((v,i)=>v.endsWith('::eWEAK')?[['sword','lance','dagger','axe','bow','staff'][i]]:[]),element_weaknesses:enemy.AttributeResist.flatMap((v,i)=>v.endsWith('::eWEAK')?[['none','fire','ice','lightning','wind','light','dark'][i]]:[])});
const abilities=table('Ability/Database/AbilityData');
const bg=JSON.parse(fs.readFileSync('data/source/bg-effects.json'));
for(const [id,skill]of [['ABI_SCH_060_04','elemental_barrage'],['ABI_WAR_010_04','aggressive_slash'],['ABI_CLE_020_01','holy_light']]){
 const r=abilities.find(r=>r.id===id);assert(r,id);
 if(!config.skills.some(x=>x.id===skill))config.skills.push({id:skill,name:skill,target:'self_only',sp_cost:r.CostValue,hit:{kind:'guaranteed'},effects:[{kind:'no_effect'}],boosted:[1,2,3].map(()=>({effects:[{kind:'no_effect'}],hits:1}))});
 bg.abilities=bg.abilities.filter(a=>a.id!==id);bg.abilities.push(r);
 const asset=r.ActionCommandSequencer.asset.text.replace('/Game/','').split('.')[0],events=audit(asset);
 bg.sequences[id]=events;
 const pkg=s.assets.loadPackage(path.join(s.assets.CONTENT,asset+'.uasset'));const movie=s.fields(objectHeader(pkg,pkg.exports.find(e=>e.name==='MovieScene_0')).properties);
 const marks=new Map((movie.MarkedFrames||[]).map(m=>[m.Label,m.FrameNumber.value]));
 const marker=b=>{const name=/StringConst "([^"]+)"/.exec(b)?.[1];assert(marks.has(name),name);return marks.get(name);};
 bg.steps[id]=events.flatMap(ev=>{let b=ev.body,tick=ev.time;
  if(/LocalVirtualFunction ExecStepCounter\b/.test(b))return [{kind:'random_target',tick}];
  if(/LocalVirtualFunction PlayVoice\b/.test(b))return [{kind:'voice',tick,voice_type:Number(/ByteConst (\d+)/.exec(b)[1])}];
  if(/LocalVirtualFunction CheckCounterToMarker\b/.test(b))return [{kind:'counter_branch',tick,destination:marker(b)}];
  if(/LocalVirtualFunction CheckPattern3 ToMarker\b/.test(b)){let names=[...b.matchAll(/StringConst "([^"]+)"/g)].map(m=>m[1]);assert.equal(names.length,2);return [{kind:'pattern_branch',tick,ice:marks.get(names[0]),thunder:marks.get(names[1])}];}
  if(/LocalVirtualFunction JumpToMarker\b/.test(b))return [{kind:'jump',tick,destination:marker(b)}];
  if(/LocalVirtualFunction ExecAilment\b/.test(b)){let flags=[...b.matchAll(/\b(True|False)\b/g)].map(m=>m[1]==='True');return [{kind:'effects',tick,index:flags[0]?null:Number(/IntConst (\d+)/.exec(b)[1]),last:flags[1]}];}
  return [];
 });
 fs.writeFileSync('data/source/'+skill+'-timeline.json',JSON.stringify({ability:r,markers:Object.fromEntries(marks),events},null,2)+'\n');
}
const used=new Set(bg.abilities.flatMap(r=>r.Ailment.map(a=>a.AilmentName)));bg.ailments=table('Ability/Database/AilmentData').filter(r=>used.has(r.id));
fs.writeFileSync('data/source/bg-effects.json',JSON.stringify(bg,null,2)+'\n');
fs.writeFileSync('data/gigantes.json',JSON.stringify(config,null,2)+'\n');
fs.writeFileSync('data/gigantes_data.json',JSON.stringify({enemy,reward:{exp:enemy.Exp,jp:enemy.JobPoint,money:enemy.Money},drop_item:enemy.HaveItemID,drop_rate:enemy.DropProbability},null,2)+'\n');
// The common boss profile needs these ability voices for the new routines.
const crt=JSON.parse(fs.readFileSync('data/lord_sands_crt.json')),voices=table('Character/Database/BattleVoiceSet');
for(const [i,prefix,ability]of [[1,'PROFESSOR','ABI_SCH_060_01'],[5,'PRIEST','ABI_CLE_020_01']]){
 for(const r of voices.filter(r=>r.id.startsWith(prefix+'_')&&r.AbilityID===ability)){
  assert.equal(r.BattleVoiceType,'EBATTLE_VOICE_TYPE::ePLAY_ABILITY');
  if(!crt.voices[i].some(v=>v.id===r.id))crt.voices[i].push({id:r.id,type:18,rate:r.VoiceRate,once:r.PlayOnce,border:r.Border,self_only:r.TargetSelf,target:r.TargetID==='None'?null:r.TargetID,counter:r.CounterFlag,decondition:r.DiseaseCategory.endsWith('::eDECONDITION'),pinch:r.PinchFlag,just_break:r.JustBreakFlag,meat_only:r.MeatFlag,ability});
 }
 assert(crt.voices[i].length<=64);
}
fs.writeFileSync('data/lord_sands_crt.json',JSON.stringify(crt,null,2)+'\n');
console.log('Imported Gigantes and authored Barrage/Slash/Holy Light timelines');


