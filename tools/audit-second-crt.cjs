'use strict';
// Read only the authored control flow which changes main-CRT consumption.
const path=require('node:path'),assert=require('node:assert/strict');
const s=require('./source.cjs'),d=require('../../OT2BattleEngine/tools/kismet_decoder.cjs');
function auditSecondCrt() {
  const sources=[];
  function fn(asset,name) {
    const file=path.join(s.assets.CONTENT,asset+'.uasset'),p=s.assets.loadPackage(file);
    const e=p.exports.find(e=>e.name===name),schema=s.extract.extractPackage(p,file).schemas.find(v=>v.exportIndex===e.index);
    const body=d.decodeFrom(d.loadPackage(file),name,schema.scriptStart-e.serialOffset,true).exprs.map(e=>d.flatten(e).join('\n')).join('\n');
    sources.push({source:'/Game/'+asset,function:name,sha256:s.assets.hash(p.b)});
    return body;
  }
  const ai='Character/BP/BattleAI/GeneralEnemy/AI_Pettern/NewAIPattern/BattleAI_NormalEnemy';
  const init=fn(ai,'AIInitialize'),think=fn(ai,'UpdateThink');
  const slots={Attack_Ability_ID:1,Hissatu_Ability_ID:0,Okunote_Ability_ID:7};
  for(const [name,index] of Object.entries(slots)) {
    assert(new RegExp(`Let property=${name}\\n[^\\n]+InstanceVariable ${name}\\n[^\\n]+IntConst ${index}\\n`).test(init));
    assert(think.includes('InstanceVariable '+name));
  }
  const start=fn('Battle/BP/BattleManagerBP','Start');
  assert(/JumpIfNot[^\n]+\n[^\n]+InstanceVariable m_bIsBattleEvent\n[^\n]+PopExecutionFlow\n[^\n]+LocalVirtualFunction PlayPlayerSideVoice_BattleStart/.test(start));
  const tick=fn('Battle/BP/BattleCharacterComponent/BP_BattleVoiceComponent','ExecuteUbergraph_BP_BattleVoiceComponent');
  assert(/Subtract_FloatFloat\n[^\n]+InstanceVariable PrevRemainingTime\n[^\n]+LocalVariable K2Node_Event_DeltaSeconds/.test(tick));
  const request=fn('Sound/BP/SoundManagerBP','RequestPlayBattleVoiceByLabel');
  assert(/Let property=Duration\n[^\n]+LocalOutVariable Duration\n[^\n]+Context skip=9 rvalue=Duration\n[^\n]+LocalVariable CallFunc_GetVoiceCue_SoundCue\n[^\n]+InstanceVariable Duration/.test(request));
  const damage=fn('Library/LibAilment','Battle_ExecDamageHP');
  assert(/PlayVoice_KillEnemy/.test(damage)&&/PlayVoice_BreakEnemy/.test(damage));
  return {sources,slots,notes:[
    'BattleStartEvent causes Start to return before random party speaker and voice selection.',
    'Slot 1 is ordinary Attack, slot 0 is the charged ability, slot 7 is the low-HP ability.',
    'SoundManager returns cooked cue Duration; voice ReceiveTick subtracts game DeltaSeconds, not unscaled wall time.',
    'LibAilment selects last-hit kill before JustBreak; CommonCommand permits these reactions. Reaction selection consumes main CRT.'
  ]};
}
module.exports={auditSecondCrt};
if(require.main===module) console.log(JSON.stringify(auditSecondCrt(),null,2));
