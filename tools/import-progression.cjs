'use strict';
const fs=require('node:fs'),path=require('node:path');
const s=require('./source.cjs');
const growth=s.table('Character/Database/CharacterGrow'),players=s.table('Character/Database/PlayableCharacterDB'),enemies=s.table('Character/Database/EnemyDB'),groups=s.table('Battle/Database/EnemyGroupData');
const items=s.table('Item/Database/ItemDB'),jobs=s.table('Character/Database/JobData');
const characters={},starters={};
const slots=['Sword','Lance','Dagger','Axe','Bow','Rod'],names=['sword','lance','dagger','axe','bow','staff'];
for(const [id,key] of [[7,'eTHIEF'],[6,'ePROFESSOR'],[8,'eDANCER'],[4,'eMERCHANT'],[1,'eFENCER'],[3,'eALCHEMIST'],[5,'ePRIEST']]) {
 const p=players.rows.find(r=>r.id===key);
 if(id===8||id===4||id===1||id===3||id===5) {
  const g=growth.rows.find(r=>Number(r.id)===p.InitLevel).Parameter;
  const base=k=>Math.trunc(g[k]*p.ParameterRevision[k]/100);
  const gear=Object.entries(p.FirstEquipment).filter(([,v])=>v!=='None').map(([slot,id])=>({slot,row:items.rows.find(r=>r.id===id)}));
  const total=k=>base(k)+gear.filter(r=>!slots.includes(r.slot)).reduce((n,r)=>n+r.row.EquipmentRevision[k],0);
  const weapon=names[slots.findIndex(slot=>slot.toUpperCase()===p.MainWeapon.split('::').at(-1).slice(1))];
  if(!weapon)throw new Error('Unsupported original weapon '+p.MainWeapon);
  const [characterId,characterName]={8:['agnea','Agnea'],4:['partitio','Partitio'],1:['hikari','Hikari'],3:['castti','Castti'],5:['temenos','Temenos']}[id];
  starters[id]={id:characterId,name:characterName,stats:{max_hp:total('HP'),max_sp:total('MP'),speed:total('AGI')},resources:{hp:total('HP'),sp:total('MP'),bp:1,latent:0},skills:['attack_'+weapon,...jobs.rows.find(r=>r.id===p.FirstJob.split('::').at(-1)).JobCommandAbility.filter(a=>a.GetParam===1).map(a=>a.AbilityName)],combat:{attack:total('ATK'),defense:total('DEF'),magic:total('MATK'),magic_defense:total('MDEF'),accuracy:total('ACC'),evasion:total('EVA'),critical:total('CON'),growth_attack:g.ATK,growth_magic:g.MATK,weapons:gear.filter(r=>slots.includes(r.slot)).map(r=>({weapon:names[slots.indexOf(r.slot)],attack:r.row.EquipmentRevision.ATK,magic:r.row.EquipmentRevision.MATK})),main_weapon:weapon,shields:0,weapon_weaknesses:[],element_weaknesses:[],statuses:[],steal_item:null}};
 }
 characters[id]=growth.rows.map(r=>({level:Number(r.id),total_exp:r.TotalExp,parameters:Object.fromEntries(Object.entries(r.Parameter).map(([k,v])=>[k,Math.trunc(v*p.ParameterRevision[k]/100)])),growth_attack:r.Parameter.ATK,growth_magic:r.Parameter.MATK}));
}
const rewards=Object.fromEntries(enemies.rows.map(r=>[r.id,{exp:r.Exp,jp:r.JobPoint,money:r.Money}]));
const formations=Object.fromEntries(groups.rows.map(r=>[r.id,r.EnemyID.filter(id=>id!=='None')]));
fs.writeFileSync(path.join(__dirname,'../data/progression.json'),JSON.stringify({characters,starters,rewards,formations},null,2)+'\n');
fs.writeFileSync(path.join(__dirname,'../data/source/progression.json'),JSON.stringify({sources:[growth,players,enemies,groups,items,jobs].map(({source,sha256})=>({source,sha256}))},null,2)+'\n');
