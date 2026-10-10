'use strict';
const assert=require('node:assert/strict');
const {blueprint}=require('./audit-pirro.cjs');
function excerpt(text,start,end) {
  const a=text.indexOf(start),b=text.indexOf(end,a+1);
  assert(a>=0&&b>a);return text.slice(a,b);
}
function auditEnemyDeathSlow() {
  const enemy=blueprint('Battle/BP/BattleEnemyBP',['ExecuteUbergraph_BattleEnemyBP']);
  const gate=excerpt(enemy.functions.ExecuteUbergraph_BattleEnemyBP,'23a57 LetBool','23b56 LetBool');
  assert(/DeadEffectSlow_[\s\S]*Not_PreBool|Not_PreBool[\s\S]*DeadEffectSlow_/.test(gate));
  assert(gate.includes('PlayEnemyDeadCamera'));
  const manager=blueprint('Battle/BP/BattleManagerBP',['PlayEnemyDeadCamera','ExecuteUbergraph_BattleManagerBP']);
  const body=manager.functions.ExecuteUbergraph_BattleManagerBP;
  const start=excerpt(body,'3e627 PushExecutionFlow','3e740 CallMath');
  assert(start.includes('ENEMY_DEAD_SLOW'));
  const timer=excerpt(body,'352e5 ','35385 ');
  assert(timer.includes('K2_SetTimerDelegate'));
  return {enemy:{asset:enemy.asset,sha256:enemy.sha256,gate},
    manager:{asset:manager.asset,sha256:manager.sha256,entry:manager.functions.PlayEnemyDeadCamera,start,timer},
    conclusion:'DeadEffectSlow and non-replay death call PlayEnemyDeadCamera. Its continuation schedules FinishEnemyDeadSlow even if a camera is already playing; zero timer variance still consumes one CRT rand. The live 5595 victory captures it at advance 5897, after damage UI and before victory voice.'};
}
module.exports={auditEnemyDeathSlow};
if(require.main===module)console.log(JSON.stringify(auditEnemyDeathSlow(),null,2));
