'use strict';
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const s=require('./source.cjs'),{audit}=require('./audit-sequence.cjs');
const {objectHeader}=require('../../OT2BattleEngine/tools/properties.cjs');
const out=path.join(__dirname,'../data');
const enemyRows=s.table('Character/Database/EnemyDB'),items=s.table('Item/Database/ItemDB'),voiceTable=s.table('Character/Database/BattleVoiceSet');
const item=items.rows.find(r=>r.id==='ITM_CSM_0450');
assert.equal(item.Ailment[0].InvocationValue,1100);
const crt=JSON.parse(fs.readFileSync(path.join(out,'field_party_crt.json')));
const asset=item.ActionCommandSequencer.asset.text.split('.')[0].slice(6),events=audit(asset);
crt.skills.shadow_soulstone_m=events.flatMap(e=>{
 if(/LocalVirtualFunction PlayVoice\b/.test(e.body))return [{kind:'voice',tick:e.time,type:Number(/ByteConst (\d+)/.exec(e.body)[1])}];
 if(/LocalVirtualFunction ExecAilment\b/.test(e.body)){const all=[...e.body.matchAll(/\b(True|False)\b/g)].map(m=>m[1]==='True');return [{kind:'damage',tick:e.time,last:all[1],effect:all[0]?null:Number(/IntConst (\d+)/.exec(e.body)[1])}];}
 return [];
});
assert(crt.skills.shadow_soulstone_m.some(e=>e.kind==='damage'));
crt.skill_uses_ability_camera.shadow_soulstone_m=require('./audit-hit-timer.cjs').auditHitTimer().item_uses_ability_camera;
const types={eBATTLE_START:1,eBATTLE_VICTORY:2,eORDER_START:3,eTAKE_DAMAGE:4,eON_DEAD:5,eNORMAL_ATTACK:6,eNORMAL_ATTACK_LAST_HIT:7,eON_BOOST:8,eOPEN_WEAKNESS:9,eBREAK_ENEMY:10,eON_AVOID:11,eKILL_ENEMY:12,eUSE_ITEM_TO_PARTY:15,eUSE_ITEM_TO_ENEMY:16};
for(const [actor,prefix,folder] of [[0,'THIEF','BattleTOU'],[1,'PROFESSOR','BattleGAK']]){
 const start=voiceTable.rows.findIndex(r=>r.id===prefix+'_VOICE_DATA_START');
 const end=voiceTable.rows.findIndex((r,i)=>i>start&&r.id.endsWith('_VOICE_DATA_START'));
 const rows=voiceTable.rows.slice(start+1,end).filter(r=>types[r.BattleVoiceType.split('::').at(-1)]&&r.EnemyGroupID==='None'&&r.EventFlag==='None'&&!(r.BattleVoiceType.endsWith('::eBATTLE_START')&&(r.BossFlag||r.FieldCommandFlag))).sort((a,b)=>b.Priority-a.Priority);
 assert(rows.length<=64);
 crt.voices[actor]=rows.map(r=>({id:r.id,type:types[r.BattleVoiceType.split('::').at(-1)],rate:r.VoiceRate,once:r.PlayOnce,border:r.Border,self_only:r.TargetSelf,target:r.TargetID==='None'?null:r.TargetID,counter:r.CounterFlag,decondition:r.DiseaseCategory.endsWith('::eDECONDITION'),pinch:r.PinchFlag,just_break:r.JustBreakFlag,...(r.SongFlag?{song:true}:{}),...(r.DanceFlag?{dance:true}:{})}));
 const v=rows.find(r=>r.id===prefix+'_eUSE_ITEM_TO_ENEMY_COMMON');assert(v,'Missing item voice');
 const pkg=s.assets.loadPackage(path.join(s.assets.CONTENT,'Sound/Resource/VO/'+folder+'/en/'+v.VoiceLabel.toLowerCase()+'.uasset'));
 const cue=s.fields(objectHeader(pkg,pkg.exports[0]).properties);
 crt.skill_voice_duration_ticks[actor===0?'shadow_soulstone_m':'shadow_soulstone_m_osvald']=Math.ceil(cue.Duration*24000);
}
const ids=Object.keys(JSON.parse(fs.readFileSync(path.join(out,'field_escape.json'))).enemies);
const rewards=Object.fromEntries(ids.map(id=>{const e=enemyRows.rows.find(e=>e.id===id);return [id,{exp:e.Exp,jp:e.JobPoint,money:e.Money,drop_item:e.HaveItemID,drop_rate:e.DropProbability}];}));
fs.writeFileSync(path.join(out,'field_rare_crt.json'),JSON.stringify(crt,null,2)+'\n');
fs.writeFileSync(path.join(out,'field_rare_rewards.json'),JSON.stringify(rewards,null,2)+'\n');
fs.writeFileSync(path.join(out,'source/field_rare.json'),JSON.stringify({sources:[enemyRows,items,voiceTable].map(({source,sha256})=>({source,sha256})),item,asset,events},null,2)+'\n');
