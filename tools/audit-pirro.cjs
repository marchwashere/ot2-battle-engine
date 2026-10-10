'use strict';
const fs=require('fs'),path=require('path'),assert=require('node:assert/strict');
const s=require('./source.cjs'),decoder=require('../../OT2BattleEngine/tools/kismet_decoder.cjs');
const {objectHeader}=require('../../OT2BattleEngine/tools/properties.cjs');
const {audit}=require('./audit-sequence.cjs');
function blueprint(asset, include=null) {
  const file=path.join(s.assets.CONTENT,asset+'.uasset'),p=s.assets.loadPackage(file),q=decoder.loadPackage(file);
  const schemas=s.extract.extractPackage(p,file).schemas,functions={};
  for(const ex of p.exports) {
    if(s.assets.objectPath(p,ex.classIndex)!=='/Script/CoreUObject.Function')continue;
    if(include&&!include.includes(ex.name))continue;
    const schema=schemas.find(e=>e.exportIndex===ex.index);
    if(schema?.scriptStart==null)continue;
    const decoded=decoder.decodeFrom(q,ex.name,schema.scriptStart-ex.serialOffset,true);
    assert.equal(decoded.exprs.at(-1).name,'EndOfScript');
    functions[ex.name]=decoded.exprs.map(e=>decoder.flatten(e).join('\n')).join('\n');
  }
  if(include) for(const name of include) assert(functions[name],`Missing decoded function ${asset}.${name}`);
  return {asset,sha256:s.assets.hash(p.b),functions,defaults:p.exports.filter(e=>e.name.startsWith('Default__')).map(e=>s.fields(objectHeader(p,e).properties))};
}
if(require.main===module) {
  const enemy=s.table('Character/Database/EnemyDB').rows.find(r=>r.id==='ENE_BOS_THI_C01_010');
  const abilities=s.table('Ability/Database/AbilityData').rows.filter(r=>enemy.EnemyAbility.includes(r.id));
  const ai=blueprint('Character/BP/BattleAI/BOSS/Thief/BattleAI_Bos_Thi_C01_010');
  const scene=audit('Event/RichEvent/01_Main/70_TOU/ep1/RI_MS_TOU_10_1500');
  const items=s.table('Item/Database/ItemDB').rows.filter(r=>r.Ailment.some(a=>a.AilmentName.includes('DAMAGE')));
  fs.writeFileSync(path.join(__dirname,'../data/source/pirro-audit.json'),JSON.stringify({enemy,abilities,ai,scene,items},null,2)+'\n');
  console.log(JSON.stringify({functions:Object.keys(ai.functions),defaults:ai.defaults,abilities:abilities.map(r=>({id:r.id,name:r.DisplayName,target:r.TargetType,ailment:r.Ailment,sequence:r.ActionCommandSequencer})),items:items.map(r=>({id:r.id,name:r.ItemNameID,attribute:r.Attribute,ailment:r.Ailment,sequence:r.ActionCommandSequencer}))}));
}
module.exports={blueprint};
