'use strict';
const fs=require('fs'),path=require('path'),assert=require('assert/strict'),s=require('./source.cjs');
const {audit}=require('./audit-sequence.cjs'),{objectHeader}=require('../../OT2BattleEngine/tools/properties.cjs');
const a=s.table('Ability/Database/AbilityData'), ail=s.table('Ability/Database/AilmentData'),sets=s.table('Ability/Database/AbilitySetData'),d=s.table('Ability/Database/DiseaseData'),v=s.table('Character/Database/BattleVoiceSet');
const abilities=a.rows.filter(r=>/^ABI_SCH_(011|070)_0[1-4]$/.test(r.id)),sequences={},steps={};
for(const r of abilities){const events=audit(r.ActionCommandSequencer.asset.text.replace('/Game/','').split('.')[0]);sequences[r.id]=events;steps[r.id]=events.flatMap(e=>{if(/LocalVirtualFunction PlayVoice\b/.test(e.body))return [{kind:'voice',tick:e.time,voice_type:Number(/ByteConst (\d+)/.exec(e.body)[1])}];if(/LocalVirtualFunction ExecAilment\b/.test(e.body)){const f=[...e.body.matchAll(/\b(True|False)\b/g)].map(m=>m[1]==='True');return [{kind:'effects',tick:e.time,index:f[0]?null:Number(/IntConst (\d+)/.exec(e.body)[1]),last:f[1]}];}return [];});assert.equal(steps[r.id].filter(e=>e.kind==='effects').length,r.id.includes('_011_')?2:1);}
const used=new Set(abilities.flatMap(r=>r.Ailment.map(a=>a.AilmentName)));const voices={},durations={};
for(const [id,prefix,folder]of [['throne','THIEF','BattleTOU'],['osvald','PROFESSOR','BattleGAK'],['agnea','DANCER','BattleODO'],['partitio','MERCHANT','BattleSHO'],['hikari','FENCER','BattleKEN'],['temenos','PRIEST','BattleSIN']]){
 voices[id]=v.rows.filter(r=>r.id.startsWith(prefix+'_')&&r.BattleVoiceType==='EBATTLE_VOICE_TYPE::ePLAY_ABILITY'&&['ABI_SCH_011_01','ABI_SCH_070_01'].includes(r.AbilityID)).sort((a,b)=>b.Priority-a.Priority);
 assert.equal(voices[id].length,2);for(const r of voices[id]){assert.equal(r.PlayOnce,false);assert.equal(r.VoiceRate,100);assert.equal(r.Border,0);assert.equal(r.EnemyGroupID,'None');assert.equal(r.EventFlag,'None');const pkg=s.assets.loadPackage(path.join(s.assets.CONTENT,'Sound/Resource/VO/'+folder+'/en/'+r.VoiceLabel.toLowerCase()+'.uasset'));const n=Math.ceil(s.fields(objectHeader(pkg,pkg.exports[0]).properties).Duration*24000);durations[id+':'+r.AbilityID]=n;if(r.AbilityID==='ABI_SCH_011_01')assert(n+steps[r.AbilityID][0].tick<steps[r.AbilityID].find(e=>e.kind==='effects').tick);}
}
const evidence={};
for(const [file,names]of [['Battle__BP__BattleAction___Base__BP_ActionCommandBase.txt',['ExecPayPostCost']],['Battle__BP__BattleCommandMenuActorBP.txt',['CreateDefaultMenuItemFromActionSet']],['Battle__BP__BattleCharacterBaseBP.txt',['UpdateDiseaseCounter']],['Battle__BP__BattlePlayerBP.txt',['OnTakeDisease']]]) {
 const text=fs.readFileSync(path.join(__dirname,'../../tools/speedrun_audit',file),'utf8');
 evidence[file]=Object.fromEntries(names.map(name=>{const start=text.indexOf('# '+name+' script=');assert(start>=0,name);const end=text.indexOf('\n# ',start+2);return [name,text.slice(start,end<0?text.length:end)];}));
}
const data={evidence,sources:[a,ail,sets,d,v].map(({source,sha256})=>({source,sha256})),abilities,ailments:ail.rows.filter(r=>used.has(r.id)),sequences,steps,voices,durations,sets:sets.rows.filter(r=>['ABI_SET_SCH_010','ABI_SET_SCH_011','ABI_SET_SCH_070'].includes(r.id)),disease:d.rows.find(r=>r.id==='SUPER_MAGIC')};
fs.writeFileSync(path.join(__dirname,'../data/source/scholar-magic.json'),JSON.stringify(data,null,2)+'\n');
console.log('Imported Advanced Magic and Fire Storm: all BP tiers, timelines, use-count disease and six caster voice profiles.');

