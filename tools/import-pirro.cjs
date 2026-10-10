'use strict';
const {initialization}=require('./enemy-initialization.cjs');
// Reproducible scalar import. No presentation runtime is needed by the core.
const fs=require('fs'),path=require('path'),assert=require('node:assert/strict');
const s=require('./source.cjs'),{audit}=require('./audit-sequence.cjs'),{blueprint}=require('./audit-pirro.cjs');
const {objectHeader}=require('../../OT2BattleEngine/tools/properties.cjs');
const out=path.join(__dirname,'../data'),sources=[];
fs.writeFileSync(path.join(out,'source/pirro-boost-gate.json'),JSON.stringify(require('./audit-pirro-boost.cjs').auditPirroBoost(),null,2)+'\n');
fs.writeFileSync(path.join(out,'source/pirro-latent-unlock.json'),JSON.stringify(require('./audit-pirro-latent.cjs').auditPirroLatent(),null,2)+'\n');
fs.writeFileSync(path.join(out,'source/pirro-mercy.json'),JSON.stringify(require('./audit-pirro-mercy.cjs').auditPirroMercy(),null,2)+'\n');
function table(a){const t=s.table(a);sources.push({source:t.source,sha256:t.sha256});return t.rows;}
const enemies=table('Character/Database/EnemyDB'),abilities=table('Ability/Database/AbilityData');
const enemy=enemies.find(r=>r.id==='ENE_BOS_THI_C01_010'),p=enemy.Param;
const group=table('Battle/Database/EnemyGroupData').find(r=>r.id==='ENG_BOS_THI_C01_010');
const text=fs.readFileSync(path.join(s.assets.CONTENT,'GameText/Database/GameTextEN.uexp'));
function english(key){const needle=Buffer.from(key+'_Text\0'),at=text.indexOf(needle);assert(at>=0,key);const x=at+needle.length,n=text.readInt32LE(x);return n>0?text.toString('utf8',x+4,x+3+n):text.toString('utf16le',x+4,x+4+(-n-1)*2);}
const config=JSON.parse(fs.readFileSync(path.join(out,'throne_second.json')));
config.players=config.players.slice(0,1);config.night_buff_turns=3;
config.players[0].skills=['attack_sword','attack_dagger','ABI_SET_THI_020'];
config.skills=config.skills.filter(r=>config.players[0].skills.includes(r.id));
config.inventory={healing_grape:1,shadow_soulstone:1,light_soulstone:1,wind_soulstone:1,ice_soulstone:1};
const weapon=['sword','lance','dagger','axe','bow','staff'],element=['none','fire','ice','lightning','wind','light','dark'];
const ids=enemy.EnemyAbility.filter(id=>id!=='None');
config.enemies=[{id:'pirro_boss',name:english(enemy.DisplayNameID),stats:{max_hp:p.HP,max_sp:p.MP,speed:p.AGI},resources:{hp:p.HP,sp:p.MP,bp:enemy.FirstBP,latent:0},skills:ids,
  combat:{attack:p.ATK,defense:p.DEF,magic:p.MATK,magic_defense:p.MDEF,accuracy:p.ACC,evasion:p.EVA,critical:p.CON,growth_attack:p.ATK,growth_magic:p.MATK,
  weapons:[],main_weapon:null,shields:p.SP,weapon_weaknesses:enemy.WeaponResist.flatMap((r,i)=>r.endsWith('::eWEAK')?[weapon[i]]:[]),element_weaknesses:enemy.AttributeResist.flatMap((r,i)=>r.endsWith('::eWEAK')?[element[i]]:[]),statuses:[],steal_item:'light_soulstone',enemy_bp_base:p.BP,enemy_initialization:initialization(enemy)},
  ai:{schema_version:1,initial_state:'pirro',states:[{id:'pirro',rules:[{condition:{kind:'always'},selection:{kind:'sequence',actions:[{skill:ids[3],target:'random_opponent'}]}}]}]}}];
function effect(a,r){switch(a.AilmentName){
  case 'None':return {kind:'no_effect'};
  case 'PHYSICS_DAMAGE':case 'MAGIC_DAMAGE':return {kind:'damage',attack:{kind:a.AilmentName==='MAGIC_DAMAGE'?'magical':'physical',weapon:null,element:r.Attribute==='EATTRIBUTE_TYPE::eNONE'?null:r.Attribute.split('::e')[1].toLowerCase(),invocation:a.InvocationValue,ability_ratio:r.AbilityRatio,hit_bonus:r.HitRatio,critical_bonus:r.CriticalRatio,critical_power:r.CriticalPower,normal_attack:false}};
  case 'ADD_ATK_UP':case 'ADD_AGI_UP':return {kind:'status',status:{kind:a.AilmentName==='ADD_ATK_UP'?'attack_up':'speed_up',turns:255}};
  case 'HP_RATIO_DAMAGE':return {kind:'unarmed_hp_ratio_damage',percent:a.InvocationValue,hit_bonus:r.HitRatio};
  case 'MP_RATIO_DAMAGE':return {kind:'sp_ratio_damage',percent:a.InvocationValue};
  case 'ADD_THIS_ATTACK_HIT_FAILED':return {kind:'force_miss'};
  case 'ADD_SIGN_OF_BOOST':return {kind:'sign_of_boost',enabled:true};
  case 'REMOVE_SIGN_OF_BOOST':return {kind:'sign_of_boost',enabled:false};
  default:throw Error(a.AilmentName);
}}
const crt=JSON.parse(fs.readFileSync(path.join(out,'throne_second_crt.json'))),schedules=[];
const latent=table('Character/Database/PotentialityActionDB').find(r=>r.id==='POTENTIALITY_ACTION_THIEF');
assert(latent.ActionPointInit<latent.ActionPointMax);
crt.latent={initial:latent.ActionPointInit,maximum:latent.ActionPointMax};
crt.skills=Object.fromEntries(Object.entries(crt.skills).filter(([id])=>id==='ABI_SET_THI_020'));
crt.skill_uses_ability_camera=Object.fromEntries(Object.entries(crt.skill_uses_ability_camera).filter(([id])=>id==='ABI_SET_THI_020'));
crt.ability_voice_ids={'ABI_SET_THI_020':'ABI_THI_020_01'};
function schedule(id,asset){const events=audit(asset),steps=[];for(const e of events){
 if(/LocalVirtualFunction PlayVoice\b/.test(e.body))steps.push({kind:'voice',tick:e.time,type:Number(/ByteConst (\d+)/.exec(e.body)[1])});
 if(/LocalVirtualFunction ExecAilment\b/.test(e.body)){const all=[...e.body.matchAll(/\b(True|False)\b/g)].map(m=>m[1]==='True');steps.push({kind:'damage',tick:e.time,last:all[1],effect:all[0]?null:Number(/IntConst (\d+)/.exec(e.body)[1])});}
 } crt.skills[id]=steps;schedules.push({id,asset,events});return steps;}
for(const id of ids){const r=abilities.find(r=>r.id===id),effects=r.Ailment.filter((a,i)=>i===0||a.AilmentName!=='None').map(a=>effect(a,r));const asset=r.ActionCommandSequencer.asset.text.split('.')[0].slice(6),steps=schedule(id,asset);
 assert.equal(typeof r.UseAbilityCamera,'boolean');crt.skill_uses_ability_camera[id]=r.UseAbilityCamera;
 config.skills.push({id,name:english(r.DisplayName),target:r.TargetType==='EBATTLE_TARGET_TYPE::eSELF'?'self_only':r.TargetType==='EBATTLE_TARGET_TYPE::eENEMY_ALL'?'all_opponents':'opponent',sp_cost:r.CostValue,hit:{kind:'guaranteed'},effects,hits:Math.max(1,steps.filter(s=>s.kind==='damage').length),boosted:[]});crt.ability_voice_ids[id]=id;
}
const items=table('Item/Database/ItemDB');
const hitTimerGates=require('./audit-hit-timer.cjs').auditHitTimer();
fs.writeFileSync(path.join(out,'source/hit-timer-gates.json'),JSON.stringify(hitTimerGates,null,2)+'\n');
const catalog=JSON.parse(fs.readFileSync(path.join(out,'items.json')));
for(const [num,id,elem,name] of [['0320','ice_soulstone','ice','Ice Soulstone'],['0380','wind_soulstone','wind','Wind Soulstone'],['0410','light_soulstone','light','Light Soulstone'],['0440','shadow_soulstone','dark','Shadow Soulstone']]){
 const r=items.find(r=>r.id==='ITM_CSM_'+num);assert.equal(r.Ailment[0].AilmentName,'HP_FIXED_DAMAGE');assert.equal(r.TargetType,'EBATTLE_TARGET_TYPE::eENEMY_ALL');
 catalog.items=catalog.items.filter(r=>r.id!==id);catalog.items.push({id,name,source_label:r.id,target:'all_opponents',effects:[{kind:'elemental_fixed_damage',amount:r.Ailment[0].InvocationValue,element:elem}]});schedule(id,r.ActionCommandSequencer.asset.text.split('.')[0].slice(6));
}
schedule('healing_grape',items.find(r=>r.id==='ITM_CSM_0010').ActionCommandSequencer.asset.text.split('.')[0].slice(6));
const voices=table('Character/Database/BattleVoiceSet'),types={eBATTLE_START:1,eBATTLE_VICTORY:2,eORDER_START:3,eTAKE_DAMAGE:4,eON_DEAD:5,eNORMAL_ATTACK:6,eNORMAL_ATTACK_LAST_HIT:7,eON_BOOST:8,eOPEN_WEAKNESS:9,eBREAK_ENEMY:10,eON_AVOID:11,eKILL_ENEMY:12,eON_RECOVERED:13,eON_REVIVED:14,eUSE_ITEM_TO_PARTY:15,eUSE_ITEM_TO_ENEMY:16,ePLAY_GUARD:17,ePLAY_ABILITY:18,eON_POTENTIALITY_ACTION:19};
const rawVoices=[];
crt.voices=['THIEF','BOSS_TOU10A_PIRLO','BOSS_TOU10B_PIRLO'].map(owner=>{
 const start=voices.findIndex(r=>r.id===owner+'_VOICE_DATA_START'),end=voices.findIndex((r,i)=>i>start&&r.id.endsWith('_VOICE_DATA_START'));
 const rows=voices.slice(start+1,end<0?voices.length:end).filter(r=>types[r.BattleVoiceType.split('::')[1]]&&(r.EnemyGroupID==='None'||r.EnemyGroupID===group.id)&&(r.AbilityID==='None'||Object.values(crt.ability_voice_ids).includes(r.AbilityID)));
 rows.sort((a,b)=>b.Priority-a.Priority);assert(rows.length<=64);assert(rows.every(r=>r.EventFlag==='None'));rawVoices.push(...rows);
 return rows.map(r=>({id:r.id,type:types[r.BattleVoiceType.split('::')[1]],rate:r.VoiceRate,once:r.PlayOnce,border:r.Border,self_only:r.TargetSelf,target:r.TargetID==='None'?null:r.TargetID,counter:r.CounterFlag,decondition:r.DiseaseCategory!=='EDISEASE_CATEGORY::eDISEASE_CATEGORY_MAX',pinch:r.PinchFlag,just_break:r.JustBreakFlag,ability:r.AbilityID==='None'?null:r.AbilityID}));
});
crt.victory_voice=group.UseVictoryAction;crt.battle_start_voice=group.BattleStartEvent.asset.text==='None';crt.drops=[{rate:enemy.DropProbability,item:'inspiriting_plum_basket'}];
const deathSlow=require('./audit-enemy-death-slow.cjs').auditEnemyDeathSlow();
crt.enemy_death_slow=enemy.DeadEffectSlow;
fs.writeFileSync(path.join(out,'source/enemy-death-slow.json'),JSON.stringify(deathSlow,null,2)+'\n');
// PlayPlayerSideVoice_BattleStart uses TalkPlayerID directly when present/alive;
// RandomInteger is only the fallback, including for one-person parties.
assert.equal(group.TalkPlayerID,'EPlayableCharacterID::eTHIEF');
assert.equal(config.players[0].id,'throne');
crt.battle_start_speaker=config.players[0].id;
const startup=blueprint('Battle/BP/BattleManagerBP',['PlayPlayerSideVoice_BattleStart']);
const startupBody=startup.functions.PlayPlayerSideVoice_BattleStart;
assert(startupBody.includes('StructMemberContext TalkPlayerID_'));
assert(/JumpIfNot target=750\n[^\n]+LocalVariable FindTalkPlayer\n/.test(startupBody));
fs.writeFileSync(path.join(out,'source/pirro-startup.json'),JSON.stringify({
  asset:startup.asset,sha256:startup.sha256,functions:startup.functions,
  group:group.id,talk_player_id:group.TalkPlayerID,resolved_actor:crt.battle_start_speaker,
  note:'Found/alive TalkPlayerID takes direct PlayBattleVoice; only absent/dead/None uses RandomInteger.'
},null,2)+'\n');
// Keep the conditional weakness reaction and asynchronous mode-frame source
// alongside the startup audit. This is research data, never a runtime asset.
const reactionAndMode=[
  blueprint('Library/LibAilment',['Battle_ExecDamageHP','CheckResistance']),
  blueprint('Battle/BP/BattleEnemyBP',['ChangeActionMode','ApplyChangeActionMode']),
  blueprint('Character/BP/Type/KSBattleCharacterEnemy',['InitializeBattleEnemy','ExecuteUbergraph_KSBattleCharacterEnemy'])
].map(({asset,sha256,functions})=>({asset,sha256,functions}));
fs.writeFileSync(path.join(out,'source/pirro-reaction-and-mode.json'),JSON.stringify(reactionAndMode,null,2)+'\n');
for(const id of ['healing_grape','shadow_soulstone','light_soulstone','wind_soulstone','ice_soulstone']){
 crt.skill_uses_ability_camera[id]=hitTimerGates.item_uses_ability_camera;
 const type=id==='healing_grape'?15:16,v=rawVoices.find(r=>r.id==='THIEF_'+(type===15?'eUSE_ITEM_TO_PARTY_SELF':'eUSE_ITEM_TO_ENEMY_COMMON'));
 const file=path.join(s.assets.CONTENT,'Sound/Resource/VO/BattleTOU/en/'+v.VoiceLabel.toLowerCase()+'.uasset'),p=s.assets.loadPackage(file),cue=s.fields(objectHeader(p,p.exports[0]).properties);crt.skill_voice_duration_ticks[id]=Math.ceil(cue.Duration*24000);
}
for(const file of ['throne_pirro.json','throne_pirro_crt.json','items.json'])fs.writeFileSync(path.join(out,file),JSON.stringify(file==='throne_pirro.json'?config:file==='items.json'?catalog:crt,null,2)+'\n');
const hooks=[['Battle/BP/BattleManagerBP',['ReleasePotentialityAction']],['Battle/BP/BattleEnemyBP',['InitializeEnemy','ChangeActionMode']],['Battle/BP/BattlePlayerBP',['ReleasePotentialityAction','SetDamageForPlayer']]].map(([a,n])=>blueprint(a,n));
fs.writeFileSync(path.join(out,'source/throne_pirro.json'),JSON.stringify({sources,group,enemy,abilities:abilities.filter(r=>ids.includes(r.id)),schedules,rawVoices,hooks},null,2)+'\n');
console.log('Imported Pirro, 14 actions, soulstones, scalar schedules and both voice phases.');
