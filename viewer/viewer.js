'use strict';
const $ = id => document.getElementById(id);
const escapeHtml = value => String(value).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
const hex = value => '0x' + value.toString(16).toUpperCase().padStart(8, '0');
let result = null;
let running = false;
let stopped = false;
let previewVersion = 0;
let previewTimer;
const names = {attack_sword:'Sword attack',attack_dagger:'Dagger attack',attack_axe:'Axe attack',defend:'Defend',ABI_ENE_0041:'Attack',ABI_ENE_0500:'Slash'};
const reasons = {actor_initialization:'Actor initialization',current_order_shuffle:'First order shuffle',battle_start_speaker:'Opening speaker',battle_start_voice:'Opening voice check',next_order_shuffle:'Next order shuffle',order_voice:'Turn-start voice check',boost_voice:'Boost voice check',normal_accuracy:'Cached accuracy check',attack_voice:'Attack voice check',last_attack_voice:'Final-hit voice check',critical_hit_and_damage:'Critical / hit / damage',damage_notification_timer:'Damage notification timer',miss_notification_timer:'Miss notification timer',guard_disease:'Guard status checks',guard_voice:'Guard voice check',round_timing_revision:'Round timing revision',enemy_ai_and_target:'Enemy AI and target',enemy_drop:'Enemy item drop',field_encounter_reset_b_then_a:'Area counter reset (B then A)'};
const actorName = id => result.actors[id]?.name ?? '—';
const shortActorName = id => actorName(id).split(' · ')[0];
const safeName = id => escapeHtml(actorName(id));
function showError(message) { $('error').textContent = message; $('error').hidden = false; }

function validateSeed(text) {
  if (!/^(?:[0-9]+|0x[0-9a-f]+)$/i.test(text)) throw new Error('Enter a decimal seed or 0x-prefixed hexadecimal seed.');
  const seed = Number(text);
  if (!Number.isInteger(seed) || seed < 0 || seed > 0xFFFFFFFF) throw new Error('Seed must be between 0 and 4294967295 (0xFFFFFFFF).');
  return seed;
}

function readSetup() {
  const seed = validateSeed($('seed').value.trim());
  const text = $('advances').value.trim();
  if (!/^[0-9]+$/.test(text)) throw new Error('Advances must be a nonnegative whole number.');
  const advances = BigInt(text);
  if (advances > 18446744073709551615n) throw new Error('Advance count exceeds 18446744073709551615.');
  return {seed,advances:advances.toString()};
}
const setupQuery = setup => `seed=${setup.seed}&advances=${setup.advances}`;
function renderSetup(setup) {
  $('prepared-state').textContent = setup.battle_seed;
  $('prepared-hex').textContent = hex(setup.battle_seed);
}
async function previewSetup() {
  const version = ++previewVersion;
  try {
    const setup = readSetup();
    const response = await fetch(`/api/crt?${setupQuery(setup)}`);
    const data = await response.json();
    if (!response.ok) throw new Error(data.error || 'Could not calculate CRT state.');
    if (version === previewVersion) renderSetup(data);
  } catch(error) {
    if (version === previewVersion) {
      $('prepared-state').textContent = '—'; $('prepared-hex').textContent = '';
      showError(error instanceof TypeError ? 'Cannot reach the local simulator.' : error.message);
    }
  }
}
function schedulePreview() {
  clearTimeout(previewTimer);
  ++previewVersion;
  $('error').hidden = true;
  $('prepared-state').textContent = '…'; $('prepared-hex').textContent = '';
  previewTimer = setTimeout(previewSetup,150);
}

async function simulate(event) {
  event?.preventDefault();
  if (running || stopped) return;
  $('error').hidden = true;
  let setup;
  clearTimeout(previewTimer);
  ++previewVersion;
  try { setup = readSetup(); }
  catch(error) { showError(error.message); $('prepared-state').textContent = '—'; $('prepared-hex').textContent = ''; return; }
  running = true;
  $('simulate').disabled = true;
  $('quit').disabled = true;
  $('seed').disabled = true;
  $('advances').disabled = true;
  $('simulate').textContent = 'Simulating…';
  $('status').textContent = `Running seed ${setup.seed} after ${setup.advances} advances…`;
  try {
    const response = await fetch(`/api/simulate?${setupQuery(setup)}`);
    const data = await response.json();
    if (!response.ok) throw new Error(data.error || 'Simulation failed.');
    result = data;
    renderSetup(data.crt_setup);
    render();
    $('status').textContent = `Completed: seed ${setup.seed} + ${setup.advances} advances → battle state ${data.initial_seed} (${hex(data.initial_seed)}).`;
  } catch(error) {
    showError(error instanceof TypeError ? 'Cannot reach the local simulator. Relaunch OT2 Battle Viewer and try again.' : error.message);
    $('status').textContent = result ? `Still displaying the previous result for seed ${result.initial_seed}.` : 'No result available.';
  } finally {
    running = false;
    $('simulate').disabled = false;
    $('quit').disabled = false;
    $('seed').disabled = false;
    $('advances').disabled = false;
    $('simulate').textContent = 'Run simulation ↗';
  }
}

function render() {
  const state = result.state;
  const after = state.opening_crt.after_battle;
  $('results').hidden = false;
  $('outcome').textContent = state.outcome === 'victory' ? 'Predicted victory' : `Predicted ${state.outcome}`;
  $('outcome').className = 'pill ' + (state.outcome === 'victory' ? 'success' : 'failure');
  const setup = result.crt_setup;
  $('result-context').textContent = `Seed ${setup.initial_seed} + ${setup.advances} advances · Start ${result.initial_seed} · Round ${state.round} · ${result.steps.length} actions`;
  $('final-seed').textContent = state.rng.state;
  $('final-hex').textContent = hex(state.rng.state);
  $('copy').textContent = 'Copy decimal';
  $('draws').textContent = state.rng.draws;
  $('draws-help').textContent = `${setup.advances} before battle · ${BigInt(setup.advances) + BigInt(state.rng.draws)} total from initial seed`;
  $('counter').textContent = after?.encounter_steps ?? '—';
  $('drops').textContent = after ? `${after.healing_grapes} Healing Grape${after.healing_grapes === 1 ? '' : 's'} dropped` : 'No victory rewards';
  const checkpoints = [
    ['Before initialization', {state:result.initial_seed,draws:0}],
    ['Before first action', result.initial_state.rng],
    ...(after ? [['Fight ended',after.rng_before_rewards],['After drops',after.rng_after_rewards],['After area reset',after.rng_after_field_reset]] : [['Battle ended',state.rng]])
  ];
  $('checkpoints').innerHTML = checkpoints.map(([label,rng]) => `<tr><td>${label}</td><td>${rng.state}</td><td>${hex(rng.state)}</td><td>${rng.draws}</td></tr>`).join('');
  $('party').innerHTML = result.actors.map((meta,i) => {
    if (meta.side !== 'player') return '';
    const r = state.actors[i].resources;
    return `<div class="member"><div class="member-heading"><span>${escapeHtml(meta.name)}</span><span>${r.hp} / ${meta.max_hp} HP</span></div><progress max="${meta.max_hp}" value="${r.hp}" aria-label="${escapeHtml(meta.name)} HP"></progress><small>${r.sp} / ${meta.max_sp} SP · ${r.bp} BP</small></div>`;
  }).join('');
  $('enemy-outcome').textContent = result.actors.map((meta,i) => meta.side === 'enemy' ? `${shortActorName(i)}: ${state.actors[i].resources.hp} HP` : '').filter(Boolean).join(' · ');
  $('initial-order').textContent = result.initial_state.queue.map(shortActorName).join(' → ');
  $('log-count').textContent = `${result.steps.length} actions · ${state.rng.draws} CRT draws`;
  $('actions').innerHTML = result.steps.map(renderAction).join('');
  $('trace').innerHTML = state.opening_crt.trace.map(span => `<tr><td>${escapeHtml(reasons[span.reason] || span.reason.replaceAll('_',' '))}</td><td>${span.actor === null ? '—' : safeName(span.actor)}</td><td>${span.before.state}</td><td>${span.after.state}</td><td>+${span.after.draws-span.before.draws}</td><td>${span.after.draws}</td></tr>`).join('');
}

function renderAction(step, index) {
  const report = step.report;
  const events = report.events;
  const action = events.find(e => e.kind === 'action');
  const bp = events.find(e => e.kind === 'resource_changed' && e.resource === 'bp' && e.actor === report.actor);
  const hits = events.filter(e => e.kind === 'damage' || e.kind === 'miss');
  const damage = hits.filter(e => e.kind === 'damage').map(e => e.amount);
  const boost = bp ? bp.before-bp.after : 0;
  const description = (names[action?.name] || action?.name || 'Action') + (action?.name === 'defend' ? '' : ` · ${boost} BP`);
  const targets = [...new Set(hits.map(e => shortActorName(e.target)))].join(', ');
  const changes = events.map(e => {
    switch(e.kind) {
      case 'shield_changed': return `${actorName(e.actor)}: shields ${e.before} → ${e.after}`;
      case 'broken': return `${actorName(e.actor)} breaks; recovery would be in round ${e.recover_round}.`;
      case 'knocked_out': return `${actorName(e.actor)} is defeated.`;
      case 'resource_changed': return `${actorName(e.actor)}: ${e.resource.toUpperCase()} ${e.before} → ${e.after}`;
      case 'battle_ended': return `Battle ended: ${e.outcome}.`;
      default: return null;
    }
  }).filter(Boolean);
  const resources = step.actors_after.map((actor,i) => `<span><b>${safeName(i)}</b>${actor.resources.hp} HP · ${actor.resources.sp} SP · ${actor.resources.bp} BP${result.actors[i].side === 'enemy' ? ` · ${actor.shields} shield` : ''}</span>`).join('');
  return `<details class="action"><summary><span class="round">R${step.round} · #${index+1}</span><span><b>${safeName(report.actor)}</b><span class="sub">${escapeHtml(description)}</span></span><span class="rolls">${escapeHtml(targets || 'Self')}<span class="sub">${damage.length ? `${damage.join(' + ')} damage rolled` : hits.length ? 'Miss' : 'Guard applied'}</span></span><span class="action-rng">${report.rng_after.state}<span class="sub">CRT after · +${report.rng_after.draws-report.rng_before.draws} draws</span></span></summary><div class="action-body"><div class="hit-list">${hits.map((hit,i) => `<span class="hit"><strong>Hit ${i+1}</strong> ${hit.kind === 'miss' ? 'Miss' : `${hit.amount} · ${hit.broken_before ? 'Broken target' : hit.weak ? 'Weakness' : 'Neutral'}${hit.critical ? ' · Critical' : ''}`}</span>`).join('')}</div><p class="small muted">CRT ${report.rng_before.state} → ${report.rng_after.state} (${hex(report.rng_after.state)}). Includes transition to the next decision or the final field reset.</p>${changes.length ? `<ul class="event-list">${changes.map(text => `<li>${escapeHtml(text)}</li>`).join('')}</ul>` : ''}<div class="after-resources">${resources}</div></div></details>`;
}

function selectTab(selected) {
  for (const name of ['actions','trace']) {
    const active = selected === name;
    $(`${name}-tab`).setAttribute('aria-selected',String(active));
    $(`${name}-tab`).tabIndex = active ? 0 : -1;
    $(`${name}-view`).hidden = !active;
  }
}
for (const name of ['actions','trace']) {
  $(`${name}-tab`).addEventListener('click',() => selectTab(name));
  $(`${name}-tab`).addEventListener('keydown',event => {
    if (['ArrowLeft','ArrowRight','Home','End'].includes(event.key)) {
      event.preventDefault();
      const next = event.key === 'Home' ? 'actions' : event.key === 'End' ? 'trace' : name === 'actions' ? 'trace' : 'actions';
      selectTab(next); $(`${next}-tab`).focus();
    }
  });
}
$('copy').addEventListener('click',async () => {
  if (!result) return;
  try { await navigator.clipboard.writeText(String(result.state.rng.state)); $('copy').textContent = 'Copied'; }
  catch { showError('Clipboard unavailable. Select and copy the final CRT value above.'); }
});
$('export').addEventListener('click',() => {
  if (!result) return;
  const link = document.createElement('a');
  link.href = `/api/export?${setupQuery({seed:result.crt_setup.initial_seed,advances:result.crt_setup.advances})}`;
  link.download = result.crt_setup.advances === '0' ? `ot2-throne-seed-${result.initial_seed}.json` : `ot2-throne-seed-${result.crt_setup.initial_seed}-advances-${result.crt_setup.advances}.json`;
  document.body.append(link); link.click(); link.remove();
});
$('quit').addEventListener('click',async () => {
  if (running || stopped) return;
  $('quit').disabled = true;
  try {
    const response = await fetch('/api/shutdown',{method:'POST',headers:{'X-OT2-Viewer':'1'}});
    if (!response.ok) throw new Error('Could not stop the viewer.');
    stopped = true;
    clearTimeout(previewTimer); ++previewVersion;
    $('results').hidden = true; $('closed').hidden = false; $('seed-form').hidden = true;
    $('quit').textContent = 'Stopped'; $('status').textContent = 'Local simulator stopped.';
  } catch(error) { showError(error.message); $('quit').disabled = false; }
});
$('seed-form').addEventListener('submit',simulate);
$('seed').addEventListener('input',schedulePreview);
$('advances').addEventListener('input',schedulePreview);
simulate();
