'use strict';
const fs=require('node:fs'),path=require('node:path');
const s=require('./source.cjs'),d=require('../../OT2BattleEngine/tools/kismet_decoder.cjs');
const out=path.resolve(__dirname,'../../outputs/mansion-escape-audit');fs.mkdirSync(out,{recursive:true});
function fn(asset,name){
 const file=path.join(s.assets.CONTENT,asset+'.uasset'),p=s.assets.loadPackage(file);
 const ex=p.exports.find(e=>e.name===name);if(!ex)throw new Error(name);
 const sc=s.extract.extractPackage(p,file).schemas.find(v=>v.exportIndex===ex.index);
 const body=d.decodeFrom(d.loadPackage(file),name,sc.scriptStart-ex.serialOffset,true).exprs.map(e=>d.flatten(e).join('\n')).join('\n');
 fs.writeFileSync(path.join(out,name+'.txt'),body);return body;
}
if(require.main===module){
 for(const name of ['Escape','EscapeFailed','PlayEscapeSuccess','PlayEscapeFailed','IsEscapeCommandEnable','SetBattleStartType'])fn('Battle/BP/BattleManagerBP',name);
 fn('Library/LibBattle','CheckEscapeSuccess');
 console.log(out);
}
module.exports={fn};
