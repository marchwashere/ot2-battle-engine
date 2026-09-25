'use strict';
// Import only scalar gameplay/CRT data. No media, renderer, or Unreal runtime.
const fs=require('node:fs'),path=require('node:path'),s=require('./source.cjs');
const {objectHeader}=require(path.join(s.root,'OT2BattleEngine/tools/properties.cjs'));
const {audit}=require('./audit-sequence.cjs');
const sources=[];
function table(name){const t=s.table(name);sources.push({source:t.source,sha256:t.sha256});return t.rows;}
const voices=table('Character/Database/BattleVoiceSet');
const types={eBATTLE_START:1,eBATTLE_VICTORY:2,eORDER_START:3,eTAKE_DAMAGE:4,eON_DEAD:5,eNORMAL_ATTACK:6,eNORMAL_ATTACK_LAST_HIT:7,eON_BOOST:8,eOPEN_WEAKNESS:9,eBREAK_ENEMY:10,eON_AVOID:11,eKILL_ENEMY:12,ePLAY_GUARD:17};
const raw=[];
const sets=['THIEF','GUEST_006','GUEST_007'].map(owner=>{
  const begin=voices.findIndex(r=>r.id===owner+'_VOICE_DATA_START');
  const next=voices.findIndex((r,i)=>i>begin&&r.id.endsWith('_VOICE_DATA_START'));
  const rows=voices.slice(begin+1,next<0?voices.length:next).filter(r=>{
    const type=types[r.BattleVoiceType.split('::').at(-1)];
    if(!type)return false;
    if(r.EnemyGroupID!=='None'&&r.EnemyGroupID!=='ENG_EVE_THI_C01_010')return false;
    if([1,2].includes(type)&&(r.BossFlag||r.FieldCommandFlag))return false;
    if(r.EventFlag!=='None')throw new Error('Unresolved opening voice flag '+r.id);
    return true;
  }).sort((a,b)=>b.Priority-a.Priority);
  raw.push(...rows);
  return rows.map(r=>({id:r.id,type:types[r.BattleVoiceType.split('::').at(-1)],rate:r.VoiceRate,once:r.PlayOnce,border:r.Border,self_only:r.TargetSelf,target:r.TargetID==='None'?null:r.TargetID,counter:r.CounterFlag,decondition:r.DiseaseCategory.endsWith('::eDECONDITION'),pinch:r.PinchFlag,just_break:r.JustBreakFlag}));
});
const abilities=table('Ability/Database/AbilityData');
const abilitySets=table('Ability/Database/AbilitySetData');
function program(row,count){
  const asset=row.ActionCommandSequencer.asset.text.replace(/^\/Game\//,'').split('.')[0];
  const file=path.join(s.assets.CONTENT,asset+'.uasset'),p=s.assets.loadPackage(file);
  sources.push({source:'/Game/'+asset,sha256:s.assets.hash(p.b)});
  const movie=s.fields(objectHeader(p,p.exports.find(e=>e.name==='MovieScene_0')).properties);
  const marks=new Map((movie.MarkedFrames||[]).map(m=>[m.Label,m.FrameNumber.value]));
  const events=audit(asset),steps=[],timeline=[]; let counter=0,clock=0,position=0;
  for(let i=0,guard=0;i<events.length;i++){
    if(++guard>200)throw new Error('Sequence loop '+asset);
    const e=events[i],fn=/LocalVirtualFunction (\S+)/.exec(e.body)?.[1];
    clock+=e.time-position;position=e.time;
    if(fn==='ExecStepCounter'){counter++;steps.push({kind:'check'});timeline.push({tick:clock,kind:'check'});}
    else if(fn==='PlayVoice'){
      const type=Number(/ByteConst (\d+)/.exec(e.body)[1]);
      steps.push({kind:'voice',type});
      timeline.push({tick:clock,kind:'voice',type});
    }else if(fn==='ExecAilment'){
      const args=[...e.body.matchAll(/\b(True|False)\b/g)].map(m=>m[1]==='True');
      steps.push({kind:'damage',last:args.at(-1)});
      timeline.push({tick:clock,kind:'damage',last:args.at(-1)});
    }else if(fn==='JumpToMarker'||fn==='CheckCounterToMarker'&&counter>=count){
      const label=JSON.parse(/(?:UnicodeStringConst|StringConst) (".*")/.exec(e.body)[1]);
      const dest=marks.get(label);if(dest===undefined)throw new Error('Unknown marker '+label);
      position=dest;
      const next=events.findIndex(e=>e.time>=dest);
      if(next<0)break;
      i=next-1;
    }else if(fn&&!['CheckCounterToMarker','PlayAction','PlaySE','PlayVibration','SetMovement','SetForceMovement','SetRelativePosition','PlayWeaponAttack','PlayActionWithDuration','SetCharacterDirection'].includes(fn)){
      throw new Error('Unreviewed sequence event '+fn+' in '+asset);
    }
  }
  if(steps.filter(s=>s.kind==='damage').length!==count)throw new Error('Unexpected hit count '+asset);
  return {steps,timeline,ticks_per_second:24000,asset};
}
const attacks={}; const schedules=[];
for(const weapon of ['sword','dagger','axe']){
  const set=abilitySets.find(r=>r.id==='PLAYER_ATTACK_'+weapon.toUpperCase());
  attacks[weapon]=[set.NoBoost,set.BoostLv1,set.BoostLv2,set.BoostLv3].map((id,i)=>{
    const p=program(abilities.find(r=>r.id===id),i+1);schedules.push({ability:id,...p});return p.steps;
  });
}
const level=table('Level/Database/LevelTable').find(r=>r.id==='Dng_Cty_3_1');
const group=table('Battle/Database/EnemyGroupData').find(r=>r.id==='ENG_EVE_THI_C01_010');
const enemy=table('Character/Database/EnemyDB').find(r=>r.id==='ENE_EVE_THI_CTY_010');
const output={voices:sets,attacks,encounter:{min:level.EncountStepMin,range_a:level.RandomStepA,range_b:level.RandomStepB,first_ratio:level.FirstEncountStepRatio},drop_rate:enemy.DropProbability,victory_voice:group.UseVictoryAction};
fs.writeFileSync(path.join(__dirname,'../data/opening_crt.json'),JSON.stringify(output,null,2)+'\n');
fs.writeFileSync(path.join(__dirname,'../data/source/opening_crt.json'),JSON.stringify({sources,rows:{voices:raw,level,group,enemy},schedules},null,2)+'\n');
console.log('Imported opening CRT candidates, weapon schedules, drops and encounter-counter parameters.');
