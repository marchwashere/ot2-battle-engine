'use strict';
// Scalar projection of FinishCallbackLoaded, not a graphics runtime.
const s=require('./source.cjs'),path=require('node:path'),assert=require('node:assert/strict');
const {objectHeader}=require('../../OT2BattleEngine/tools/properties.cjs');
let tables;
function initialization(enemy,npcActionLabel=null,direction=3) {
  tables??={resources:s.table('Character/Database/CharacterResourceDB'),actions:s.table('Character/Database/CharactersAction')};
  const label=npcActionLabel??tables.resources.rows.find(r=>r.id===enemy.ResourceLabel)?.ActionLabel;
  assert(label,`Unresolved initial action table for ${enemy.id}`);
  const action=tables.actions.rows.find(r=>r.id===label);
  assert(action,`Missing action table ${label}`);
  // Default PlayBattleIdleType=BAT_IDLE_ENEMY (130), SetupActionTable direction=3.
  assert(direction===2||direction===3);
  let reference=(direction===2?action.LeActionList:action.RiActionList)[130].asset.text;
  // Native GetActionFlipbookName 140e58920 retries the opposite horizontal
  // direction and mirrors it when the requested direction has no asset.
  if(reference==='None')reference=(direction===2?action.RiActionList:action.LeActionList)[130].asset.text;
  let frames=0;
  if(reference!=='None') {
    const asset=s.assets.loadPackage(path.join(s.assets.CONTENT,reference.slice(6).split('.')[0]+'.uasset'));
    const row=s.fields(objectHeader(asset,asset.exports[0]).properties);
    assert(Array.isArray(row.KeyFrames),`Not a supported initial flipbook: ${reference}`);
    frames=row.KeyFrames.reduce((n,f)=>n+f.FrameRun,0);
    assert(Number.isSafeInteger(frames)&&frames>=0);
  }
  return {initial_idle_frames:frames,sync_to_parent:enemy.DeadType==='EENEMY_DEAD_TYPE::eDESTRUCTION',sync_children:enemy.MainEnemy};
}
module.exports={initialization};
