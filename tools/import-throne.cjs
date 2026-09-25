'use strict';
// Regenerate the opening encounter from local cooked records; no game runtime.
const fs=require('node:fs'),path=require('node:path'),s=require('./source.cjs');
const sources=[];
function load(name){const t=s.table(name);sources.push({source:t.source,sha256:t.sha256});return new Map(t.rows.map(r=>[r.id,r]));}
const groups=load('Battle/Database/EnemyGroupData'),enemies=load('Character/Database/EnemyDB');
const players=load('Character/Database/PlayableCharacterDB'),jobs=load('Character/Database/JobData');
const growth=load('Character/Database/CharacterGrow'),items=load('Item/Database/ItemDB');
const sets=load('Ability/Database/AbilitySetData'),abilities=load('Ability/Database/AbilityData');
const ailments=load('Ability/Database/AilmentData');
for(const id of ['ADD_GUARD','ADD_ATK_UP','ADD_DEF_DOWN']) {
  const r=ailments.get(id);
  if(r.AddDisease.length!==20||r.AdvValue1!==0||r.AdvValue2!==0||r.DiseaseRatio!==100||r.AddDisease.slice(1).some(v=>v.DiseaseName!=='None'))throw new Error('Status RNG layout changed: '+id);
}
const group=groups.get('ENG_EVE_THI_C01_010');
if(group.EnemyID.filter(v=>v!=='None').join(',')!=='ENE_EVE_THI_CTY_010,ENE_EVE_THI_CTY_010')throw new Error('Opening formation changed');
const weapons=['Sword','Lance','Dagger','Axe','Bow','Rod'];
const weaponNames=['sword','lance','dagger','axe','bow','staff'];
const enumWeapon=v=>({eSWORD:'sword',eLANCE:'lance',eDAGGER:'dagger',eAXE:'axe',eBOW:'bow',eROD:'staff',eNONE:null})[v.split('::').at(-1)];
const enumElement=v=>({eNONE:null,eFIRE:'fire',eICE:'ice',eTHUNDER:'lightning',eWIND:'wind',eLIGHT:'light',eDARK:'dark'})[v.split('::').at(-1)];
const skills=[];
const sourceRows={group,players:[],enemies:[],skills:[],equipment:[],ailments:['ADD_GUARD','ADD_ATK_UP','ADD_DEF_DOWN'].map(id=>ailments.get(id))};
function effects(row,normalWeapon){
  return row.Ailment.filter(a=>a.AilmentName!=='None').map(a=>{
    switch(a.AilmentName){
      case 'PHYSICS_DAMAGE':case 'MAGIC_DAMAGE': return {kind:'damage',attack:{kind:a.AilmentName==='PHYSICS_DAMAGE'?'physical':'magical',weapon:normalWeapon||enumWeapon(row.RestrictWeapon),element:enumElement(row.Attribute),invocation:a.InvocationValue,ability_ratio:row.AbilityRatio,hit_bonus:row.HitRatio,critical_bonus:row.CriticalRatio,critical_power:row.CriticalPower,normal_attack:!!normalWeapon}};
      case 'STEAL_ITEM':return {kind:'steal',bonus:a.InvocationValue};
      case 'ADD_ATK_UP':return {kind:'status',status:{kind:'attack_up',turns:a.InvocationTurn}};
      case 'ADD_DEF_DOWN':return {kind:'status',status:{kind:'defense_down',turns:a.InvocationTurn}};
      case 'ADD_COVERED':return {kind:'cover',hits:a.InvocationTurn};
      default:throw new Error('Unsupported ailment '+a.AilmentName);
    }
  });
}
const names={ABI_SET_THI_010:'Steal',ABI_SET_THI_020:'Darkest Night',ABI_SET_GST_0170:'Slice',ABI_SET_GST_0180:'Sharpen Dagger',ABI_SET_GST_0200:'Cover Me!',ABI_SET_GST_0210:'Heckle',ABI_ENE_0041:'Attack',ABI_ENE_0500:'Slash'};
function skill(id,rowIds,weapon){
  const rows=rowIds.map(id=>{const r=abilities.get(id);if(!r)throw new Error('Missing ability '+id);return r;});
  const r=rows[0];
  const target={eENEMY_SINGLE:'opponent',eSELF:'self_only',ePARTY_SINGLE:r.ExceptEnforcer?'other_ally':'ally',eALL:r.ExceptEnforcer?'everyone_except_self':'unsupported'}[r.TargetType.split('::').at(-1)];
  if(!target||target==='unsupported')throw new Error('Unsupported target '+id);
  const result={id,name:names[id]||id,target,sp_cost:r.CostValue,hit:{kind:'guaranteed'},effects:effects(r,weapon),hits:1,boosted:rows.slice(1).map((r,i)=>({effects:effects(r,weapon),hits:weapon?i+2:1}))};
  skills.push(result);sourceRows.skills.push(...rows);return id;
}
for(const weapon of ['sword','dagger','axe']){
  const set=sets.get('PLAYER_ATTACK_'+weapon.toUpperCase());
  skill('attack_'+weapon,[set.NoBoost,set.BoostLv1,set.BoostLv2,set.BoostLv3],weapon);
}
function player(sourceId,id,name){
  const r=players.get(sourceId),g=growth.get(String(r.InitLevel)).Parameter;
  sourceRows.players.push(r);
  const base=Object.fromEntries(Object.entries(g).map(([k,v])=>[k,Math.trunc(v*r.ParameterRevision[k]/100)]));
  const gear=Object.entries(r.FirstEquipment).filter(([,v])=>v!=='None').map(([slot,id])=>({slot,row:items.get(id)}));
  sourceRows.equipment.push(...gear.map(g=>g.row));
  // Character revision includes the base job. Only a second job adds its revision.
  const armor=gear.filter(g=>!weapons.includes(g.slot));
  const total=k=>base[k]+armor.reduce((n,g)=>n+g.row.EquipmentRevision[k],0);
  const weaponStats=gear.filter(g=>weapons.includes(g.slot)).map(g=>({weapon:weaponNames[weapons.indexOf(g.slot)],attack:g.row.EquipmentRevision.ATK,magic:g.row.EquipmentRevision.MATK}));
  const learned=jobs.get(r.FirstJob.split('::').at(-1)).JobCommandAbility.filter(a=>a.GetParam===1).map(a=>{
    const set=sets.get(a.AbilityName);return skill(a.AbilityName,[set.NoBoost,set.BoostLv1,set.BoostLv2,set.BoostLv3]);
  });
  return {id,name,stats:{max_hp:total('HP'),max_sp:total('MP'),speed:total('AGI')},resources:{hp:total('HP'),sp:total('MP'),bp:1,latent:0},skills:[...weaponStats.map(w=>'attack_'+w.weapon),...learned],combat:{attack:total('ATK'),defense:total('DEF'),magic:total('MATK'),magic_defense:total('MDEF'),accuracy:total('ACC'),evasion:total('EVA'),critical:total('CON'),growth_attack:g.ATK,growth_magic:g.MATK,weapons:weaponStats,main_weapon:enumWeapon(r.MainWeapon),shields:0,weapon_weaknesses:[],element_weaknesses:[],statuses:[],steal_item:null}};
}
const party=[player('eTHIEF','throne','Throné'),player('eGUEST_006','pirro','Pirro'),player('eGUEST_007','scaracci','Scaracci')];
for(const id of ['ABI_ENE_0041','ABI_ENE_0500'])skill(id,[id]);
const e=enemies.get('ENE_EVE_THI_CTY_010'),p=e.Param;
sourceRows.enemies.push(e);
const foes=['pursuer_a','pursuer_b'].map((id,index)=>({id,name:'Pursuer '+(index?'B':'A'),stats:{max_hp:p.HP,max_sp:p.MP,speed:p.AGI},resources:{hp:p.HP,sp:p.MP,bp:e.FirstBP,latent:0},skills:['ABI_ENE_0041','ABI_ENE_0500'],combat:{attack:p.ATK,defense:p.DEF,magic:p.MATK,magic_defense:p.MDEF,accuracy:p.ACC,evasion:p.EVA,critical:p.CON,growth_attack:p.ATK,growth_magic:p.MATK,weapons:[],main_weapon:null,shields:p.SP,weapon_weaknesses:e.WeaponResist.flatMap((v,i)=>v.endsWith('::eWEAK')?[weaponNames[i]]:[]),element_weaknesses:e.AttributeResist.flatMap((v,i)=>v.endsWith('::eWEAK')?[['none','fire','ice','lightning','wind','light','dark'][i]]:[]),statuses:[],steal_item:'healing_grape',enemy_bp_base:p.BP},ai:{schema_version:1,initial_state:'normal',states:[{id:'normal',rules:[{condition:{kind:'always'},selection:{kind:'opening_guard',attack:{skill:'ABI_ENE_0041',target:'random_opponent'},slash:{skill:'ABI_ENE_0500',target:'random_opponent'}}}]}]}}));
const config={schema_version:1,rules:'ot2_opening_v1',seed:123,night_buff_turns:3,players:party,enemies:foes,skills,inventory:{healing_grape:0,inspiriting_plum:0}};
const dir=path.join(__dirname,'../data');
fs.mkdirSync(path.join(dir,'source'),{recursive:true});
fs.writeFileSync(path.join(dir,'throne_opening.json'),JSON.stringify(config,null,2)+'\n');
fs.writeFileSync(path.join(dir,'source/throne_opening.json'),JSON.stringify({sources,notes:['EnemyGroup ENG_EVE_THI_C01_010; opening formation, not Pirro chapter boss.','Default level-one starting gear and learned (GetParam=1) abilities.','Night talent applies after the initial order shuffle; seed precedes actor initialization, not game process startup.','Starting inventory is empty; supply owned items explicitly. No invented consumables.'],rows:sourceRows},null,2)+'\n');
console.log('Imported '+party.length+' players, '+foes.length+' enemies, '+skills.length+' skills.');
