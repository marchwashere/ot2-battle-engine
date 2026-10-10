'use strict';
const {initialization}=require('./enemy-initialization.cjs');
// Bounded solo Osvald Mug profile; scalar callbacks only.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const s=require('./source.cjs'),{audit}=require('./audit-sequence.cjs');
const {objectHeader}=require('../../OT2BattleEngine/tools/properties.cjs');
const sources=[];
function table(name){const t=s.table(name);sources.push({source:t.source,sha256:t.sha256});return new Map(t.rows.map(r=>[r.id,r]));}
const groups=table('Battle/Database/EnemyGroupData'),enemies=table('Character/Database/EnemyDB');
const abilities=table('Ability/Database/AbilityData'),sets=table('Ability/Database/AbilitySetData');
const characters=table('Character/Database/PlayableCharacterDB'),growth=table('Character/Database/CharacterGrow');
const items=table('Item/Database/ItemDB'),voices=[...table('Character/Database/BattleVoiceSet').values()];
assert.equal(table('GameParam/BattleParamDefineTable').get('DISEASE_PARANETER_RATIO').Min,'0.667');
const group=groups.get('ENG_NPC_Twn_Snw_1_2_A_1100'),enemy=enemies.get(group.EnemyID[0]);
assert.deepEqual(group.EnemyID.filter(x=>x!=='None'),[enemy.id]);assert(enemy.NPCFlag);
assert.deepEqual(enemy.EnemyAbility.filter(x=>x!=='None'),['ABI_NPB_2570','ABI_NPB_0040','ABI_NPB_0040']);
const ch=characters.get('ePROFESSOR'),g=growth.get('1').Parameter;
const staff=items.get(ch.FirstEquipment.Rod),armor=items.get(ch.FirstEquipment.Body);
const stat=k=>Math.trunc(g[k]*ch.ParameterRevision[k]/100)+armor.EquipmentRevision[k];
const weapons=['sword','lance','dagger','axe','bow','staff'],elements=['none','fire','ice','lightning','wind','light','dark'];
const config={schema_version:1,rules:'ot2_opening_v1',seed:0,night_buff_turns:0,players:[{
 id:'osvald',name:'Osvald',stats:{max_hp:stat('HP'),max_sp:stat('MP'),speed:stat('AGI')},
 resources:{hp:stat('HP'),sp:stat('MP'),bp:1,latent:0},skills:['attack_staff','ABI_SET_SCH_010','ABI_SET_SCH_020'],
 combat:{attack:stat('ATK'),defense:stat('DEF'),magic:stat('MATK'),magic_defense:stat('MDEF'),accuracy:stat('ACC'),evasion:stat('EVA'),critical:stat('CON'),growth_attack:g.ATK,growth_magic:g.MATK,
 weapons:[{weapon:'staff',attack:staff.EquipmentRevision.ATK,magic:staff.EquipmentRevision.MATK}],main_weapon:'staff',shields:0,weapon_weaknesses:[],element_weaknesses:[],statuses:[],steal_item:null}}],enemies:[],skills:[],inventory:{}};
const p=enemy.Param;
config.enemies.push({id:'cape_cold_man',name:'Man',stats:{max_hp:p.HP,max_sp:p.MP,speed:p.AGI},resources:{hp:p.HP,sp:p.MP,bp:enemy.FirstBP,latent:0},skills:['ABI_NPB_2570','ABI_NPB_0040'],
 combat:{attack:p.ATK,defense:p.DEF,magic:p.MATK,magic_defense:p.MDEF,accuracy:p.ACC,evasion:p.EVA,critical:p.CON,growth_attack:Math.round((enemy.DamageRatio-.5)*128),growth_magic:Math.round((enemy.DamageRatio-.5)*128),weapons:[],main_weapon:null,shields:p.SP,weapon_weaknesses:enemy.WeaponResist.flatMap((r,i)=>r.endsWith('::eWEAK')?[weapons[i]]:[]),element_weaknesses:enemy.AttributeResist.flatMap((r,i)=>r.endsWith('::eWEAK')?[elements[i]]:[]),statuses:[],steal_item:null,enemy_bp_base:p.BP,enemy_initialization:initialization(enemy,s.table('Character/Database/NPCData').rows.find(r=>r.FCmd_EnemyGroup===group.id).ActionID)},
 ai:{schema_version:1,initial_state:'pattern',states:[{id:'pattern',rules:[{condition:{kind:'always'},selection:{kind:'sequence',actions:[{skill:'ABI_NPB_2570',target:'self_only'},{skill:'ABI_NPB_0040',target:'random_opponent'},{skill:'ABI_NPB_0040',target:'random_opponent'}]}}]}]}});
const rawText=fs.readFileSync(path.join(s.assets.CONTENT,'GameText/Database/GameTextEN.uexp'));
function en(key){const b=Buffer.from(key+'_Text\0'),i=rawText.indexOf(b);assert(i>=0,key);const j=i+b.length,n=rawText.readInt32LE(j);return n>0?rawText.toString('utf8',j+4,j+4+n-1):rawText.toString('utf16le',j+4,j+4+(-n-1)*2);}
const effects=(r,weapon=null)=>r.Ailment.filter(a=>a.AilmentName!=='None').map(a=>{
 if(['PHYSICS_DAMAGE','MAGIC_DAMAGE'].includes(a.AilmentName))return{kind:'damage',attack:{kind:a.AilmentName==='MAGIC_DAMAGE'?'magical':'physical',weapon,element:r.Attribute.endsWith('::eNONE')?null:r.Attribute.split('::e')[1].toLowerCase(),invocation:a.InvocationValue,ability_ratio:r.AbilityRatio,hit_bonus:r.HitRatio,critical_bonus:r.CriticalRatio,critical_power:r.CriticalPower,normal_attack:weapon!==null}};
 assert(['ADD_DEF_UP','ADD_MDEF_UP'].includes(a.AilmentName));assert.equal(a.DiseaseRate,100);
 return{kind:'status',status:{kind:a.AilmentName==='ADD_DEF_UP'?'defense_up':'magic_defense_up',turns:a.InvocationTurn}};
});
const specs=[['ABI_SET_SCH_010','Fireball'],['ABI_SET_SCH_020','Icewind'],['attack_staff','Attack'],['ABI_NPB_2570',null],['ABI_NPB_0040',null]];
const rows=specs.map(([id,name])=>{const set=sets.get(id),r=abilities.get(set?.NoBoost||(id==='attack_staff'?'NORMAL_ATTACK_ROD_1':id));const weapon=id==='attack_staff'?'staff':null;
 config.skills.push({id,name:name||en(r.DisplayName),target:id==='ABI_NPB_2570'?'self_only':set?'all_opponents':'opponent',sp_cost:r.CostValue,hit:{kind:'guaranteed'},hits:1,effects:effects(r,weapon),boosted:set?['BoostLv1','BoostLv2','BoostLv3'].map(k=>({hits:1,effects:effects(abilities.get(set[k]))})):[]});return{id,r};});
const crt=JSON.parse(fs.readFileSync(path.join(__dirname,'../data/opening_crt.json')));
crt.skills={};crt.skill_uses_ability_camera={};crt.skill_voice_duration_ticks={};crt.ability_voice_ids={};
crt.battle_start_voice=true;crt.battle_start_speaker=null;crt.victory_voice=group.UseVictoryAction;crt.enemy_death_slow=enemy.DeadEffectSlow;
// Record the drop independently of usable healing items; this profile cannot use it.
crt.drops=[{rate:enemy.DropProbability,item:enemy.HaveItemID}];
const types={eBATTLE_START:1,eBATTLE_VICTORY:2,eORDER_START:3,eTAKE_DAMAGE:4,eON_DEAD:5,eNORMAL_ATTACK:6,eNORMAL_ATTACK_LAST_HIT:7,eON_BOOST:8,eOPEN_WEAKNESS:9,eBREAK_ENEMY:10,eON_AVOID:11,eKILL_ENEMY:12,ePLAY_GUARD:17,ePLAY_ABILITY:18};
const start=voices.findIndex(v=>v.id==='PROFESSOR_VOICE_DATA_START'),end=voices.findIndex((v,i)=>i>start&&v.id.endsWith('_VOICE_DATA_START'));
const voiceRows=voices.slice(start+1,end).filter(r=>types[r.BattleVoiceType.split('::').at(-1)]&&(r.EnemyGroupID==='None'||r.EnemyGroupID===group.id)&&(types[r.BattleVoiceType.split('::').at(-1)]!==18||rows.some(v=>v.r.id===r.AbilityID))).sort((a,b)=>b.Priority-a.Priority);
assert(voiceRows.length<=64);assert(voiceRows.every(r=>r.EventFlag==='None'));
crt.voices=[voiceRows.map(r=>({id:r.id,type:types[r.BattleVoiceType.split('::').at(-1)],rate:r.VoiceRate,once:r.PlayOnce,border:r.Border,self_only:r.TargetSelf,target:r.TargetID==='None'?null:r.TargetID,counter:r.CounterFlag,decondition:r.DiseaseCategory.endsWith('::eDECONDITION'),pinch:r.PinchFlag,just_break:r.JustBreakFlag,ability:r.AbilityID==='None'?null:r.AbilityID})),[],[]];
const schedules=[];
for(const {id,r} of rows){const asset=r.ActionCommandSequencer.asset.text.replace('/Game/','').split('.')[0];const events=audit(asset),steps=[];
 for(const e of events){if(/LocalVirtualFunction PlayVoice\b/.test(e.body))steps.push({kind:'voice',tick:e.time,type:Number(/ByteConst (\d+)/.exec(e.body)[1])});if(/LocalVirtualFunction ExecAilment\b/.test(e.body)){const bs=[...e.body.matchAll(/\b(True|False)\b/g)].map(m=>m[1]==='True');assert.equal(bs.length,2);steps.push({kind:'damage',tick:e.time,last:bs[1],effect:Number(/IntConst (\d+)/.exec(e.body)[1])});}}
 assert(steps.some(e=>e.kind==='damage'));if(id==='attack_staff')crt.staff_attack=steps;else crt.skills[id]=steps;
 crt.skill_uses_ability_camera[id]=r.UseAbilityCamera;crt.ability_voice_ids[id]=r.id;
 const voice=id==='attack_staff'?voiceRows.find(v=>v.BattleVoiceType.endsWith('::eNORMAL_ATTACK')):voiceRows.find(v=>v.AbilityID===r.id);
 if(voice){const cueAsset='Sound/Resource/VO/BattleGAK/en/'+voice.VoiceLabel.toLowerCase();const pkg=s.assets.loadPackage(path.join(s.assets.CONTENT,cueAsset+'.uasset'));const cue=s.fields(objectHeader(pkg,pkg.exports[0]).properties);assert(cue.Duration>0);crt.skill_voice_duration_ticks[id]=Math.ceil(cue.Duration*24000);sources.push({source:'/Game/'+cueAsset,sha256:s.assets.hash(pkg.b)});}
 schedules.push({id,asset,events});}
const out=path.join(__dirname,'../data');fs.writeFileSync(path.join(out,'cape_cold_mug.json'),JSON.stringify(config,null,2)+'\n');fs.writeFileSync(path.join(out,'cape_cold_mug_crt.json'),JSON.stringify(crt,null,2)+'\n');fs.writeFileSync(path.join(out,'source/cape_cold_mug.json'),JSON.stringify({sources,group,enemy,character:ch,staff,armor,abilities:rows,voiceRows,schedules},null,2)+'\n');
console.log('Imported Cape Cold Mug:',config.players[0].stats,config.skills.map(s=>s.name),crt.skill_voice_duration_ticks);
