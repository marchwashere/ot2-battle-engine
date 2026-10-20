'use strict';
const {initialization}=require('./enemy-initialization.cjs');
// Import scalar combat data and CRT-affecting event keys only.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const s=require('./source.cjs'),{audit}=require('./audit-sequence.cjs');
const {objectHeader}=require('../../OT2BattleEngine/tools/properties.cjs');
const sources=[];
const crtControlFlow=require('./audit-second-crt.cjs').auditSecondCrt();
sources.push(...crtControlFlow.sources);
const textBytes=fs.readFileSync(path.join(s.assets.CONTENT,'GameText/Database/GameTextEN.uexp'));
sources.push({source:'/Game/GameText/Database/GameTextEN',sha256:s.assets.hash(textBytes)});
function english(key) {
  const needle=Buffer.from(key+'_Text\0'),start=textBytes.indexOf(needle);
  assert(start>=0,'Missing English text '+key);
  const p=start+needle.length,n=textBytes.readInt32LE(p);
  return n>0?textBytes.toString('utf8',p+4,p+4+n-1):textBytes.toString('utf16le',p+4,p+4+(-n-1)*2);
}
function table(name) {const t=s.table(name);sources.push({source:t.source,sha256:t.sha256});return new Map(t.rows.map(r=>[r.id,r]));}
const groups=table('Battle/Database/EnemyGroupData'),enemies=table('Character/Database/EnemyDB');
const abilities=table('Ability/Database/AbilityData'),sets=table('Ability/Database/AbilitySetData');
const voices=[...table('Character/Database/BattleVoiceSet').values()];
const placements=table('Battle/Database/BattlePlacementType'),positions=table('Battle/Database/BattlePlacementList');
const group=groups.get('ENG_EVE_THI_C01_020');
assert.deepEqual(group.EnemyID.filter(x=>x!=='None'),['ENE_EVE_THI_CTY_010','ENE_EVE_THI_CTY_020','ENE_EVE_THI_CTY_010']);
const openingPath=path.join(__dirname,'../data/throne_opening.json');
const config=JSON.parse(fs.readFileSync(openingPath,'utf8'));
sources.push({source:'data/throne_opening.json',sha256:s.assets.hash(fs.readFileSync(openingPath))});
const weaponNames=['sword','lance','dagger','axe','bow','staff'];
const elementNames=['none','fire','ice','lightning','wind','light','dark'];
const learned=[];
const sliceSet=sets.get('ABI_SET_GST_0190');
const ratioRows=[sliceSet.NoBoost,sliceSet.BoostLv1,sliceSet.BoostLv2,sliceSet.BoostLv3].map(id=>abilities.get(id));
for(const [i,row] of ratioRows.entries()) {
  assert.equal(row.Ailment[0].AilmentName,'HP_RATIO_DAMAGE');
  assert.equal(row.Ailment[0].InvocationValue,70+i*10);
  assert.equal(row.RestrictWeapon,'EWEAPON_CATEGORY::eDAGGER');
}
const ratioEffect=r=>[{kind:'hp_ratio_damage',percent:r.Ailment[0].InvocationValue,weapon:'dagger',hit_bonus:r.HitRatio}];
config.skills.push({id:sliceSet.id,name:english(ratioRows[0].DisplayName),target:'opponent',sp_cost:ratioRows[0].CostValue,hit:{kind:'guaranteed'},hits:1,
  effects:ratioEffect(ratioRows[0]),boosted:ratioRows.slice(1).map(r=>({hits:1,effects:ratioEffect(r)}))});
config.players[1].skills.push(sliceSet.id);
// Event command 8900 invokes FUN_140f34b30(15,18,2,1): Pirro's third job skill.
const eventFile=path.join(s.assets.CONTENT,'Event/json/MS_TOU_10_0600');
const event=JSON.parse(fs.readFileSync(eventFile,'utf8'));
assert(Object.values(event).some(c=>c.cmd===8900));
sources.push({source:'/Game/Event/json/MS_TOU_10_0600',sha256:s.assets.hash(fs.readFileSync(eventFile))});
for(const id of ['ABI_ENE_0200','ABI_ENE_0010']) {
  const row=abilities.get(id);assert.equal(row.Ailment[0].AilmentName,'PHYSICS_DAMAGE');
  learned.push(row);
  config.skills.push({id,name:english(row.DisplayName),target:'opponent',sp_cost:row.CostValue,hit:{kind:'guaranteed'},
    hits:id==='ABI_ENE_0200'?2:1,boosted:[],effects:[{kind:'damage',attack:{kind:'physical',weapon:null,element:null,
      invocation:row.Ailment[0].InvocationValue,ability_ratio:row.AbilityRatio,hit_bonus:row.HitRatio,
      critical_bonus:row.CriticalRatio,critical_power:row.CriticalPower,normal_attack:false}}]});
}
const ids=['grunt_right','leader','grunt_left'];
config.enemies=group.EnemyID.slice(0,3).map((id,index)=>{
  const row=enemies.get(id),p=row.Param;
  assert.deepEqual(row.EnemyAbility.flatMap((x,i)=>x==='None'?[]:[i]),[0,1,7]);
  const action=i=>({skill:row.EnemyAbility[i],target:'random_opponent'});
  return {id:ids[index],name:english(row.DisplayNameID),stats:{max_hp:p.HP,max_sp:p.MP,speed:p.AGI},
    resources:{hp:p.HP,sp:p.MP,bp:row.FirstBP,latent:0},skills:[...new Set(row.EnemyAbility.filter(x=>x!=='None'))],
    combat:{attack:p.ATK,defense:p.DEF,magic:p.MATK,magic_defense:p.MDEF,accuracy:p.ACC,evasion:p.EVA,critical:p.CON,
      growth_attack:p.ATK,growth_magic:p.MATK,weapons:[],main_weapon:null,shields:p.SP,
      weapon_weaknesses:row.WeaponResist.flatMap((r,i)=>r.endsWith('::eWEAK')?[weaponNames[i]]:[]),
      element_weaknesses:row.AttributeResist.flatMap((r,i)=>r.endsWith('::eWEAK')?[elementNames[i]]:[]),
      statuses:[],steal_item:index===1?'inspiriting_plum':'healing_grape',enemy_bp_base:p.BP,enemy_initialization:initialization(row)},
    // BattleAI_NormalEnemy.AIInitialize: Attack_Ability_ID=1,
    // Hissatu_Ability_ID=0, Okunote_Ability_ID=7.
    ai:{schema_version:1,initial_state:'normal',states:[{id:'normal',rules:[{condition:{kind:'always'},selection:{kind:'normal_enemy',attack:action(1),charged:action(0),low_hp:action(7)}}]}]}};
});
const crt=JSON.parse(fs.readFileSync(path.join(__dirname,'../data/opening_crt.json'),'utf8'));
const types={eBATTLE_START:1,eBATTLE_VICTORY:2,eORDER_START:3,eTAKE_DAMAGE:4,eON_DEAD:5,eNORMAL_ATTACK:6,eNORMAL_ATTACK_LAST_HIT:7,eON_BOOST:8,eOPEN_WEAKNESS:9,eBREAK_ENEMY:10,eON_AVOID:11,eKILL_ENEMY:12,ePLAY_GUARD:17,ePLAY_ABILITY:18};
const abilityIds=['ABI_THI_020_01','ABI_GST_0170_01','ABI_GST_0190_01'];
const voiceRows=[];
crt.voices=['THIEF','GUEST_006','GUEST_007'].map(owner=>{
  const start=voices.findIndex(r=>r.id===owner+'_VOICE_DATA_START');
  const end=voices.findIndex((r,i)=>i>start&&r.id.endsWith('_VOICE_DATA_START'));
  const rows=voices.slice(start+1,end<0?voices.length:end).filter(r=>{
    const type=types[r.BattleVoiceType.split('::').at(-1)];
    return type && (type!==18||abilityIds.includes(r.AbilityID)) &&
      (r.EnemyGroupID==='None'||r.EnemyGroupID===group.id) &&
      (!([1,2].includes(type)&&r.FieldCommandFlag));
  }).sort((a,b)=>b.Priority-a.Priority);
  assert(rows.every(r=>r.EventFlag==='None'));
  assert(rows.length<=64);
  voiceRows.push(...rows);
  return rows.map(r=>({id:r.id,type:types[r.BattleVoiceType.split('::').at(-1)],rate:r.VoiceRate,once:r.PlayOnce,border:r.Border,
    self_only:r.TargetSelf,target:r.TargetID==='None'?null:r.TargetID,counter:r.CounterFlag,
    decondition:r.DiseaseCategory.endsWith('::eDECONDITION'),pinch:r.PinchFlag,just_break:r.JustBreakFlag,...(r.SongFlag?{song:true}:{}),...(r.DanceFlag?{dance:true}:{}),
    ability:r.AbilityID==='None'?null:r.AbilityID}));
});
const schedules=[];crt.skills={};
crt.skill_uses_ability_camera={};
crt.skill_voice_duration_ticks={};
const skillVoiceTiming=[];
for(const id of ['ABI_SET_THI_020','ABI_SET_GST_0170','ABI_SET_GST_0190','ABI_ENE_0041','ABI_ENE_0500','ABI_ENE_0200','ABI_ENE_0010']) {
  const row=abilities.get(sets.get(id)?.NoBoost||id);
  assert.equal(typeof row.UseAbilityCamera,'boolean');
  if(sets.has(id)) for(const key of ['BoostLv1','BoostLv2','BoostLv3'])
    assert.equal(abilities.get(sets.get(id)[key]).UseAbilityCamera,row.UseAbilityCamera);
  crt.skill_uses_ability_camera[id]=row.UseAbilityCamera;
  const asset=row.ActionCommandSequencer.asset.text.replace('/Game/','').split('.')[0];
  const events=audit(asset),steps=[];
  sources.push({source:'/Game/'+asset,sha256:s.assets.hash(s.assets.loadPackage(path.join(s.assets.CONTENT,asset+'.uasset')).b)});
  for(const e of events) {
    if(/LocalVirtualFunction PlayVoice\b/.test(e.body)) steps.push({kind:'voice',tick:e.time,type:Number(/ByteConst (\d+)/.exec(e.body)[1])});
    if(/LocalVirtualFunction ExecAilment\b/.test(e.body)) {
      assert(/IntConst 0/.test(e.body));
      const bools=[...e.body.matchAll(/\b(True|False)\b/g)].map(m=>m[1]==='True');
      assert.equal(bools.length,2);assert(bools[0]);
      steps.push({kind:'damage',tick:e.time,last:bools[1]});
    }
  }
  assert(steps.some(e=>e.kind==='damage')); assert(steps.at(-1).last);
  const skill=config.skills.find(r=>r.id===id);assert.equal(steps.filter(e=>e.kind==='damage').length,skill.hits);
  crt.skills[id]=steps;schedules.push({id,asset,events});
  const voice=voiceRows.find(v=>v.AbilityID===row.id);
  if(voice) {
    const folder=id==='ABI_SET_THI_020'?'BattleTOU':'BattleNPC';
    const cueAsset=`Sound/Resource/VO/${folder}/en/${voice.VoiceLabel.toLowerCase()}`;
    const p=s.assets.loadPackage(path.join(s.assets.CONTENT,cueAsset+'.uasset'));
    const cue=s.fields(objectHeader(p,p.exports[0]).properties);
    assert(Number.isFinite(cue.Duration)&&cue.Duration>0);
    // PrevRemainingTime is decremented by ReceiveTick.DeltaSeconds (game
    // time), the same clock as the authored sequence. It can expire before
    // the real clip ends at 2x. Import metadata, never run an audio system.
    crt.skill_voice_duration_ticks[id]=Math.ceil(cue.Duration*24000);
    const elapsed=steps.find(e=>e.kind==='damage').tick-steps.find(e=>e.kind==='voice').tick;
    assert(Math.abs(elapsed-crt.skill_voice_duration_ticks[id])>1600);
    sources.push({source:'/Game/'+cueAsset,sha256:s.assets.hash(p.b)});
    skillVoiceTiming.push({id,cue:voice.VoiceLabel,duration:cue.Duration,elapsed_game_seconds:elapsed/24000,
      reaction_gate_expired:elapsed>=crt.skill_voice_duration_ticks[id]});
  }
}
crt.victory_voice=group.UseVictoryAction;
// BattleManagerBP.Start returns before random start-voice selection for a
// BattleStartEvent. This group's authored OpenTalk sequence draws no CRT.
crt.battle_start_voice=group.BattleStartEvent===null || group.BattleStartEvent.asset?.text==='None';
crt.drops=group.EnemyID.slice(0,3).map((id,i)=>({rate:enemies.get(id).DropProbability,item:i===1?'inspiriting_plum':'healing_grape'}));
const out=path.join(__dirname,'../data');
// Preserve a level-one variant for routes whose opening did not award 6 EXP.
fs.writeFileSync(path.join(out,'throne_second_level1.json'),JSON.stringify(config,null,2)+'\n');
const characters=table('Character/Database/PlayableCharacterDB'),growth=table('Character/Database/CharacterGrow');
const levelOne=growth.get('1').Parameter,levelTwo=growth.get('2').Parameter;
for(const [i,id] of ['eTHIEF','eGUEST_006','eGUEST_007'].entries()) {
  const p=config.players[i],revision=characters.get(id).ParameterRevision;
  const change=key=>Math.trunc(levelTwo[key]*revision[key]/100)-Math.trunc(levelOne[key]*revision[key]/100);
  for(const [key,stat] of [['HP','max_hp'],['MP','max_sp'],['AGI','speed']]) p.stats[stat]+=change(key);
  for(const [key,stat] of [['ATK','attack'],['DEF','defense'],['MATK','magic'],['MDEF','magic_defense'],['ACC','accuracy'],['EVA','evasion'],['CON','critical']]) p.combat[stat]+=change(key);
  p.combat.growth_attack=levelTwo.ATK;p.combat.growth_magic=levelTwo.MATK;
  p.resources.hp=p.stats.max_hp;p.resources.sp=p.stats.max_sp;
}
fs.writeFileSync(path.join(out,'throne_second.json'),JSON.stringify(config,null,2)+'\n');
fs.writeFileSync(path.join(out,'throne_second_crt.json'),JSON.stringify(crt,null,2)+'\n');
fs.writeFileSync(path.join(out,'source/throne_second.json'),JSON.stringify({sources,group,event,
  notes:['Chapter 1 second scripted battle; CRT path matched live-second-5120.json on 2026-09-27.','Default level two, original equipment; separate level-one variant for openings without the overkill EXP bonus.','Event 0600 command 8900 unlocks Pirro job 18 skill index 2 (Pierce Through).',
    'Pierce Through: min(current HP, maximum HP * percentage / 100), integer division; CalcType 2 skips critical and variance rolls.'],
  formation:placements.get(group.PlacementType).Placement.filter(x=>x!=='None').map(id=>positions.get(id)),
  enemies:group.EnemyID.slice(0,3).map(id=>enemies.get(id)),skills:[...ratioRows,...learned],voiceRows,schedules,skillVoiceTiming,crtControlFlow},null,2)+'\n');
console.log('Imported second sewer fight: three enemies, Pierce Through, skill callback schedules and boss voice candidates.');
