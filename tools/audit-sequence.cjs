'use strict';
// Read only gameplay event keys from a cooked sequence, never render it.
const path=require('node:path'),s=require('./source.cjs');
const {objectHeader}=require(path.join(s.root,'OT2BattleEngine/tools/properties.cjs'));
const decoder=require(path.join(s.root,'OT2BattleEngine/tools/kismet_decoder.cjs'));
function audit(name){
  const file=path.join(s.assets.CONTENT,name+'.uasset'),p=s.assets.loadPackage(file);
  const schemas=s.extract.extractPackage(p,file).schemas,q=decoder.loadPackage(file),out=[];
  for(const ex of p.exports){
    if(!/Event.*Section/.test(ex.name.text||ex.name))continue;
    const ev=s.fields(objectHeader(p,ex).properties).EventChannel;
    if(!ev)continue;
    // Event tracks fire in evaluation phases, not package export order. The
    // native default is AtEndOfEvaluation; normal sword's counter explicitly
    // uses AfterSpawn while its simultaneous voice key uses that default.
    const track=p.exports[ex.outerIndex-1];
    const position=track?s.fields(objectHeader(p,track).properties).EventPosition:undefined;
    const phase=position?.endsWith('::AtStartOfEvaluation')?0:position?.endsWith('::AfterSpawn')?1:2;
    for(let i=0;i<ev.KeyValues.length;i++){
      const ix=ev.KeyValues[i].Ptrs.Function.index,e=p.exports[ix-1];
      if(!ix)continue; // Empty/deleted event key.
      const schema=schemas.find(x=>x.exportIndex===ix);
      const body=decoder.decodeFrom(q,e.name,schema.scriptStart-e.serialOffset,true).exprs.map(v=>decoder.flatten(v).join('\n')).join('\n');
      out.push({time:ev.KeyTimes[i].value,function:e.name,body,phase});
    }
  }
  return out.sort((a,b)=>a.time-b.time||a.phase-b.phase).map(({phase,...event})=>event);
}
if(require.main===module){
  const [name,pattern]=process.argv.slice(2);
  for(const e of audit(name))if(!pattern||new RegExp(pattern).test(e.body))console.log(JSON.stringify(e));
}
module.exports={audit};
