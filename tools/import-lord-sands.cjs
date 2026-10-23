'use strict';
const fs=require('fs'),path=require('path'),assert=require('assert/strict'),s=require('./source.cjs');
const {audit}=require('./audit-sequence.cjs'),{blueprint}=require('./audit-pirro.cjs'),{initialization}=require('./enemy-initialization.cjs'),{objectHeader}=require('../../OT2BattleEngine/tools/properties.cjs');
const tables={};const table=n=>(tables[n]??=s.table(n)).rows;
const abilities=table('Ability/Database/AbilityData'),enemy=table('Character/Database/EnemyDB').find(r=>r.id==='ENE_NML_FLD_DST_280'),group=table('Battle/Database/EnemyGroupData').find(r=>r.id==='ENG_DNG_DST_2_2_L_110');
assert.equal(enemy.Param.HP,40000);assert.equal(group.BossBattle,true);
const config=JSON.parse(fs.readFileSync('data/temenos_recruitment.json'));const e=structuredClone(config.enemies[0]),p=enemy.Param;
e.id=enemy.id;e.name='Lord of the Sands';e.stats={max_hp:p.HP,max_sp:p.MP,speed:p.AGI};e.resources={hp:p.HP,sp:p.MP,bp:enemy.FirstBP,latent:0};
Object.assign(e.combat,{attack:p.ATK,defense:p.DEF,magic:p.MATK,magic_defense:p.MDEF,accuracy:p.ACC,evasion:p.EVA,critical:p.CON,growth_attack:Math.round((enemy.DamageRatio-.5)*128),growth_magic:Math.round((enemy.DamageRatio-.5)*128),shields:p.SP,enemy_bp_base:p.BP,enemy_initialization:initialization(enemy),weapon_weaknesses:enemy.WeaponResist.flatMap((v,i)=>v.endsWith('::eWEAK')?[['sword','lance','dagger','axe','bow','staff'][i]]:[]),element_weaknesses:enemy.AttributeResist.flatMap((v,i)=>v.endsWith('::eWEAK')?[['none','fire','ice','lightning','wind','light','dark'][i]]:[])});
config.enemies=[e];config.night_buff_turns=3;
for(const [id,name,cost]of [['aggressive_slash','Aggressive Slash',9],['critical_scope','Critical Scope',0],['hired_help_beastlings','Hired Help: Beastlings',0]])config.skills.push({id,name,target:'self_only',sp_cost:cost,hit:{kind:'guaranteed'},effects:[{kind:'no_effect'}],boosted:[1,2,3].map(()=>({effects:[{kind:'no_effect'}],hits:1}))});
const bg=JSON.parse(fs.readFileSync('data/source/bg-effects.json'));
const ids=['ABI_WAR_010_03','ABI_INV_030_01','ABI_MER_YOHEI_05_01'];
for(const id of ids){
 const r=abilities.find(r=>r.id===id);bg.abilities=bg.abilities.filter(a=>a.id!==id);bg.abilities.push(r);
 const asset=r.ActionCommandSequencer.asset.text.replace('/Game/','').split('.')[0];const events=audit(asset);bg.sequences[id]=events;
 let unfolded=events;
 if(id==='ABI_WAR_010_03'){
  const pkg=s.assets.loadPackage(path.join(s.assets.CONTENT,asset+'.uasset'));const movie=s.fields(objectHeader(pkg,pkg.exports.find(e=>e.name==='MovieScene_0')).properties);const marks=new Map(movie.MarkedFrames.map(m=>[m.Label,m.FrameNumber.value]));
  let counter=0,time=0,pos=0;unfolded=[];
  for(let i=0;i<events.length;i++){const ev=events[i];time+=ev.time-pos;pos=ev.time;if(/LocalVirtualFunction ExecStepCounter\b/.test(ev.body))counter++;
   if(/LocalVirtualFunction CheckCounterToMarker\b/.test(ev.body)&&counter>=5){pos=marks.get('Finish');assert(pos!==undefined);i=events.findIndex(e=>e.time>=pos)-1;continue;}
   unfolded.push({...ev,time});
  }
  assert.equal(unfolded.filter(e=>/LocalVirtualFunction ExecAilment\b/.test(e.body)).length,5);
 }
 bg.steps[id]=unfolded.flatMap(ev=>{const b=ev.body;if(/LocalVirtualFunction ExecPlayDiseaseMissUI\b/.test(b))return [{kind:'disease_miss_ui',tick:ev.time}];if(/LocalVirtualFunction ExecStepCounter\b/.test(b))return [{kind:'random_target',tick:ev.time}];if(/LocalVirtualFunction PlayVoice\b/.test(b))return [{kind:'voice',tick:ev.time,voice_type:Number(/ByteConst (\d+)/.exec(b)[1])}];if(/LocalVirtualFunction ExecAilment\b/.test(b)){const flags=[...b.matchAll(/\b(True|False)\b/g)].map(m=>m[1]==='True');return [{kind:'effects',tick:ev.time,index:flags[0]?null:Number(/IntConst (\d+)/.exec(b)[1]),last:flags[1]}]};return []});
}
const used=new Set(bg.abilities.flatMap(r=>r.Ailment.map(a=>a.AilmentName)));bg.ailments=table('Ability/Database/AilmentData').filter(r=>used.has(r.id));
const summons=table('Ability/Database/InvadeData').filter(r=>/^PC_ABI_MER_YOHEI_05_0[1-4]_INVADE$/.test(r.id));assert.equal(summons.length,4);
for(const r of summons){const resource=table('Character/Database/CharacterResourceDB').find(x=>x.id===r.ResourceLabel);const voiceRows=table('Character/Database/BattleVoiceSet').filter(v=>v.id.startsWith(resource.BattleVoiceCharacterID+'_')&&!v.id.endsWith('VOICE_DATA_START'));assert(voiceRows.every(v=>v.BattleVoiceType==='EBATTLE_VOICE_TYPE::ePLAY_ABILITY'&&v.VoiceRate===100&&v.AbilityID==='ABI_MER_YOHEI_05_01'));bg.summon_profiles=bg.summon_profiles.filter(x=>x.id!==r.id);bg.summon_profiles.push({id:r.id,parameter:r.Parameter,weapon:r.WeaponType,ability_voice:voiceRows.length===1,...initialization(r,null,2)});}
const crt=JSON.parse(fs.readFileSync('data/temenos_recruitment_crt.json'));
// Scope, Slash and Beastlings use their own authored voices; boss start excludes ordinary-only rows.
const vt=table('Character/Database/BattleVoiceSet'),types={eBATTLE_START:1,eBATTLE_VICTORY:2,eORDER_START:3,eTAKE_DAMAGE:4,eON_DEAD:5,eNORMAL_ATTACK:6,eNORMAL_ATTACK_LAST_HIT:7,eON_BOOST:8,eOPEN_WEAKNESS:9,eBREAK_ENEMY:10,eON_AVOID:11,eKILL_ENEMY:12,eON_RECOVERED:13,eON_REVIVED:14,eUSE_ITEM_TO_PARTY:15,eUSE_ITEM_TO_ENEMY:16,ePLAY_GUARD:17,ePLAY_ABILITY:18};
crt.voices=['THIEF','PROFESSOR','DANCER','MERCHANT','FENCER','PRIEST'].map(prefix=>{const start=vt.findIndex(r=>r.id===prefix+'_VOICE_DATA_START'),end=vt.findIndex((r,i)=>i>start&&r.id.endsWith('_VOICE_DATA_START'));const rows=vt.slice(start+1,end).filter(r=>types[r.BattleVoiceType.split('::').at(-1)]&&r.EnemyGroupID==='None'&&r.EventFlag==='None'&&(!r.BattleVoiceType.endsWith('::eBATTLE_START')||!r.FieldCommandFlag)&&(!r.BattleVoiceType.endsWith('::ePLAY_ABILITY')||({THIEF:['ABI_INV_030_01'],PROFESSOR:['ABI_WAR_010_01'],MERCHANT:['ABI_MER_070_01','ABI_MER_070_E_1']}[prefix]||[]).includes(r.AbilityID))).sort((a,b)=>b.Priority-a.Priority);assert(rows.length<=64);return rows.map(r=>({id:r.id,type:types[r.BattleVoiceType.split('::').at(-1)],rate:r.VoiceRate,once:r.PlayOnce,border:r.Border,self_only:r.TargetSelf,target:r.TargetID==='None'?null:r.TargetID,counter:r.CounterFlag,decondition:r.DiseaseCategory.endsWith('::eDECONDITION'),pinch:r.PinchFlag,just_break:r.JustBreakFlag,...(r.SongFlag?{song:true}:{}),...(r.DanceFlag?{dance:true}:{}),meat_only:r.MeatFlag,ability:r.AbilityID==='None'?null:r.AbilityID}));});
const parent=abilities.find(r=>r.id==='ABI_MER_070_E_1');const seq=audit(parent.ActionCommandSequencer.asset.text.replace('/Game/','').split('.')[0]);const times=seq.filter(e=>/LocalVirtualFunction ExecSummonAbility\b/.test(e.body)).map(e=>e.time);assert.deepEqual(times,[53280,64778,76778,88778]);
const data={enemy,group,summons,beastlings_cost:parent.CostValue,beastlings_variants:parent.Ailment.filter(a=>a.AilmentName!=='None').map(a=>({id:a.AilmentName,weight:a.InvocationValue})),summon_ticks:times,reward:{exp:enemy.Exp,jp:enemy.JobPoint,money:enemy.Money},drop_item:enemy.HaveItemID,drop_rate:enemy.DropProbability};
for(const [file,obj]of [['lord_sands.json',config],['lord_sands_crt.json',crt],['lord_sands_data.json',data],['source/bg-effects.json',bg]])fs.writeFileSync('data/'+file,JSON.stringify(obj,null,2)+'\n');
fs.writeFileSync('data/source/lord-sands.json',JSON.stringify({sources:Object.values(tables).map(({source,sha256})=>({source,sha256})),enemy,group,parent_sequence:seq,random_count:blueprint('Battle/BP/BattleAction/RandomCountAttack/RandomCountAttackCommand'),scope:abilities.find(r=>r.id==='ABI_INV_030_01'),night_params:table('GameParam/BattleParamDefineTable').filter(r=>/BUFF_IN_NIGHT_PARAM/.test(r.id))},null,2)+'\n');
console.log('Imported Lord of the Sands, five-hit Slash, Critical Scope and four Beastlings');
