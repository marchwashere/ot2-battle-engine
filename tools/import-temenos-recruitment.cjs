'use strict';
const fs=require('fs'),path=require('path'),assert=require('assert/strict'),s=require('./source.cjs');
const {audit}=require('./audit-sequence.cjs'),{initialization}=require('./enemy-initialization.cjs'),{objectHeader}=require('../../OT2BattleEngine/tools/properties.cjs');
const sources=[],schedules=[];
const table=n=>{const t=s.table(n);sources.push({source:t.source,sha256:t.sha256});return t.rows;};
const abilities=table('Ability/Database/AbilityData'),enemy=table('Character/Database/EnemyDB').find(r=>r.id==='ENE_EVE_CLE_MNT_030');
const group=table('Battle/Database/EnemyGroupData').find(r=>r.id==='ENG_EVE_CLE_C00_010');
assert.deepEqual(group.EnemyID.filter(x=>x!=='None'),[enemy.id]);assert.equal(enemy.Param.HP,500);
const config=JSON.parse(fs.readFileSync('data/hikari_recruitment.json'));config.skills=config.skills.filter(s=>!s.id.startsWith("attack_"));config.inventory={};config.night_buff_turns=0;
const e=structuredClone(config.enemies[0]),p=enemy.Param;e.id=enemy.id;e.name=enemy.id;e.stats={max_hp:p.HP,max_sp:p.MP,speed:p.AGI};e.resources={hp:p.HP,sp:p.MP,bp:enemy.FirstBP,latent:0};e.skills=['ABI_ENE_0010'];e.ai={schema_version:1,initial_state:'idle',states:[{id:'idle',rules:[{condition:{kind:'always'},selection:{kind:'sequence',actions:[{skill:'ABI_ENE_0010',target:'random_opponent'}]}}]}]};
Object.assign(e.combat,{attack:p.ATK,defense:p.DEF,magic:p.MATK,magic_defense:p.MDEF,accuracy:p.ACC,evasion:p.EVA,critical:p.CON,growth_attack:Math.round((enemy.DamageRatio-.5)*128),growth_magic:Math.round((enemy.DamageRatio-.5)*128),shields:p.SP,enemy_bp_base:p.BP,enemy_initialization:initialization(enemy),weapon_weaknesses:enemy.WeaponResist.flatMap((r,i)=>r.endsWith('::eWEAK')?[['sword','lance','dagger','axe','bow','staff'][i]]:[]),element_weaknesses:enemy.AttributeResist.flatMap((r,i)=>r.endsWith('::eWEAK')?[['none','fire','ice','lightning','wind','light','dark'][i]]:[])});config.enemies=[e];
const crt=JSON.parse(fs.readFileSync('data/gravell_crt.json'));crt.normal_attacks={};
function schedule(id,r,normal=false,hits=1){
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
  if(fn==='JumpToMarker'||fn==='CheckCounterToMarker'&&counter>=hits){
   assert(normal,'Unexpected non-normal jump');
   const label=JSON.parse(/(?:UnicodeStringConst|StringConst) (".*")/.exec(e.body)[1]),dest=marks.get(label);
   assert(dest!==undefined,'Unknown marker '+label);position=dest;
   const next=events.findIndex(e=>e.time>=dest);if(next<0)break;i=next-1;
  }
 }
 if(normal)assert.equal(steps.filter(e=>e.kind==='damage').length,hits);
 assert(steps.some(e=>e.kind==='damage'),id);assert(steps.filter(e=>e.kind==='damage').at(-1).last,id);
 (normal?crt.normal_attacks:crt.skills)[id]=steps;
 schedules.push({id,asset,events});sources.push({source:'/Game/'+asset,sha256:s.assets.hash(s.assets.loadPackage(path.join(s.assets.CONTENT,asset+'.uasset')).b)});
 return steps;
}

for(const [id,prefix,weapon]of [['attack_dagger','DAGGER','dagger'],['attack_staff','ROD','staff'],['attack_lance','SPEAR','lance']]){
 const tiers=[];
 for(const boost of [0,1,2,3]){const r=abilities.find(r=>r.id==='NORMAL_ATTACK_'+prefix+'_'+(boost+1));assert(r);const effects=r.Ailment.filter(a=>a.AilmentName!=='None').map(a=>{assert.equal(a.AilmentName,'PHYSICS_DAMAGE');return {kind:'damage',attack:{kind:'physical',weapon,element:null,invocation:a.InvocationValue,ability_ratio:r.AbilityRatio,hit_bonus:r.HitRatio,critical_bonus:r.CriticalRatio,critical_power:r.CriticalPower,normal_attack:true}}});tiers.push({effects,hits:boost+1});schedule(id+':'+boost,r,true,boost+1);crt.skill_uses_ability_camera[id]=r.UseAbilityCamera;crt.ability_voice_ids[id]=r.id;}
 config.skills.push({id,name:id,target:'opponent',sp_cost:0,hit:{kind:'guaranteed'},...tiers[0],boosted:tiers.slice(1)});
}
config.players.forEach(p=>p.skills=['attack_'+p.combat.main_weapon]);
for(const [file,data]of [['temenos_recruitment.json',config],['temenos_recruitment_crt.json',crt],['source/temenos-recruitment.json',{sources,enemy,group,schedules}]])fs.writeFileSync('data/'+file,JSON.stringify(data,null,2)+'\n');
