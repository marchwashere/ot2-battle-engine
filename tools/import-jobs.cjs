'use strict';
// Build-time projection of Scholar assignment; no menu/presentation runtime.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const s=require('./source.cjs'),{blueprint}=require('./audit-pirro.cjs');
const sources=[];
function table(name){const t=s.table(name);sources.push({source:t.source,sha256:t.sha256});return new Map(t.rows.map(r=>[r.id,r]));}
const jobs=table('Character/Database/JobData'),players=table('Character/Database/PlayableCharacterDB');
const items=table('Item/Database/ItemDB'),params=table('GameParam/GameParamDefineTable');
const sets=table('Ability/Database/AbilitySetData'),abilities=table('Ability/Database/AbilityData');
const growth=table('Character/Database/CharacterGrow');
const scholar=jobs.get('ePROFESSOR');assert.equal(scholar.JobID,6);
const weaponNames=['sword','lance','dagger','axe','bow','staff'];
const defaults=['SWORD','LANCE','DAGGER','AXE','BOW','ROD'].map((name,i)=>{
 const param=params.get('FIRST_EQUIPMENTID_'+name),item=items.get(param.Param);assert(item);
 return {weapon:weaponNames[i],source_id:item.id,attack:item.EquipmentRevision.ATK,magic:item.EquipmentRevision.MATK};
});
assert.equal(defaults[5].source_id,'ITM_EQP_ROD_010');assert.equal(defaults[5].attack,0);assert.equal(defaults[5].magic,0);
const characters={};
const imported=JSON.parse(fs.readFileSync(path.join(__dirname,'../data/progression.json')));
for(const [id,name,coreId] of [[7,'eTHIEF','throne'],[6,'ePROFESSOR','osvald'],[8,'eDANCER','agnea'],[4,'eMERCHANT','partitio'],[1,'eFENCER','hikari'],[5,'ePRIEST','temenos']]){
 const p=players.get(name),main=jobs.get(p.FirstJob.split('::').at(-1));
 characters[id]={id:coreId,main_job:main.JobID,revision:p.ParameterRevision,permitted_weapons:weaponNames.filter((_,i)=>main.ProperEquipment[i])};
 // Native Multiply_CharacterParam_RevisionParam is float32 product*.01 then
 // FloorToInt. Verify the existing growth projection agrees at every level.
 for(const g of growth.values())for(const [k,v] of Object.entries(g.Parameter)) {
  const expected=Math.floor(Math.fround(Math.fround(v*p.ParameterRevision[k])*Math.fround(.01)));
  assert.equal(imported.characters[id][Number(g.id)-1].parameters[k],expected,`${id}/${g.id}/${k}`);
 }
}
const text=fs.readFileSync(path.join(s.assets.CONTENT,'GameText/Database/GameTextEN.uexp'));
sources.push({source:'/Game/GameText/Database/GameTextEN',sha256:s.assets.hash(text)});
function english(key){const needle=Buffer.from(key+'_Text\0'),start=text.indexOf(needle);assert(start>=0,key);const at=start+needle.length,n=text.readInt32LE(at);return n>0?text.toString('utf8',at+4,at+4+n-1):text.toString('utf16le',at+4,at+4+(-n-1)*2);}
const scholarAbilities=['ABI_SET_SCH_010','ABI_SET_SCH_020','ABI_SET_SCH_030','ABI_SET_SCH_060'].map(id=>{
 const set=sets.get(id),tier_ids=['NoBoost','BoostLv1','BoostLv2','BoostLv3'].map(k=>set[k]),tiers=tier_ids.map(id=>abilities.get(id));
 return {id,name:english(tiers[0].DisplayName),default_on_assignment:scholar.JobCommandAbility.find(a=>a.AbilityName===id).GetParam===1,
  sp_cost:tiers[0].CostValue,tier_ids,tiers};
});
assert.deepEqual(scholarAbilities.map(a=>a.name),['Fireball','Icewind','Lightning Bolt','Elemental Barrage']);
const functions=['GetPlayerParameterByID','Calc Player Parameter','Calc Player Parameter Internal','GetPlayerProperEquipment',
 'RefreshWeaponEquipmentState','SetPlayerRoundingHPAndMP','RemoveEquipmentItem','ClampMaxParameter'];
const characterAudit=blueprint('Library/LibCharacter',functions);
const itemAudit=blueprint('Item/BP/ItemFunction',['GetFirstEquipmentID']);
sources.push(...[characterAudit,itemAudit].map(({asset,sha256})=>({source:'/Game/'+asset,sha256})));
const nativePath=path.join(s.root,'Octopath_Traveler2-Win64-Shipping.exe.c'),nativeText=fs.readFileSync(nativePath,'utf8'),native={};
for(const name of ['FUN_140fcd1a0','FUN_140fcaf50']){
 const m=new RegExp('^[A-Za-z_][^\\r\\n]* '+name+'\\(','m').exec(nativeText);assert(m,name);
 const body=nativeText.slice(m.index,nativeText.indexOf('\n}',m.index)+2);
 native[name]={source:path.basename(nativePath),function:name,sha256:s.assets.hash(Buffer.from(body)),body};
}
assert(native.FUN_140fcd1a0.body.includes('FUN_140fcaf50'));
assert.equal((native.FUN_140fcaf50.body.match(/ROUND/g)||[]).length,13);
const data={scholar:{id:6,revision:scholar.ParameterRevision,permitted_weapons:weaponNames.filter((_,i)=>scholar.ProperEquipment[i])},
 raw_growth:Object.fromEntries([...growth.values()].map(r=>[r.id,r.Parameter])),
 characters,default_weapons:defaults,scholar_abilities:scholarAbilities,
 maximum_hp_sp:Number(params.get('PLAYER_STATUS_HP_RANGE').Max),maximum_stat:Number(params.get('PLAYER_STATUS_RANGE').Max)};
data.additional_jobs=['eINVENTOR','eMERCHANT','eDANCER','eFENCER'].map(id=>{const j=jobs.get(id);return {id:j.JobID,revision:j.ParameterRevision,permitted_weapons:weaponNames.filter((_,i)=>j.ProperEquipment[i])};});
const out=path.join(__dirname,'../data');
fs.writeFileSync(path.join(out,'jobs.json'),JSON.stringify(data,null,2)+'\n');
fs.writeFileSync(path.join(out,'source/jobs.json'),JSON.stringify({sources,scholar,ability_sets:scholarAbilities.map(a=>sets.get(a.id)),ability_definitions:scholarAbilities,characters:[...players.values()].filter(p=>Object.values(characters).some(c=>c.main_job===jobs.get(p.FirstJob.split('::').at(-1))?.JobID)),default_items:defaults.map(d=>items.get(d.source_id)),staff_normal_attack:abilities.get('NORMAL_ATTACK_ROD_1'),characterAudit,itemAudit,native,
 notes:'Scholar is applied to already character-revised base stats before flat gear/support values. Refresh fills permitted empty weapon slots directly with default equipment IDs; it does not award inventory items. HP/SP clamp, without healing. Ability rows are metadata, not an implementation of random Barrage execution.'},null,2)+'\n');
console.log('Imported Scholar assignment:',data.scholar,scholarAbilities.map(a=>[a.id,a.name]));
