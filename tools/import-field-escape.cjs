'use strict';
const {initialization}=require('./enemy-initialization.cjs');
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const s=require('./source.cjs');
const enemies=s.table('Character/Database/EnemyDB');
const groups=s.table('Battle/Database/EnemyGroupData');
const growth=s.table('Character/Database/CharacterGrow');
const players=s.table('Character/Database/PlayableCharacterDB');
const encounterData=require('../../OT2SpeedrunAssistant/data/encounters.json');
const snowIds=Object.values(encounterData.encounters).filter(t=>/^ENC_FLD_SNW_(1_2|2_2)_LND_(DAY|NGT)_010$/.test(t.id)).flatMap(t=>t.groups.filter(g=>g.weight>0 && g.id!=='None').flatMap(g=>{const row=groups.rows.find(r=>r.id===g.id);assert(row,'Missing snow group '+g.id);return row.EnemyID;})).filter(id=>id!=='None');
const forestGroups=groups.rows.filter(r=>/^ENG_FLD_FST_1_3_LND_(DAY|NGT)_/.test(r.id));
assert(forestGroups.length>0 && forestGroups.every(r=>r.EscapeEnable&&!r.BossBattle&&r.TalkPlayerID==='EPlayableCharacterID::eNONE'&&r.BattleStartEvent.asset.text==='None'));
const forestIds=forestGroups.flatMap(r=>r.EnemyID).filter(id=>id!=='None');
const waterGroups=groups.rows.filter(r=>/^ENG_FLD_SEA_1_3_WTR_DAY_/.test(r.id));
assert.equal(waterGroups.length,12);
assert(waterGroups.every(r=>r.EscapeEnable&&!r.BossBattle&&r.TalkPlayerID==='EPlayableCharacterID::eNONE'&&r.BattleStartEvent.asset.text==='None'));
const waterIds=[...waterGroups.flatMap(r=>r.EnemyID).filter(id=>id!=='None'),'ENE_NML_WTR_550']; // Harbor Octopuff
const ids=[...new Set(['ENE_NML_FLD_CTY_010','ENE_NML_FLD_ISD_100','ENE_NML_FLD_CTY_020','ENE_NML_FLD_CTY_120','ENE_NML_OTR_130','ENE_NML_WTR_530','ENE_NML_FLD_WLD_020','ENE_NML_FLD_WLD_010','ENE_NML_FLD_WLD_210','ENE_NML_FLD_WLD_270','ENE_NML_FLD_SNW_010','ENE_NML_FLD_SNW_020','ENE_NML_FLD_SNW_160','ENE_NML_FLD_SNW_100','ENE_NML_FLD_SNW_310','ENE_NML_FLD_SNW_330','ENE_NML_FLD_SNW_290','ENE_NML_WTR_510','ENE_NML_OTR_140','ENE_NML_WTR_590','ENE_NML_FLD_SEA_130','ENE_NML_WTR_560','ENE_NML_WTR_570',...snowIds,...forestIds,...waterIds])];
const rows=ids.map(id=>enemies.rows.find(r=>r.id===id));
assert(rows.every(r=>r.EnemyAI.asset.text.endsWith('/BattleAI_NormalEnemy.BattleAI_NormalEnemy_C')));
const roadGroups=groups.rows.filter(r=>/^ENG_FLD_CTY_1_3_LND_DAY_1[1-6]0$/.test(r.id));
assert.equal(roadGroups.length,6);
assert(roadGroups.every(r=>r.EscapeEnable&&!r.BossBattle&&r.TalkPlayerID==='EPlayableCharacterID::eNONE'&&r.BattleStartEvent.asset.text==='None'));
const harborGroups=groups.rows.filter(r=>/^ENG_FLD_WLD_1_3_LND_DAY_1[1-5]0$/.test(r.id));
assert.equal(harborGroups.length,5);
assert(harborGroups.every(r=>r.EscapeEnable&&!r.BossBattle&&r.TalkPlayerID==='EPlayableCharacterID::eNONE'&&r.BattleStartEvent.asset.text==='None'));
const snowsGroups=groups.rows.filter(r=>/^ENG_FLD_SNW_1_2_LND_(DAY|NGT)_1[1-7]0$/.test(r.id));
assert(snowsGroups.length>0 && snowsGroups.every(r=>r.EscapeEnable&&!r.BossBattle&&r.TalkPlayerID==='EPlayableCharacterID::eNONE'&&r.BattleStartEvent.asset.text==='None'));
const throne=players.rows.find(r=>r.id==='eTHIEF');
const items=s.table('Item/Database/ItemDB');
const speedEquipment=Object.entries(throne.FirstEquipment).filter(([k])=>!['Sword','Lance','Dagger','Axe','Bow','Rod'].includes(k)).map(([,id])=>items.rows.find(r=>r.id===id)).filter(Boolean);
// Initial armor has no speed modifier. Weapons cannot change the initial order speed.
assert.equal(speedEquipment.length,3);
assert(speedEquipment.every(r=>r.EquipmentRevision.AGI===0));
const routeGroups=['ENG_EVE_THI_C01_020','ENG_BOS_THI_C01_010'];
const professor=players.rows.find(r=>r.id==='ePROFESSOR');
const osvaldArmor=Object.entries(professor.FirstEquipment).filter(([k])=>!['Sword','Lance','Dagger','Axe','Bow','Rod'].includes(k)).map(([,id])=>items.rows.find(r=>r.id===id)).filter(Boolean);
const data={route_base_exp:Object.fromEntries(routeGroups.map(id=>[id,groups.rows.find(r=>r.id===id).EnemyID.filter(n=>n!=='None').reduce((total,id)=>total+enemies.rows.find(r=>r.id===id).Exp,0)])),enemies:Object.fromEntries(rows.map(r=>[r.id,{speed:r.Param.AGI,bp_base:r.Param.BP,weakness_count:[...r.WeaponResist,...r.AttributeResist].filter(x=>x.endsWith('::eWEAK')).length,initialization:initialization(r)}])),
 osvald_levels:growth.rows.map(r=>({level:Number(r.id),total_exp:r.TotalExp,max_hp:Math.trunc(r.Parameter.HP*professor.ParameterRevision.HP/100),speed:Math.trunc(r.Parameter.AGI*professor.ParameterRevision.AGI/100)+osvaldArmor.reduce((n,x)=>n+x.EquipmentRevision.AGI,0)})),
 levels:growth.rows.map(r=>({level:Number(r.id),total_exp:r.TotalExp,max_hp:Math.trunc(r.Parameter.HP*throne.ParameterRevision.HP/100),speed:Math.trunc(r.Parameter.AGI*throne.ParameterRevision.AGI/100)}))};
// Bounded one-action break profile. Enemy AI is never executed by this API.
const config=JSON.parse(fs.readFileSync(path.join(__dirname,'../data/mansion_escape.json')));
const p=config.players[0],base=growth.rows.find(r=>r.id==='2').Parameter,level=growth.rows.find(r=>r.id==='5').Parameter;
const change=key=>Math.trunc(level[key]*throne.ParameterRevision[key]/100)-Math.trunc(base[key]*throne.ParameterRevision[key]/100);
for(const [key,stat] of [['HP','max_hp'],['MP','max_sp'],['AGI','speed']])p.stats[stat]+=change(key);
for(const [key,stat] of [['ATK','attack'],['DEF','defense'],['MATK','magic'],['MDEF','magic_defense'],['ACC','accuracy'],['EVA','evasion'],['CON','critical']])p.combat[stat]+=change(key);
p.combat.growth_attack=level.ATK;p.combat.growth_magic=level.MATK;
p.resources={hp:p.stats.max_hp,sp:p.stats.max_sp,bp:1,latent:0};
config.night_buff_turns=0;
const weapons=['sword','lance','dagger','axe','bow','staff'],elements=['none','fire','ice','lightning','wind','light','dark'];
const singleEnemies=Object.fromEntries(rows.map(row=>{
  const enemy=structuredClone(config.enemies[0]),p=row.Param;
  enemy.id=row.id;enemy.name=row.id;enemy.stats={max_hp:p.HP,max_sp:p.MP,speed:p.AGI};
  enemy.resources={hp:p.HP,sp:p.MP,bp:row.FirstBP,latent:0};
  Object.assign(enemy.combat,{use_cat_damage_type:row.UseCatDamageType,attack:p.ATK,defense:p.DEF,magic:p.MATK,magic_defense:p.MDEF,accuracy:p.ACC,evasion:p.EVA,critical:p.CON,
    growth_attack:Math.round((row.DamageRatio-.5)*128),growth_magic:Math.round((row.DamageRatio-.5)*128),shields:p.SP,enemy_bp_base:p.BP,enemy_initialization:initialization(row),
    weapon_weaknesses:row.WeaponResist.flatMap((r,i)=>r.endsWith('::eWEAK')?[weapons[i]]:[]),
    element_weaknesses:row.AttributeResist.flatMap((r,i)=>r.endsWith('::eWEAK')?[elements[i]]:[])});
  return [row.id,enemy];
}));
const voiceTable=s.table('Character/Database/BattleVoiceSet'),voiceRows=voiceTable.rows;
const start=voiceRows.findIndex(r=>r.id==='THIEF_VOICE_DATA_START'),end=voiceRows.findIndex((r,i)=>i>start&&r.id.endsWith('_VOICE_DATA_START'));
const types={eBATTLE_START:1,eORDER_START:3,eTAKE_DAMAGE:4,eON_DEAD:5,eNORMAL_ATTACK:6,eNORMAL_ATTACK_LAST_HIT:7,eON_BOOST:8,eOPEN_WEAKNESS:9,eBREAK_ENEMY:10,eON_AVOID:11,eKILL_ENEMY:12};
const voices=voiceRows.slice(start+1,end).filter(r=>types[r.BattleVoiceType.split('::').at(-1)]&&r.EnemyGroupID==='None'&&r.EventFlag==='None'&&!(r.BattleVoiceType.endsWith('::eBATTLE_START')&&(r.BossFlag||r.FieldCommandFlag))).sort((a,b)=>b.Priority-a.Priority);
assert(voices.length<=64);
const crt=JSON.parse(fs.readFileSync(path.join(__dirname,'../data/mansion_escape_crt.json')));
crt.voices=[voices.map(r=>({id:r.id,type:types[r.BattleVoiceType.split('::').at(-1)],rate:r.VoiceRate,once:r.PlayOnce,border:r.Border,self_only:r.TargetSelf,target:r.TargetID==='None'?null:r.TargetID,counter:r.CounterFlag,decondition:r.DiseaseCategory.endsWith('::eDECONDITION'),pinch:r.PinchFlag,just_break:r.JustBreakFlag,...(r.SongFlag?{song:true}:{}),...(r.DanceFlag?{dance:true}:{})})),[],[]];
const professorStart=voiceRows.findIndex(r=>r.id==='PROFESSOR_VOICE_DATA_START');
const professorEnd=voiceRows.findIndex((r,i)=>i>professorStart&&r.id.endsWith('_VOICE_DATA_START'));
const professorVoices=voiceRows.slice(professorStart+1,professorEnd).filter(r=>types[r.BattleVoiceType.split('::').at(-1)]&&r.EnemyGroupID==='None'&&r.EventFlag==='None'&&!(r.BattleVoiceType.endsWith('::eBATTLE_START')&&(r.BossFlag||r.FieldCommandFlag))).sort((a,b)=>b.Priority-a.Priority);
assert(professorVoices.length<=64);
const partyCrt=structuredClone(crt);
const mugCrt=JSON.parse(fs.readFileSync(path.join(__dirname,'../data/cape_cold_mug_crt.json')));
partyCrt.staff_attack=mugCrt.staff_attack;
partyCrt.skill_voice_duration_ticks.attack_staff=mugCrt.skill_voice_duration_ticks.attack_staff;
partyCrt.voices[1]=professorVoices.map(r=>({id:r.id,type:types[r.BattleVoiceType.split('::').at(-1)],rate:r.VoiceRate,once:r.PlayOnce,border:r.Border,self_only:r.TargetSelf,target:r.TargetID==='None'?null:r.TargetID,counter:r.CounterFlag,decondition:r.DiseaseCategory.endsWith('::eDECONDITION'),pinch:r.PinchFlag,just_break:r.JustBreakFlag,...(r.SongFlag?{song:true}:{}),...(r.DanceFlag?{dance:true}:{})}));
const dancerStart=voiceRows.findIndex(r=>r.id==='DANCER_VOICE_DATA_START');
const dancerEnd=voiceRows.findIndex((r,i)=>i>dancerStart&&r.id.endsWith('_VOICE_DATA_START'));
assert(dancerStart>=0 && dancerEnd>dancerStart);
const dancerVoices=voiceRows.slice(dancerStart+1,dancerEnd).filter(r=>types[r.BattleVoiceType.split('::').at(-1)]&&r.EnemyGroupID==='None'&&r.EventFlag==='None'&&!(r.BattleVoiceType.endsWith('::eBATTLE_START')&&(r.BossFlag||r.FieldCommandFlag))).sort((a,b)=>b.Priority-a.Priority);
assert(dancerVoices.length<=64);
partyCrt.voices[2]=dancerVoices.map(r=>({id:r.id,type:types[r.BattleVoiceType.split('::').at(-1)],rate:r.VoiceRate,once:r.PlayOnce,border:r.Border,self_only:r.TargetSelf,target:r.TargetID==='None'?null:r.TargetID,counter:r.CounterFlag,decondition:r.DiseaseCategory.endsWith('::eDECONDITION'),pinch:r.PinchFlag,just_break:r.JustBreakFlag,...(r.SongFlag?{song:true}:{}),...(r.DanceFlag?{dance:true}:{})}));
fs.writeFileSync(path.join(__dirname,'../data/field_party_crt.json'),JSON.stringify(partyCrt,null,2)+'\n');
const partyConfig=structuredClone(config);
const mugConfig=JSON.parse(fs.readFileSync(path.join(__dirname,'../data/cape_cold_mug.json')));
const osvald=structuredClone(mugConfig.players[0]);osvald.skills=['attack_staff'];
partyConfig.players.push(osvald);partyConfig.skills.push(mugConfig.skills.find(s=>s.id==='attack_staff'));

fs.writeFileSync(path.join(__dirname,'../data/field_break.json'),JSON.stringify({level:5,config,party_config:partyConfig,enemies:singleEnemies},null,2)+'\n');
fs.writeFileSync(path.join(__dirname,'../data/field_break_crt.json'),JSON.stringify(crt,null,2)+'\n');
fs.writeFileSync(path.join(__dirname,'../data/field_escape.json'),JSON.stringify(data,null,2)+'\n');
fs.writeFileSync(path.join(__dirname,'../data/source/field_escape.json'),JSON.stringify({sources:[enemies,groups,growth,players,items,voiceTable].map(({source,sha256})=>({source,sha256})),enemies:rows,groups:[...roadGroups,...snowsGroups,...forestGroups],voices,professorVoices,dancerVoices,notes:['First player menu at Normal/Preemptive daytime openings; Surprise is rejected before consuming state.','All imported enemies use BattleAI_NormalEnemy, including the possible low-tier Cait and Octopuff.','Uses shared core shuffle, combat, ordinary voices and Escape formula. No enemy actions or continuation after a failed Escape are exposed.','Single-enemy break profile at route level 5: original equipment, actual weakness/shields/hit rolls, then guaranteed Escape while the enemy remains broken. Internal AI template is unreachable; step_enemy explicitly rejects this private profile.']},null,2)+'\n');
