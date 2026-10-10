'use strict';
const {initialization}=require('./enemy-initialization.cjs');
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const s=require('./source.cjs');
const sources=[];
function table(n){const t=s.table(n);sources.push({source:t.source,sha256:t.sha256});return t.rows;}
const group=table('Battle/Database/EnemyGroupData').find(r=>r.id==='ENG_DNG_CTY_1_1_LND_DAY_110');
const enemy=table('Character/Database/EnemyDB').find(r=>r.id===group.EnemyID[0]);
assert(group.EscapeEnable&&!group.BossBattle);
assert.equal(enemy.EnemyAbility[1],'ABI_ENE_0010');
const config=JSON.parse(fs.readFileSync(path.join(__dirname,'../data/throne_second.json')));
config.players=config.players.slice(0,1);config.night_buff_turns=3;
const p=enemy.Param;
const damageGrowth=(enemy.DamageRatio-0.5)*128;
assert(Number.isInteger(damageGrowth)&&damageGrowth>=0);
config.enemies=[{id:'guard',name:'Diamante Guard I',stats:{max_hp:p.HP,max_sp:p.MP,speed:p.AGI},resources:{hp:p.HP,sp:p.MP,bp:enemy.FirstBP,latent:0},
 skills:['ABI_ENE_0010'],
 // Only the pre-first-menu ordinary action is exposed by FirstEscapeBattle.
 ai:{schema_version:1,initial_state:'ordinary',states:[{id:'ordinary',rules:[{condition:{kind:'always'},selection:{kind:'normal_enemy',attack:{skill:'ABI_ENE_0010',target:'random_opponent'},charged:{skill:'ABI_ENE_0010',target:'random_opponent'},low_hp:{skill:'ABI_ENE_0010',target:'random_opponent'}}}]}]},
 combat:{attack:p.ATK,defense:p.DEF,magic:p.MATK,magic_defense:p.MDEF,accuracy:p.ACC,evasion:p.EVA,critical:p.CON,growth_attack:damageGrowth,growth_magic:damageGrowth,shields:p.SP,enemy_bp_base:p.BP,enemy_initialization:initialization(enemy),
 weapon_weaknesses:['dagger'],element_weaknesses:['dark'],statuses:[],weapons:[],main_weapon:null,steal_item:null}}];
const crt=JSON.parse(fs.readFileSync(path.join(__dirname,'../data/throne_second_crt.json')));
const rows=table('Character/Database/BattleVoiceSet');
const start=rows.findIndex(r=>r.id==='THIEF_VOICE_DATA_START'),end=rows.findIndex((r,i)=>i>start&&r.id.endsWith('_VOICE_DATA_START'));
const types={eBATTLE_START:1,eORDER_START:3,eTAKE_DAMAGE:4,eON_DEAD:5,eNORMAL_ATTACK:6,eNORMAL_ATTACK_LAST_HIT:7,eON_BOOST:8,eOPEN_WEAKNESS:9,eBREAK_ENEMY:10,eON_AVOID:11,eKILL_ENEMY:12};
const voices=rows.slice(start+1,end).filter(r=>types[r.BattleVoiceType.split('::').at(-1)]&&r.EnemyGroupID==='None'&&r.EventFlag==='None'&&!(r.BattleVoiceType.endsWith('::eBATTLE_START')&&(r.BossFlag||r.FieldCommandFlag))).sort((a,b)=>b.Priority-a.Priority);
crt.voices=[voices.map(r=>({id:r.id,type:types[r.BattleVoiceType.split('::').at(-1)],rate:r.VoiceRate,once:r.PlayOnce,border:r.Border,self_only:r.TargetSelf,target:r.TargetID==='None'?null:r.TargetID,counter:r.CounterFlag,decondition:r.DiseaseCategory.endsWith('::eDECONDITION'),pinch:r.PinchFlag,just_break:r.JustBreakFlag})),[],[]];
crt.battle_start_voice=true;crt.victory_voice=false;crt.drops=[];
const param=table('GameParam/BattleParamDefineTable').find(r=>r.id==='ESCAPE_SUCCESS_RATE');
assert.deepEqual([param.Min,param.Max,param.Init,param.Param],['0','15','30','100']);
const growth=table('Character/Database/CharacterGrow');
assert.equal(growth.find(r=>r.id==='3').TotalExp,16);
const out=path.join(__dirname,'../data');
fs.writeFileSync(path.join(out,'mansion_escape.json'),JSON.stringify(config,null,2)+'\n');
fs.writeFileSync(path.join(out,'mansion_escape_crt.json'),JSON.stringify(crt,null,2)+'\n');
fs.writeFileSync(path.join(out,'source/mansion_escape.json'),JSON.stringify({sources,group,enemy,voices,escape_attempt_bonus:[param.Min,param.Max,param.Init,param.Param].map(Number),notes:['Entry is AFTER field selection and player timing initialization.','Fresh route remains level 2: opening <=6 EXP, second <=9 EXP, level 3 needs 16.','Only first Escape attempt is exposed; before that, the guard can only use ordinary Attack.','BanFlag 652 is cleared by 1600; night talent is active.','Failed attempts stop at the check; no fabricated continuation or field reset.']},null,2)+'\n');
console.log('Imported first mansion encounter and scalar voice candidates.');
