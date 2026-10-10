'use strict';
// Exact scalar callbacks for the supplied Ice Soulstone -> normal attack route.
// Grunts die before any enemy action. Their AI is deliberately not exposed.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const s=require('./source.cjs'),{audit}=require('./audit-sequence.cjs');
const {initialization}=require('./enemy-initialization.cjs');
const {objectHeader}=require('../../OT2BattleEngine/tools/properties.cjs');
const sources=[];
function table(name){const t=s.table(name);sources.push({source:t.source,sha256:t.sha256});return new Map(t.rows.map(r=>[r.id,r]));}
const groups=table('Battle/Database/EnemyGroupData'),enemies=table('Character/Database/EnemyDB');
const abilities=table('Ability/Database/AbilityData'),items=table('Item/Database/ItemDB');
const voices=[...table('Character/Database/BattleVoiceSet').values()];
const volume=table('Battle/Database/EncountVolumeData').get('ENM_EVE_WAR_C00_010');
const encounter=table('Battle/Database/BattleEncounterData').get('ENC_EVE_WAR_C00_010');
assert.deepEqual(volume.EncounterList.filter(r=>r.EncounterName!=='None').map(r=>[r.ProgressBorder,r.EncounterName]),[[0,encounter.id]]);
assert.equal(encounter.Group[0].GroupID,'ENG_EVE_WAR_C00_010');
const latentTable=table('Character/Database/PotentialityActionDB');
const latentRows=['THIEF','PROFESOR','DANCER','MERCHANT'].map(id=>latentTable.get('POTENTIALITY_ACTION_'+id));
const latentParam=table('GameParam/BattleParamDefineTable').get('POTENTIALITY_ACTION_POINT_BASE');
assert.equal(latentParam.Param,'1.3');
const {blueprint}=require('./audit-pirro.cjs');
const latentAudit=[blueprint('Library/LibBattle',['CalcPotentialityActionPoint']),
 blueprint('Battle/BP/BattlePlayerBP',['SetDamageForPlayer','AddPotentialityActionPoint','UpdatePotentinalityActionUI']),
 blueprint('Battle/BP/BattleCharacterComponent/BP_PotentialityActionComponent',['AddPotentialityActionPoint'])];
sources.push(...latentAudit.map(({asset,sha256})=>({source:'/Game/'+asset,sha256})));
const group=groups.get('ENG_EVE_WAR_C00_010');
assert.deepEqual(group.EnemyID.filter(x=>x!=='None'),['ENE_EVE_WAR_DST_090','ENE_EVE_WAR_DST_080','ENE_EVE_WAR_DST_080']);
assert.equal(group.TalkPlayerID,'EPlayableCharacterID::eNONE');assert.equal(group.BattleStartEvent.asset.text,'None');assert.equal(group.BossBattle,false);
const weaponNames=['sword','lance','dagger','axe','bow','staff'],elementNames=['none','fire','ice','lightning','wind','light','dark'];
const rawText=fs.readFileSync(path.join(s.assets.CONTENT,'GameText/Database/GameTextEN.uexp'));
function en(key){const b=Buffer.from(key+'_Text\0'),i=rawText.indexOf(b);assert(i>=0,key);const j=i+b.length,n=rawText.readInt32LE(j);return n>0?rawText.toString('utf8',j+4,j+4+n-1):rawText.toString('utf16le',j+4,j+4+(-n-1)*2);}
const config={schema_version:1,rules:'ot2_opening_v1',seed:0,night_buff_turns:0,players:[],enemies:[],skills:[],inventory:{ice_soulstone:1}};
const enemyRows=group.EnemyID.filter(x=>x!=='None').map(id=>enemies.get(id));
for(const [index,e] of enemyRows.entries()){
 const p=e.Param,action=i=>({skill:e.EnemyAbility[i],target:'random_opponent'});
 assert.equal(e.NPCFlag,false);assert.equal(e.DeadEffectSlow,false);assert.equal(e.FirstBP,0);
 const ai=index===0?{kind:'normal_enemy',attack:action(1),charged:action(0),low_hp:action(7)}
   :{kind:'sequence',actions:[action(1)]}; // unreachable scaffold; public adapter forbids grunt turns
 config.enemies.push({id:e.id,name:en(e.DisplayNameID),stats:{max_hp:p.HP,max_sp:p.MP,speed:p.AGI},resources:{hp:p.HP,sp:p.MP,bp:e.FirstBP,latent:0},
 skills:index===0?[...new Set(e.EnemyAbility.filter(x=>x!=='None'))]:[e.EnemyAbility[1]],
 combat:{attack:p.ATK,defense:p.DEF,magic:p.MATK,magic_defense:p.MDEF,accuracy:p.ACC,evasion:p.EVA,critical:p.CON,growth_attack:Math.round((e.DamageRatio-.5)*128),growth_magic:Math.round((e.DamageRatio-.5)*128),weapons:[],main_weapon:null,shields:p.SP,weapon_weaknesses:e.WeaponResist.flatMap((r,i)=>r.endsWith('::eWEAK')?[weaponNames[i]]:[]),element_weaknesses:e.AttributeResist.flatMap((r,i)=>r.endsWith('::eWEAK')?[elementNames[i]]:[]),statuses:[],steal_item:null,enemy_bp_base:p.BP,enemy_initialization:initialization(e)},
 ai:{schema_version:1,initial_state:'normal',states:[{id:'normal',rules:[{condition:{kind:'always'},selection:ai}]}]}});
}
const crt=JSON.parse(fs.readFileSync(path.join(__dirname,'../data/opening_crt.json')));
crt.skills={};crt.normal_attacks={};crt.skill_uses_ability_camera={};crt.skill_voice_duration_ticks={};crt.ability_voice_ids={};
crt.player_latent_maxima=latentRows.map(r=>r.ActionPointMax);
crt.battle_start_voice=true;crt.battle_start_speaker=null;crt.victory_voice=group.UseVictoryAction;crt.enemy_death_slow=false;
crt.drops=enemyRows.map(e=>({rate:e.DropProbability,item:e.HaveItemID}));
const specs=[['attack_dagger','NORMAL_ATTACK_DAGGER_1','dagger'],['attack_staff','NORMAL_ATTACK_ROD_1','staff'],['attack_lance','NORMAL_ATTACK_SPEAR_1','lance'],...['ABI_ENE_0010','ABI_ENE_0180','ABI_ENE_0190'].map(id=>[id,id,null])];
const schedules=[],rows=[];
function schedule(id,r,normal=false){
 const asset=r.ActionCommandSequencer.asset.text.replace('/Game/','').split('.')[0],events=audit(asset),steps=[];
 const pkg=s.assets.loadPackage(path.join(s.assets.CONTENT,asset+'.uasset'));
 const movie=s.fields(objectHeader(pkg,pkg.exports.find(e=>e.name==='MovieScene_0')).properties);
 const marks=new Map((movie.MarkedFrames||[]).map(m=>[m.Label,m.FrameNumber.value]));
 let counter=0,clock=0,position=0;
 for(let i=0,guard=0;i<events.length;i++){
  assert(++guard<200,'Unexpected loop '+asset);
  const e=events[i],fn=/LocalVirtualFunction (\S+)/.exec(e.body)?.[1];
  clock+=e.time-position;position=e.time;
  if(fn==='ExecStepCounter'){counter++;steps.push({kind:'check',tick:clock});}
  if(fn==='PlayVoice')steps.push({kind:'voice',tick:clock,type:Number(/ByteConst (\d+)/.exec(e.body)[1])});
  if(fn==='ExecAilment'){const bs=[...e.body.matchAll(/\b(True|False)\b/g)].map(m=>m[1]==='True');assert.equal(bs.length,2);steps.push({kind:'damage',tick:clock,last:bs[1],effect:bs[0]?null:Number(/IntConst (\d+)/.exec(e.body)[1])});}
  if(fn==='JumpToMarker'||fn==='CheckCounterToMarker'&&counter>=1){
   assert(normal,'Unexpected non-normal jump');
   const label=JSON.parse(/(?:UnicodeStringConst|StringConst) (".*")/.exec(e.body)[1]),dest=marks.get(label);
   assert(dest!==undefined,'Unknown marker '+label);position=dest;
   const next=events.findIndex(e=>e.time>=dest);if(next<0)break;i=next-1;
  }
 }
 if(normal)assert.equal(steps.filter(e=>e.kind==='damage').length,1);
 assert(steps.some(e=>e.kind==='damage'),id);assert(steps.filter(e=>e.kind==='damage').at(-1).last,id);
 (normal?crt.normal_attacks:crt.skills)[id]=steps;
 schedules.push({id,asset,events});sources.push({source:'/Game/'+asset,sha256:s.assets.hash(s.assets.loadPackage(path.join(s.assets.CONTENT,asset+'.uasset')).b)});
 return steps;
}
for(const [id,label,weapon] of specs){
 const r=abilities.get(label);rows.push(r);const effects=r.Ailment.filter(a=>a.AilmentName!=='None').map(a=>{assert.equal(a.AilmentName,'PHYSICS_DAMAGE');return{kind:'damage',attack:{kind:'physical',weapon,element:null,invocation:a.InvocationValue,ability_ratio:r.AbilityRatio,hit_bonus:r.HitRatio,critical_bonus:r.CriticalRatio,critical_power:r.CriticalPower,normal_attack:!!weapon}};});
 const steps=schedule(id,r,!!weapon);
 config.skills.push({id,name:en(r.DisplayName),target:'opponent',sp_cost:r.CostValue,hit:{kind:'guaranteed'},hits:steps.filter(e=>e.kind==='damage').length,effects,boosted:[]});
 crt.skill_uses_ability_camera[id]=r.UseAbilityCamera;crt.ability_voice_ids[id]=r.id;
}
const ice=items.get('ITM_CSM_0320');assert.equal(ice.Ailment[0].InvocationValue,200);schedule('ice_soulstone',ice);
crt.skill_uses_ability_camera.ice_soulstone=require('./audit-hit-timer.cjs').auditHitTimer().item_uses_ability_camera;
const types={eBATTLE_START:1,eBATTLE_VICTORY:2,eORDER_START:3,eTAKE_DAMAGE:4,eON_DEAD:5,eNORMAL_ATTACK:6,eNORMAL_ATTACK_LAST_HIT:7,eON_BOOST:8,eOPEN_WEAKNESS:9,eBREAK_ENEMY:10,eON_AVOID:11,eKILL_ENEMY:12,eUSE_ITEM_TO_PARTY:15,eUSE_ITEM_TO_ENEMY:16};
const voiceRows=[];
crt.voices=[['THIEF','BattleTOU','attack_dagger'],['PROFESSOR','BattleGAK','attack_staff'],['DANCER','BattleODO','attack_dagger'],['MERCHANT','BattleSHO','attack_lance']].map(([prefix,folder,attack],actor)=>{
 const start=voices.findIndex(r=>r.id===prefix+'_VOICE_DATA_START'),end=voices.findIndex((r,i)=>i>start&&r.id.endsWith('_VOICE_DATA_START'));
 assert(start>=0,prefix);
 const rows=voices.slice(start+1,end<0?undefined:end).filter(r=>types[r.BattleVoiceType.split('::').at(-1)]&&(r.EnemyGroupID==='None'||r.EnemyGroupID===group.id)&&r.EventFlag==='None'&&!(r.BattleVoiceType.endsWith('::eBATTLE_START')&&(r.BossFlag||r.FieldCommandFlag))).sort((a,b)=>b.Priority-a.Priority);
 assert(rows.length<=64);voiceRows.push(rows);
 for(const [id,type] of [['ice_soulstone','eUSE_ITEM_TO_ENEMY'],[attack,'eNORMAL_ATTACK']]){
  const v=rows.find(v=>v.BattleVoiceType.endsWith('::'+type));assert(v,prefix+type);
  const asset='Sound/Resource/VO/'+folder+'/en/'+v.VoiceLabel.toLowerCase(),pkg=s.assets.loadPackage(path.join(s.assets.CONTENT,asset+'.uasset'));
  const cue=s.fields(objectHeader(pkg,pkg.exports[0]).properties);assert(cue.Duration>0);
  crt.skill_voice_duration_ticks[id+':'+actor]=Math.ceil(cue.Duration*24000);sources.push({source:'/Game/'+asset,sha256:s.assets.hash(pkg.b)});
 }
 return rows.map(r=>({id:r.id,type:types[r.BattleVoiceType.split('::').at(-1)],rate:r.VoiceRate,once:r.PlayOnce,border:r.Border,self_only:r.TargetSelf,target:r.TargetID==='None'?null:r.TargetID,counter:r.CounterFlag,decondition:r.DiseaseCategory.endsWith('::eDECONDITION'),pinch:r.PinchFlag,just_break:r.JustBreakFlag}));
});
const out=path.join(__dirname,'../data');
fs.writeFileSync(path.join(out,'hikari_recruitment.json'),JSON.stringify(config,null,2)+'\n');
fs.writeFileSync(path.join(out,'hikari_recruitment_crt.json'),JSON.stringify(crt,null,2)+'\n');
fs.writeFileSync(path.join(out,'source/hikari_recruitment.json'),JSON.stringify({sources,volume,encounter,group,enemyRows,ice,abilities:rows,voiceRows,schedules,latentRows,latentParam,latentAudit,bounded_scope:'Grunts always die to first player Ice Soulstone; their AI is unreachable and rejected. Only leader AI is simulated. All four chapter-one latent release flags are required; no latent-gain support equipped.'},null,2)+'\n');
console.log('Imported Hikari recruitment:',config.enemies.map(e=>e.name),crt.skill_voice_duration_ticks);
