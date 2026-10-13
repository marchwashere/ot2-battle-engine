'use strict';
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {blueprint}=require('./audit-pirro.cjs');
const order=blueprint('Battle/BP/BattleActionOrderBP',[
  'MoveActionOrder','SetActionOrderChange','BreakActionOrder','RemoveActionOrder','InsertActionOrder',
  'SortPredicateByNextTime','CreateActionOrder','UpdateNextOrder']);
const actor=blueprint('Battle/BP/BattleCharacterBaseBP',[
  'CheckOverTakeEnable','CheckWrapAroundEnable','CheckShuffleEnable',
  'OrderCountControl','GetNextActionOrderData','GetActionTimeForNextTurn']);
const player=blueprint('Battle/BP/BattlePlayerBP',['CalcNextTurnWaitTime','ExecuteUbergraph_BattlePlayerBP']);
// Retain the called event block without committing unrelated player UI code.
assert.match(player.functions.CalcNextTurnWaitTime,/IntConst 23419/);
const lines=player.functions.ExecuteUbergraph_BattlePlayerBP.split('\n');
const start=lines.findIndex(l=>l.startsWith('303b0 '));
const end=lines.findIndex(l=>l.startsWith('30589 '));
assert(start>=0&&end>start);
player.functions.ExecuteUbergraph_BattlePlayerBP=lines.slice(start,end).join('\n');
assert.match(player.functions.ExecuteUbergraph_BattlePlayerBP,/IntConst 20000/);
assert.match(actor.functions.CheckShuffleEnable,/ByteConst 15/);
assert.match(actor.functions.CheckOverTakeEnable,/ByteConst 6/);
assert.doesNotMatch(actor.functions.CheckOverTakeEnable,/ByteConst (15|17)\b/);
assert.match(order.functions.BreakActionOrder,/ByteConst 7/);
assert.match(order.functions.MoveActionOrder,/LocalVariable EnableGuardIndex/);
assert.match(order.functions.MoveActionOrder,/LocalVariable EnableTopIndex/);
const evidence={order_types:{NORMAL:0,TOP:1,FIRST:2,GUARD:3,LAST:4,DELAY:5,BREAK_STYLE:6,BREAK:7},
  diseases:{BE_THE_FIRST:15,GUARD:17,SLOW:61,BE_THE_FIRST_FOR_BATTLE_START:92},
  order,actor,player};
const enums=require('../../OT2BattleEngine/tools/enums.cjs');
for(const [type,values] of [['EACTION_ORDER_TYPE',evidence.order_types],['EDISEASE_TYPE',evidence.diseases]]) {
  const native=enums.load('/Script/Majesty.'+type);
  for(const [name,value] of Object.entries(values)) assert.equal(native.get(type+'::e'+name),value,type+'.'+name);
}
fs.writeFileSync(path.join(__dirname,'../data/source/turn-order-controls.json'),JSON.stringify(evidence,null,2)+'\n');
console.log('Audited queue controls, break insertion, player Boots timing and shuffle eligibility.');
