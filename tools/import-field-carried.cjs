'use strict';
// Ordinary first-menu voices indexed by character identity, not fixed party slot.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const s=require('./source.cjs'),{blueprint}=require('./audit-pirro.cjs');
const sources=[];
function table(name){const t=s.table(name);sources.push({source:t.source,sha256:t.sha256});return t.rows;}
const voices=table('Character/Database/BattleVoiceSet');
const latent=table('Character/Database/PotentialityActionDB');
const bp=table('GameParam/BattleParamDefineTable').find(r=>r.id==='DEFAULT_BP_VALUE');
assert.equal(Number(bp.Init),1);assert.equal(Number(bp.Max),5);
const specs=[['throne','THIEF','THIEF'],['osvald','PROFESSOR','PROFESOR'],['agnea','DANCER','DANCER'],['partitio','MERCHANT','MERCHANT'],['hikari','FENCER','FENCER_LIGHT']];
const types={eBATTLE_START:1,eORDER_START:3,eTAKE_DAMAGE:4,eON_DEAD:5,eNORMAL_ATTACK:6,eNORMAL_ATTACK_LAST_HIT:7,eON_BOOST:8,eOPEN_WEAKNESS:9,eBREAK_ENEMY:10,eON_AVOID:11,eKILL_ENEMY:12};
const crt=JSON.parse(fs.readFileSync(path.join(__dirname,'../data/field_party_crt.json')));
const rawVoices=[];
crt.voices=specs.map(([,prefix])=>{
 const a=voices.findIndex(r=>r.id===prefix+'_VOICE_DATA_START'),b=voices.findIndex((r,i)=>i>a&&r.id.endsWith('_VOICE_DATA_START'));
 assert(a>=0&&b>a,prefix);
 const rows=voices.slice(a+1,b).filter(r=>types[r.BattleVoiceType.split('::').at(-1)]&&r.EnemyGroupID==='None'&&r.EventFlag==='None'&&!(r.BattleVoiceType.endsWith('::eBATTLE_START')&&(r.BossFlag||r.FieldCommandFlag))).sort((a,b)=>b.Priority-a.Priority);
 assert(rows.length>0&&rows.length<=64,prefix);rawVoices.push(rows);
 return rows.map(r=>({id:r.id,type:types[r.BattleVoiceType.split('::').at(-1)],rate:r.VoiceRate,once:r.PlayOnce,border:r.Border,self_only:r.TargetSelf,target:r.TargetID==='None'?null:r.TargetID,counter:r.CounterFlag,decondition:r.DiseaseCategory.endsWith('::eDECONDITION'),pinch:r.PinchFlag,just_break:r.JustBreakFlag}));
});
const latentRows=specs.map(([, ,label])=>latent.find(r=>r.id==='POTENTIALITY_ACTION_'+label));
assert(latentRows.every(Boolean));assert.equal(latentRows[4].ActionPointMax,125);
const players=Object.fromEntries(specs.map(([id],i)=>[id,{voice:i,latent_max:latentRows[i].ActionPointMax}]));
const waterGroups=table('Battle/Database/EnemyGroupData').filter(r=>/^ENG_FLD_SEA_1_3_WTR_DAY_/.test(r.id));
assert.equal(waterGroups.length,12);
const audits=[blueprint('Battle/BP/BattlePlayerBP',['Initialize','BeginCharacterTurn_OLD']),
 blueprint('Battle/BP/BattleCharacterComponent/BP_BattleVoiceComponent',['InitBattleVoice','PlayBattleVoice','CheckBattleVoiceData'])];
sources.push(...audits.map(a=>({source:'/Game/'+a.asset,sha256:a.sha256})));
const out=path.join(__dirname,'../data');
fs.writeFileSync(path.join(out,'field_carried_crt.json'),JSON.stringify(crt,null,2)+'\n');
fs.writeFileSync(path.join(out,'field_carried.json'),JSON.stringify({players,initial_bp:Number(bp.Init),groups:Object.fromEntries(waterGroups.map(g=>[g.id,g.EnemyID.filter(id=>id!=='None')]))},null,2)+'\n');
fs.writeFileSync(path.join(out,'source/field_carried.json'),JSON.stringify({sources,voiceRows:rawVoices,latentRows,bp,waterGroups,audits,scope:'Ordinary Normal/Preemptive first player menu, after field-selector timing draws. No player/enemy actions precede this menu. No BP-start supports or carried combat statuses. All character latent powers may be released; no latent change occurs.'},null,2)+'\n');
console.log('Imported carried field menu voices/caps:',players);
