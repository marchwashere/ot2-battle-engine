'use strict';
// Only project flags which gate main-CRT callbacks. Never run presentation.
const assert=require('node:assert/strict');
const {blueprint}=require('./audit-pirro.cjs');

function auditHitTimer() {
  const sources=[];
  function read(asset,name,pattern,before=0,after=0) {
    // Use the schema's real script offset. The old exploratory trace helper
    // silently omitted large ubergraphs and GetActionCommandForItem.
    const p=blueprint(asset,[name]),body=p.functions[name],lines=body.split('\n');
    const indices=lines.flatMap((l,i)=>pattern.test(l)?[i]:[]);
    assert(indices.length,`${asset}.${name}: missing gate`);
    const keep=new Set(indices.flatMap(i=>Array.from({length:before+after+1},(_,n)=>i-before+n).filter(n=>n>=0&&n<lines.length)));
    sources.push({asset,sha256:p.sha256,function:name,excerpt:lines.filter((_,i)=>keep.has(i)).join('\n')});
    return body;
  }
  const item=read('Library/LibActionCommand','GetActionCommandForItem',/StructMemberContext UseAbilityCamera/,1,3);
  assert(/StructMemberContext UseAbilityCamera\n[^\n]+LocalVariable K2Node_MakeStruct_AbilityDataBase\n[^\n]+True/.test(item));
  const init=read('Battle/BP/BattleAction/_Base/BP_ActionCommandBase','Setup Action Command',/EnableAbilityCamera|UseAbilityCamera/,1,3);
  assert(/InstanceVariable EnableAbilityCamera\n[^\n]+StructMemberContext UseAbilityCamera/.test(init));
  const command=read('Battle/BP/BattleAction/_Base/BP_ActionCommandBase','ExecuteUbergraph_BP_ActionCommandBase',/EnableAbilityCamera|StartAbilityCamera/,1,8);
  assert(command.includes('LocalVirtualFunction StartAbilityCamera'));
  const camera=read('Battle/BP/BattleCameraBP','ExecuteUbergraph_BattleCameraBP',/InstanceVariable Playing$|LocalVirtualFunction ExecAbilityCamera/,2,8);
  assert(/InstanceVariable Playing\n[^\n]+True/.test(camera));
  assert(camera.includes('LocalVirtualFunction ExecAbilityCamera'));
  const manager=read('Battle/BP/BattleManagerBP','ExecuteUbergraph_BattleManagerBP',/CallFunc_IsPlaying_playing$|FINISH_BLOW_SLOW|FinishFinishBlow|K2_SetTimerDelegate_ReturnValue_2/,2,13);
  assert(/JumpIfNot target=33608\n[^\n]+LocalVariable CallFunc_IsPlaying_playing\n[^\n]+PopExecutionFlow/.test(manager));
  assert(manager.includes('NameConst FINISH_BLOW_SLOW'));
  assert(manager.includes('BindDelegate FinishFinishBlow'));
  const director=read('Battle/BP/BattleSequenceDirectorBP','ExecuteUbergraph_BattleSequenceDirectorBP',/LocalVirtualFunction FinishAbilityCamera/,2,5);
  assert(director.includes('LocalVirtualFunction FinishAbilityCamera'));
  return {item_uses_ability_camera:true,sources,notes:[
    'GetActionCommandForItem explicitly sets UseAbilityCamera=true; the generic USE_ITEM ability row is not the final item command.',
    'Setup Action Command copies this flag. Pre-action StartAbilityCamera sets Playing; the post-action camera callback clears it.',
    'PlayFinishBlow exits while IsPlaying, before the zero-delay continuation and K2_SetTimerDelegate CRT call.',
    'PlayBreakCamera has a separate pushed continuation for its timer; retain the draw when a hit newly breaks the target.',
    'Normal attacks keep their existing hit/break timing. This is a scalar callback gate, not an item-wide or seed-specific RNG offset.'
  ]};
}
module.exports={auditHitTimer};
if(require.main===module)console.log(JSON.stringify(auditHitTimer(),null,2));
