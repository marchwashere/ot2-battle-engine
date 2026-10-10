'use strict';
const assert=require('node:assert/strict');
const s=require('./source.cjs'),{blueprint}=require('./audit-pirro.cjs');
function auditPirroLatent() {
  const table=s.table('Character/Database/PotentialityActionDB');
  const row=table.rows.find(r=>r.id==='POTENTIALITY_ACTION_THIEF');
  assert.equal(row.ActionPointInit,55);assert.equal(row.ActionPointMax,80);
  const director=blueprint('Battle/BP/BattleSequenceDirectorBP',['ReleasePotentialityAction','ExecuteUbergraph_BattleSequenceDirectorBP']);
  const body=director.functions.ExecuteUbergraph_BattleSequenceDirectorBP;
  const start=body.indexOf('206f5 Context'),end=body.indexOf('207ca LetObj');
  assert(start>=0&&end>start);
  const release=body.slice(start,end);
  assert(release.includes('SearchPlayerCharacter'));
  assert(/LocalVariable CallFunc_SearchPlayerCharacter_PlayerCharacter\n[^\n]+LocalVirtualFunction ReleasePotentialityAction/.test(release));
  const player=blueprint('Battle/BP/BattlePlayerBP',['ReleasePotentialityAction']);
  assert(/FinalFunction import:-488:SetPotentialityActionPoint\n[^\n]+StructMemberContext ActionPointInit/.test(player.functions.ReleasePotentialityAction));
  return {table:{source:table.source,sha256:table.sha256,row},director:{asset:director.asset,sha256:director.sha256,entry:director.functions.ReleasePotentialityAction,release},player,
    conclusion:'The battle sequence director calls ReleasePotentialityAction on the selected player. That sets ActionPointInit (55), not ActionPointMax (80). BattleManagerBP.ReleasePotentialityAction is a separate full-gauge helper and is not this call path.'};
}
module.exports={auditPirroLatent};
if(require.main===module)console.log(JSON.stringify(auditPirroLatent(),null,2));
