'use strict';
const fs=require('fs'),path=require('path'),assert=require('assert/strict'),s=require('./source.cjs');
const {initialization}=require('./enemy-initialization.cjs');
const enemies=s.table('Character/Database/EnemyDB'),groups=s.table('Battle/Database/EnemyGroupData');
const selected=groups.rows.filter(r=>/^ENG_FLD_((CTY|WLD)_1_3|WLD_2_1|WLD_3_2)_LND_(DAY|NGT)_/.test(r.id));
assert(selected.length>0);assert(selected.every(r=>r.EscapeEnable&&!r.BossBattle));
const params=s.table('GameParam/GameParamDefineTable');
const rareIds=[...new Set(params.rows.filter(r=>r.id.startsWith('RARE_ENEMY_')).flatMap(r=>[r.Min,r.Max,r.Init]))];
const ids=[...new Set([...selected.flatMap(r=>r.EnemyID).filter(x=>x!=='None'),...rareIds])];
const root=path.join(__dirname,'../data'),escape=JSON.parse(fs.readFileSync(path.join(root,'field_escape.json'))),battle=JSON.parse(fs.readFileSync(path.join(root,'field_break.json')));
const weapons=['sword','lance','dagger','axe','bow','staff'],elements=['none','fire','ice','lightning','wind','light','dark'];
for(const id of ids){const r=enemies.rows.find(r=>r.id===id);assert(r);const p=r.Param;
 escape.enemies[id]={speed:p.AGI,bp_base:p.BP,weakness_count:[...r.WeaponResist,...r.AttributeResist].filter(x=>x.endsWith('::eWEAK')).length,initialization:initialization(r)};
 if(battle.enemies[id])continue;
 const e=structuredClone(battle.enemies.ENE_NML_FLD_CTY_010);e.id=id;e.name=id;e.stats={max_hp:p.HP,max_sp:p.MP,speed:p.AGI};e.resources={hp:p.HP,sp:p.MP,bp:r.FirstBP,latent:0};
 Object.assign(e.combat,{use_cat_damage_type:r.UseCatDamageType,attack:p.ATK,defense:p.DEF,magic:p.MATK,magic_defense:p.MDEF,accuracy:p.ACC,evasion:p.EVA,critical:p.CON,growth_attack:Math.round((r.DamageRatio-.5)*128),growth_magic:Math.round((r.DamageRatio-.5)*128),shields:p.SP,enemy_bp_base:p.BP,enemy_initialization:initialization(r),weapon_weaknesses:r.WeaponResist.flatMap((v,i)=>v.endsWith('::eWEAK')?[weapons[i]]:[]),element_weaknesses:r.AttributeResist.flatMap((v,i)=>v.endsWith('::eWEAK')?[elements[i]]:[])});battle.enemies[id]=e;
}
for(const [file,data]of [['field_escape.json',escape],['field_break.json',battle]])fs.writeFileSync(path.join(root,file),JSON.stringify(data,null,2)+'\n');
fs.writeFileSync(path.join(root,'source/highroad-return.json'),JSON.stringify({sources:[enemies,groups,params].map(({source,sha256})=>({source,sha256})),groups:selected,enemy_ids:ids,rare_enemy_ids:rareIds},null,2)+'\n');
console.log(JSON.stringify({enemies:ids}));

// Keep rewards complete for newly supported first-menu item victories.
const rp=path.join(root,'field_rare_rewards.json'),rewards=JSON.parse(fs.readFileSync(rp));
for(const id of ids){const e=enemies.rows.find(r=>r.id===id);rewards[id]={exp:e.Exp,jp:e.JobPoint,money:e.Money,drop_item:e.HaveItemID,drop_rate:e.DropProbability};}
fs.writeFileSync(rp,JSON.stringify(rewards,null,2)+'\n');
