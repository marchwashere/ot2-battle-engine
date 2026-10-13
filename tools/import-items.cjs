'use strict';
// Optional build-time bridge to the existing extractor. The Rust crate builds
// and runs with the checked-in JSON and never imports the legacy runtime.
const fs=require('node:fs'),path=require('node:path');
const legacy=path.resolve(process.argv[2]||path.join(__dirname,'../../OT2BattleEngine'));
const a=require(path.join(legacy,'tools/assets.cjs')),{table}=require(path.join(legacy,'tools/extract.cjs'));
const p=a.loadPackage(path.join(a.CONTENT,'Item/Database/ItemDB.uasset'));
const source=table(p,p.exports.find(e=>a.objectPath(p,e.classIndex)==='/Script/Engine.DataTable'));
const fields=properties=>Object.fromEntries(properties.map(p=>[p.name.text,p.value]));
const selected=[
  ['0010','healing_grape','Healing Grape'],['0020','healing_grape_m','Healing Grape (M)'],
  ['0030','healing_grape_bunch','Healing Grape Bunch'],
  ['0040','inspiriting_plum','Inspiriting Plum'],['0050','inspiriting_plum_m','Inspiriting Plum (M)'],
  ['0060','inspiriting_plum_basket','Inspiriting Plum Basket'],
  ['0070','energizing_pomegranate','Energizing Pomegranate'],
  ['0080','energizing_pomegranate_m','Energizing Pomegranate (M)'],
  ['0100','refreshing_jam','Refreshing Jam'],['0101','rejuvenating_jam','Rejuvenating Jam'],
  ['0110','reinforcing_jam','Reinforcing Jam'],['0111','revitalizing_jam','Revitalizing Jam']
];
const recovery={HP_FIXED_RECOVERY:['hp','fixed'],MP_FIXED_RECOVERY:['sp','fixed'],
  HP_RATIO_RECOVERY:['hp','percent'],MP_RATIO_RECOVERY:['sp','percent'],
  BP_RECOVERY:['bp','fixed'],RECOVERY_POTENTIALITY_ACTION:['latent','fixed']};
const items=selected.map(([suffix,id,name])=>{
  const source_label='ITM_CSM_'+suffix,row=source.rows.find(r=>r.name.text===source_label);
  if(!row)throw new Error('missing original item '+source_label);
  const f=fields(row.properties),target={ 'EBATTLE_TARGET_TYPE::ePARTY_SINGLE':'ally','EBATTLE_TARGET_TYPE::ePARTY_ALL':'all_allies' }[f.TargetType.value.text];
  if(!target||f.AbilityLabel.text!=='None')throw new Error('unsupported item target/extra ability '+source_label);
  const effects=f.Ailment.elements.flatMap(e=>{
    const f=fields(e.properties),label=f.AilmentName.text;if(label==='None')return [];
    const type=recovery[label];if(!type||f.InvocationTurn!==0||f.DiseaseRate!==0||!Number.isInteger(f.InvocationValue)||f.InvocationValue<0)
      throw new Error('unsupported recovery handler '+label);
    return [{kind:'restore',resource:type[0],recovery:{kind:type[1],[type[1]==='fixed'?'amount':'percent']:f.InvocationValue}}];
  });
  return {id,name,source_label,target,effects};
});
const output={schema_version:1,source:'/Game/Item/Database/ItemDB.ItemDB',source_sha256:a.hash(p.b),items};
for(const [suffix,id,name,element] of [
  ['0320','ice_soulstone','Ice Soulstone','ice'],['0380','wind_soulstone','Wind Soulstone','wind'],['0400','wind_soulstone_l','Wind Soulstone (L)','wind'],
  ['0370','thunder_soulstone_l','Thunder Soulstone (L)','lightning'],
  ['0410','light_soulstone','Light Soulstone','light'],['0440','shadow_soulstone','Shadow Soulstone','dark'],['0450','shadow_soulstone_m','Shadow Soulstone (M)','dark']
]) {
  const source_label='ITM_CSM_'+suffix,row=source.rows.find(r=>r.name.text===source_label);
  const f=fields(row.properties),damage=fields(f.Ailment.elements[0].properties);
  if(f.TargetType.value.text!=='EBATTLE_TARGET_TYPE::eENEMY_ALL'||damage.AilmentName.text!=='HP_FIXED_DAMAGE')throw Error('Unsupported soulstone '+source_label);
  items.push({id,name,source_label,target:'all_opponents',effects:[{kind:'elemental_fixed_damage',amount:damage.InvocationValue,element}]});
}
fs.mkdirSync(path.join(__dirname,'../data'),{recursive:true});
fs.writeFileSync(path.join(__dirname,'../data/items.json'),JSON.stringify(output,null,2)+'\n');
console.log('Imported '+items.length+' recovery items and soulstones from '+output.source_sha256);
