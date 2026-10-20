'use strict';
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const s=require('./source.cjs'),{objectHeader}=require('../../OT2BattleEngine/tools/properties.cjs');
const table=s.table('Character/Database/BattleVoiceSet');
const crt=JSON.parse(fs.readFileSync(path.join(__dirname,'../data/field_rare_crt.json')));
const types={eBATTLE_START:1,eBATTLE_VICTORY:2,eORDER_START:3,eTAKE_DAMAGE:4,eON_DEAD:5,eNORMAL_ATTACK:6,eNORMAL_ATTACK_LAST_HIT:7,eON_BOOST:8,eOPEN_WEAKNESS:9,eBREAK_ENEMY:10,eON_AVOID:11,eKILL_ENEMY:12,eUSE_ITEM_TO_PARTY:15,eUSE_ITEM_TO_ENEMY:16,ePLAY_GUARD:17};
const specs=[['THIEF','BattleTOU'],['PROFESSOR','BattleGAK'],['DANCER','BattleODO'],['MERCHANT','BattleSHO'],['FENCER','BattleKEN'],['PRIEST','BattleSIN']];
const sources=[{source:table.source,sha256:table.sha256}],voiceRows=[];
crt.voices=specs.map(([prefix,folder],i)=>{
 const a=table.rows.findIndex(r=>r.id===prefix+'_VOICE_DATA_START'),b=table.rows.findIndex((r,j)=>j>a&&r.id.endsWith('_VOICE_DATA_START'));
 assert(a>=0&&b>a);
 const rows=table.rows.slice(a+1,b).filter(r=>types[r.BattleVoiceType.split('::').at(-1)]&&r.EnemyGroupID==='None'&&r.EventFlag==='None'&&!(r.BattleVoiceType.endsWith('::eBATTLE_START')&&(r.BossFlag||r.FieldCommandFlag))).sort((a,b)=>b.Priority-a.Priority);
 assert(rows.length<=64);voiceRows.push(rows);
 const v=rows.find(r=>r.id===prefix+'_eUSE_ITEM_TO_ENEMY_COMMON');assert(v);
 const asset='Sound/Resource/VO/'+folder+'/en/'+v.VoiceLabel.toLowerCase();
 const pkg=s.assets.loadPackage(path.join(s.assets.CONTENT,asset+'.uasset'));
 crt.skill_voice_duration_ticks['shadow_soulstone_m:'+i]=Math.ceil(s.fields(objectHeader(pkg,pkg.exports[0]).properties).Duration*24000);
 sources.push({source:'/Game/'+asset,sha256:s.assets.hash(pkg.b)});
 return rows.map(r=>({id:r.id,type:types[r.BattleVoiceType.split('::').at(-1)],rate:r.VoiceRate,once:r.PlayOnce,border:r.Border,self_only:r.TargetSelf,target:r.TargetID==='None'?null:r.TargetID,counter:r.CounterFlag,decondition:r.DiseaseCategory.endsWith('::eDECONDITION'),pinch:r.PinchFlag,just_break:r.JustBreakFlag,...(r.SongFlag?{song:true}:{}),...(r.DanceFlag?{dance:true}:{})}));
});
// Wind Soulstone L uses the same ordinary enemy-item voice, its own action sequence.
const wind=s.table('Item/Database/ItemDB').rows.find(r=>r.id==='ITM_CSM_0400');
assert.equal(wind.Ailment[0].InvocationValue,2700);
const windAsset=wind.ActionCommandSequencer.asset.text.split('.')[0].slice(6);
const windEvents=require('./audit-sequence.cjs').audit(windAsset);
crt.skills.wind_soulstone_l=windEvents.flatMap(e=>{
 if(/LocalVirtualFunction PlayVoice\b/.test(e.body))return [{kind:'voice',tick:e.time,type:Number(/ByteConst (\d+)/.exec(e.body)[1])}];
 if(/LocalVirtualFunction ExecAilment\b/.test(e.body)){const flags=[...e.body.matchAll(/\b(True|False)\b/g)].map(m=>m[1]==='True');return [{kind:'damage',tick:e.time,last:flags[1],effect:flags[0]?null:Number(/IntConst (\d+)/.exec(e.body)[1])}];}
 return [];
});
assert(crt.skills.wind_soulstone_l.some(e=>e.kind==='damage'));
crt.skill_uses_ability_camera.wind_soulstone_l=crt.skill_uses_ability_camera.shadow_soulstone_m;
for(let i=0;i<specs.length;i++)crt.skill_voice_duration_ticks['wind_soulstone_l:'+i]=crt.skill_voice_duration_ticks['shadow_soulstone_m:'+i];
fs.writeFileSync(path.join(__dirname,'../data/source/wind-soulstone-l.json'),JSON.stringify({item:wind,asset:windAsset,events:windEvents},null,2)+'\n');
crt.player_latent_maxima=[80,100,110,100,125,110];
fs.writeFileSync(path.join(__dirname,'../data/carried_field_item_crt.json'),JSON.stringify(crt,null,2)+'\n');
fs.writeFileSync(path.join(__dirname,'../data/source/carried_field_item.json'),JSON.stringify({sources,voiceRows,scope:'One first-menu Shadow Soulstone M with actual ordered party, ordinary field voices and source item sequence.'},null,2)+'\n');
