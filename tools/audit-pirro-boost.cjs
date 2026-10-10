'use strict';
// Establish who owns the BP-threshold reroll; never infer it from BP alone.
const assert=require('node:assert/strict'),path=require('path');
const s=require('./source.cjs'),{blueprint}=require('./audit-pirro.cjs');
function auditPirroBoost() {
  const root='Character/BP/BattleAI/',sources=[];
  for(const [asset,parent] of [
    [root+'BOSS/Thief/BattleAI_Bos_Thi_C01_010',root+'Base/BP_BattleAI_Boss_Base'],
    [root+'Base/BP_BattleAI_Boss_Base',root+'Base/BP_BattleAI_Base'],
    [root+'Base/BP_BattleAI_Base','/Script/Majesty.BattleAIBase']
  ]) {
    const p=s.assets.loadPackage(path.join(s.assets.CONTENT,asset+'.uasset'));
    const cls=p.exports.find(e=>e.name.endsWith('_C')&&!e.name.startsWith('Default__'));
    const superclass=s.assets.objectPath(p,cls.superIndex);
    assert(superclass.startsWith(parent.startsWith('/')?parent:'/Game/'+parent+'.'));
    const names=p.exports.filter(e=>s.assets.objectPath(p,e.classIndex)==='/Script/CoreUObject.Function').map(e=>e.name);
    const decoded=blueprint(asset,names); // Reject any silently omitted function.
    for(const body of Object.values(decoded.functions)) {
      assert(!/LocalVirtualFunction (BoostActionCheck|SetBoostPoint_OLD|CheckBoostMax)\b/.test(body));
    }
    sources.push({asset,sha256:decoded.sha256,superclass,checked_functions:names});
  }
  const normal=blueprint(root+'GeneralEnemy/AI_Pettern/NewAIPattern/BattleAI_NormalEnemy',['UpdateThink','BoostActionCheck']);
  assert(normal.functions.UpdateThink.includes('LocalVirtualFunction BoostActionCheck'));
  assert(/LocalVirtualFunction SetBoostPoint_OLD\n[^\n]+IntConst 0\n[^\n]+True/.test(normal.functions.BoostActionCheck));
  sources.push({asset:normal.asset,sha256:normal.sha256,boost_action_check:normal.functions.BoostActionCheck});
  const enemy=blueprint('Battle/BP/BattleEnemyBP',['SetBoostPoint_OLD','ExecuteUbergraph_BattleEnemyBP']);
  const body=enemy.functions.ExecuteUbergraph_BattleEnemyBP;
  const start=body.indexOf('2a91b PushExecutionFlow'),end=body.indexOf('2aafa Let property');
  assert(start>=0&&end>start);
  const reroll=body.slice(start,end);
  assert(reroll.includes('EqualEqual_IntInt'));
  assert(reroll.includes('K2Node_Event_Use'));
  assert(reroll.includes('RandomIntegerInRange'));
  sources.push({asset:enemy.asset,sha256:enemy.sha256,entry:enemy.functions.SetBoostPoint_OLD,reroll});
  return {sources,conclusion:'NormalEnemy.UpdateThink calls BoostActionCheck, which sets BP to zero with Use=true when full. BattleEnemyBP then rerolls BoostMax. Pirro overrides the boss AI hooks and neither calls nor inherits NormalEnemy; his scripted charged orders are separate. Preserve initialization BP draws, but no automatic threshold reroll at full BP.'};
}
module.exports={auditPirroBoost};
if(require.main===module)console.log(JSON.stringify(auditPirroBoost(),null,2));
