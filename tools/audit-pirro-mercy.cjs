'use strict';
const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path');
const s=require('./source.cjs'),{blueprint}=require('./audit-pirro.cjs');
function auditPirroMercy() {
  const abilityTable=s.table('Ability/Database/AbilityData');
  const ailmentTable=s.table('Ability/Database/AilmentData');
  const ability=abilityTable.rows.find(r=>r.id==='ABI_BOS_THI_C01_010_008');
  const ratio=ailmentTable.rows.find(r=>r.id==='HP_RATIO_DAMAGE');
  const status=ailmentTable.rows.find(r=>r.id==='ADD_THIS_ATTACK_HIT_FAILED');
  assert.equal(ability.Ailment[0].AilmentName,ratio.id);
  assert.equal(ability.Ailment[0].InvocationValue,1);
  assert.equal(ability.Ailment[1].AilmentName,status.id);
  assert.equal(ratio.CalcType,'EAILMENT_CALC_TYPE_OLD::NewEnumerator2');
  assert.equal(status.AddDisease[0].DiseaseName,'ATTACK_HIT_FAILED_NO_ICON');
  const command=blueprint('Battle/BP/BattleAction/_Base/BP_ActionCommandBase',[
    'JudgeSpecialHitRate','CalcAilmentDamageHP'
  ]);
  const gate=command.functions.JudgeSpecialHitRate;
  const start=gate.lastIndexOf('32165 LetBool');
  assert(start>=0);
  const forcedMissGate=gate.slice(start);
  assert(/EqualEqual_ByteByte[\s\S]*StructMemberContext CalcType_[\s\S]*ByteConst 0[\s\S]*PopExecutionFlowIfNot[\s\S]*LocalVirtualFunction SearchDisease\n[^\n]*ByteConst 71/.test(forcedMissGate));
  const calculation=command.functions.CalcAilmentDamageHP;
  assert(/LocalVariable CalcType\n[^\n]*ByteConst 2\n[^\n]*JumpIfNot target=549/.test(calculation));
  // Offset 549 is the max-HP integer-percent branch, not normal damage variance.
  assert(calculation.includes('2675c Context'));
  const ratioCalculation=calculation.slice(calculation.indexOf('2675c Context'),calculation.indexOf('268ac PushExecutionFlow'));
  assert(/GetHP_HPMax_2[\s\S]*InvocationValue[\s\S]*Divide_IntInt[\s\S]*IntConst 100[\s\S]*Min/.test(ratioCalculation));
  const names={};
  for(const language of ['EN','ZH_CN']) {
    const file=path.join(s.assets.CONTENT,`GameText/Database/GameText${language}.uexp`);
    const b=fs.readFileSync(file),needle=Buffer.from(ability.DisplayName+'_Text\0');
    const at=b.indexOf(needle);assert(at>=0);
    const p=at+needle.length,n=b.readInt32LE(p);
    names[language]=n>0?b.toString('utf8',p+4,p+3+n):b.toString('utf16le',p+4,p+4+(-n-1)*2);
  }
  assert.equal(names.EN,'Pierce Through');assert.equal(names.ZH_CN,'穿心');
  return {
    sources:[abilityTable,ailmentTable].map(({source,sha256})=>({source,sha256})),
    ability,ratio,status,names,
    command:{asset:command.asset,sha256:command.sha256,forcedMissGate,ratioCalculation},
    conclusion:'ATTACK_HIT_FAILED gates CalcType 0 only. Pierce Through uses HP_RATIO_DAMAGE (CalcType 2), so it takes the ordinary accuracy check and successful damage reaction. Its 1% of 261 max HP truncates to 2 damage. Both scripted status applications still consume their CRT calls.'
  };
}
module.exports={auditPirroMercy};
if(require.main===module)console.log(JSON.stringify(auditPirroMercy(),null,2));
