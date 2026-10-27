'use strict';
const fs=require('node:fs'),assert=require('node:assert/strict');
const s=require('./source.cjs'),{blueprint}=require('./audit-pirro.cjs');
const tame=blueprint('Battle/BP/BattleTameMonsterBP',['InitializeTameMonster','PlayBattleVoice']);
assert(tame.functions.InitializeTameMonster.includes('BattleVoiceCharacterID'));
assert(tame.functions.PlayBattleVoice.includes('m_cBattleVoiceComponent'));
assert(tame.functions.PlayBattleVoice.includes('m_cOwnerPlayer'));
const inv=s.table('Ability/Database/InvadeData').rows;
const resources=s.table('Character/Database/CharacterResourceDB').rows;
const path=require('node:path').join(__dirname,'../data/source/bg-effects.json');
const data=JSON.parse(fs.readFileSync(path));
for(const p of data.summon_profiles){
 const row=inv.find(x=>x.id===p.id);assert(row);
 const resource=resources.find(x=>x.id===row.ResourceLabel);assert(resource);
 p.has_voice_component=resource.BattleVoiceCharacterID!=='None';
 assert.equal(p.has_voice_component,!p.id.startsWith('PC_ABI_MAKA_'));
}
fs.writeFileSync(path,JSON.stringify(data,null,2)+'\n');
console.log('Verified owner voice fallback: both BG monsters forward; all eight Hired Help variants retain their own component.');
