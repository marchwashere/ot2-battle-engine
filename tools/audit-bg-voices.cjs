'use strict';
// Read-only cooked-source audit; refreshes only the checked-in audit artifact.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const s=require('./source.cjs'),{blueprint}=require('./audit-pirro.cjs'),{audit}=require('./audit-sequence.cjs');
const bg=JSON.parse(fs.readFileSync(path.join(__dirname,'../data/source/bg-effects.json')));
const voices=s.table('Character/Database/BattleVoiceSet');
const base=blueprint('Battle/BP/BattleAction/_Base/BP_ActionCommandBase',[
 'GetSettledPlayOnce_OnRecovered','SetSettledPlayOnce_OnRecovered','CheckLastHit','SetSummonAbility','PlayVoiceCommand','OnHitDamageSP','OnAttackDamageSP'
]);
const ailment=blueprint('Library/LibAilment',[
 'SetupVoiceFlag_OnRecovered','Battle_ExecRecoveryHP','Battle_ExecRecoveryMP','Battle_ExecRecoveryBP',
 'Battle_ExecRecoveryPOT','Battle_ExecRevive','Battle_ExecDamageHP','Battle_ExecInstantDeath',
 'Battle_ExecForceSetHPTo1','Battle_ExecAddDisease','Battle_ExecReverseDisease','SortDiseaseListForReverse',
 'Battle_ExecRemoveDisease','Battle_ExecDamageMP','Battle_ExecDamageBP','Battle_ExecDamagePOT',
 'Battle_ExecEXPUp','Battle_ExecJPUp','Battle_ExecAddMoney','Battle_ExecDamageSP'
]);
const player=blueprint('Battle/BP/BattlePlayerBP',[
 'CanPlayRecoveredVoice','GetRecoveredVoiceEnforcer','PlayBattleVoice','PlayBattleVoiceForDamage','OnTakeDisease','CanPlayVoiceWhileDisease','OnHitDamageSP'
]);
const random=bg.commands['Battle/BP/BattleAction/RandomSelectAbility/RandomSelectAbilityCommand'];
for(const method of ['GetSettledPlayOnce_OnRecovered','SetSettledPlayOnce_OnRecovered']) {
 assert(base.functions[method].includes('m_bIsSummonAbility'));
 assert(base.functions[method].includes('m_cSummonOwnerAbility'));
}
assert(random.functions.ExecuteUbergraph_RandomSelectAbilityCommand.includes('LocalVirtualFunction SetSummonAbility'));
assert(ailment.functions.SetupVoiceFlag_OnRecovered.includes('GetSettledPlayOnce_OnRecovered'));
assert(ailment.functions.SetupVoiceFlag_OnRecovered.includes('Array_Shuffle'));
for(const id of ['HP','MP','BP','POT'])assert(ailment.functions['Battle_ExecRecovery'+id].includes('SetupVoiceFlag_OnRecovered'));
const profiles=['THIEF','PROFESSOR','DANCER','MERCHANT','FENCER','PRIEST'];
const types=['eTAKE_DAMAGE','eON_DEAD','eON_AVOID','eOPEN_WEAKNESS','eBREAK_ENEMY','eKILL_ENEMY','eON_RECOVERED','eON_REVIVED'];
const imported=JSON.parse(fs.readFileSync(path.join(__dirname,'../data/gravell_crt.json')));
// Verify the full candidate lists, not just the one voice normally heard.
const candidate_counts={};
profiles.forEach((prefix,i)=>{
 const start=voices.rows.findIndex(r=>r.id===prefix+'_VOICE_DATA_START');
 const end=voices.rows.findIndex((r,j)=>j>start&&r.id.endsWith('_VOICE_DATA_START'));
 const eligible=voices.rows.slice(start+1,end).filter(r=>types.some(t=>r.BattleVoiceType.endsWith('::'+t))&&r.EventFlag==='None'&&r.EnemyGroupID==='None');
 for(const row of eligible)assert(imported.voices[i].some(x=>x.id===row.id),`Missing BG reaction candidate ${row.id}`);
 candidate_counts[prefix]=eligible.length;
});
const {PE}=require('../../OT2BattleEngine/tools/pe.cjs');
const pe=new PE(process.env.OT2_GAME_EXE||'C:/Program Files (x86)/Steam/steamapps/common/Octopath_Traveler2/Octopath_Traveler2/Binaries/Win64/Octopath_Traveler2-Win64-Shipping.exe');
// Resolve the actual native enum values instead of guessing status names
// from the numeric SearchDisease operands in the Blueprint.
const disease_enum_base=0x143d80e60;
const disease_enums=[[1,'eSILENCE'],[4,'eSLEEP'],[6,'eSTUN'],[53,'eSTONE'],[85,'eFROZEN'],[101,'eSLEEP_DOOM']].map(([value,name])=>{
 const at=disease_enum_base+16*value;
 assert.equal(pe.text(pe.ptr(at)),'EDISEASE_TYPE::'+name);
 assert.equal(pe.ptr(at+8),value);
 return {name,value,entry:'0x'+at.toString(16)};
});
const disease_gate=player.functions.CanPlayVoiceWhileDisease;
const voice_blocking_diseases=[...disease_gate.matchAll(/LocalVirtualFunction SearchDisease\n\s+\w+ ByteConst (\d+)/g)].map(m=>Number(m[1]));
assert.deepEqual(voice_blocking_diseases,[4,101,85,53,6]);
assert(!voice_blocking_diseases.includes(disease_enums.find(e=>e.name==='eSILENCE').value));
const shield_events=[['OnAttackDamageSP',0x144cc2be0],['OnHitDamageSP',0x144ccd330]].map(([name,at])=>{
 assert.equal(pe.text(pe.ptr(at+16)),name);const flags=pe.u32(at+64);
 assert.equal(flags,0x0c020800); // BlueprintImplementableEvent, no native implementation.
 return {name,descriptor:'0x'+at.toString(16),flags:'0x'+flags.toString(16)};
});
assert(!ailment.functions.Battle_ExecDamageSP.includes('PlayBattleVoice'));
assert(!base.functions.OnHitDamageSP.includes('PlayBattleVoice'));
assert(!base.functions.OnAttackDamageSP.includes('PlayBattleVoice'));
assert(!player.functions.OnHitDamageSP.includes('PlayBattleVoice'));
const reverse_sort=ailment.functions.SortDiseaseListForReverse;
for(const token of ['EnableResist','EnableTurnCount','SupPermanent','Permanent','Less_IntInt',':RandomBool',':EqualEqual_IntInt','Array_Insert'])assert(reverse_sort.includes(token),token);
assert(reverse_sort.indexOf(':RandomBool')<reverse_sort.indexOf(':EqualEqual_IntInt'));
const effects=bg.abilities.filter(a=>/^ABI_DAN_MAKA_\d{3}$/.test(a.id)).sort((a,b)=>a.id.localeCompare(b.id)).map(a=>({
 id:a.id,target:a.TargetType,attribute:a.Attribute,
 operations:a.Ailment.filter(x=>x.AilmentName!=='None'),
 authored_voice_events:bg.steps[a.id].filter(e=>e.kind==='voice'),
 nested_summons:a.Ailment.filter(x=>x.AilmentName.startsWith('PC_ABI_MAKA')).map(x=>x.AilmentName)
}));
assert.equal(effects.length,39);
// Only HP-to-one requests an authored child voice, type NONE. BG's cast voice
// belongs to the parent; damage reactions are requested by ailment callbacks.
assert.deepEqual(effects.flatMap(e=>e.authored_voice_events.map(v=>[e.id,v.voice_type])),[['ABI_DAN_MAKA_007',0]]);
const parent=s.table('Ability/Database/AbilityData').rows.find(r=>r.id==='ABI_DAN_070_04');
const parent_sequence=audit(parent.ActionCommandSequencer.asset.text.split('.')[0].replace('/Game/',''));
const sources=[base,ailment,player,random].map(({asset,sha256})=>({asset,sha256}));
sources.push({asset:voices.source,sha256:voices.sha256});
const evidence={
 shield_events,
 disease_voice_gate:{native_enum_base:'0x'+disease_enum_base.toString(16),disease_enums,voice_blocking_diseases,body:disease_gate},
 reverse_buff_sort:{reverse:ailment.functions.Battle_ExecReverseDisease,sort:reverse_sort,native_random_bool:{function:'FUN_1430d6c80',algorithm:'One CRT rand, binary32 unit times 2, truncate and clamp to 1; true if result == 1'}},
 recovery_owner:{get:base.functions.GetSettledPlayOnce_OnRecovered,set:base.functions.SetSettledPlayOnce_OnRecovered,setup:ailment.functions.SetupVoiceFlag_OnRecovered},
 damage_callbacks:ailment.functions.Battle_ExecDamageHP.split('\n').filter(l=>/IsValid|IsCharacterDead|PlayBattleVoice|CheckLastHit|PlayVoice_/.test(l)),
 player_callbacks:Object.fromEntries(Object.entries(player.functions).map(([k,v])=>[k,v.split('\n').filter(l=>/PlayBattleVoice|CanPlay|Disease|Recovered|IsDead/.test(l))]))
};
fs.writeFileSync(path.join(__dirname,'../data/source/bg-voice-audit.json'),JSON.stringify({sources,candidate_counts,parent_sequence,effects,evidence},null,2)+'\n');
console.log(`Audited ${effects.length} BG children, both nested summons, and ${Object.values(candidate_counts).reduce((a,b)=>a+b,0)} reaction candidates across six route characters.`);
