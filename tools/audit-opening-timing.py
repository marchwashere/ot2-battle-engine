"""Read duration metadata only. Never decode/play media or sample CRI RNG."""
import hashlib
import json
import struct
import sys
from pathlib import Path

repo = Path(__file__).resolve().parents[1]
root = repo.parent
sys.path.insert(0, str(root / 'tools'))
sys.dont_write_bytecode = True
from inspect_boost_audio import utf

schedule = json.loads((repo / 'data/source/opening_crt.json').read_text())['schedules']
actors = [
    ('throne', 'Battletou', 'TOU', 'VO_TOU_PC_BT_00060', 'NORMAL_ATTACK_DAGGER_1'),
    ('pirro', 'Battlenpc', 'NPC', 'VO_TOU_NPC01_BT_00060', 'NORMAL_ATTACK_DAGGER_1'),
    ('scaracci', 'Battlenpc', 'NPC', 'VO_TOU_NPC02_BT_00060', 'NORMAL_ATTACK_AXE_1'),
]
result = []
for actor, folder, sheet, cue_name, ability in actors:
    source = f'Sound/Resource/VO/{folder}/en/CueSheet_VO_Battle{sheet}_en.uexp'
    buf = (root / 'Octopath_Traveler2/Content' / source).read_bytes()
    acb = utf(buf, buf.index(b'@UTF'))[0]
    tables = {k: utf(v) for k, v in acb.items() if isinstance(v, bytes) and v.startswith(b'@UTF')}
    name = next(r for r in tables['CueNameTable'] if r['CueName'] == cue_name)
    cue = tables['CueTable'][name['CueIndex']]
    assert cue['ReferenceType'] == 3
    sequence = tables['SequenceTable'][cue['ReferenceIndex']]
    assert sequence['Type'] == 4 and sequence['NumActionTracks'] == 0
    tracks = sequence['TrackIndex']
    waves = []
    for (index,) in struct.iter_unpack('>H', tracks):
        track = tables['TrackTable'][index]
        command = tables['TrackEventTable'][track['EventIndex']]['Command']
        assert len(command) == 10 and command[:3] == bytes.fromhex('07d004') and command[7:] == bytes(3)
        ref_type, synth_index = struct.unpack_from('>HH', command, 3)
        assert ref_type == 2
        synth = tables['SynthTable'][synth_index]
        wave_type, wave_index = struct.unpack('>HH', synth['ReferenceItems'])
        assert wave_type == 1
        wave = tables['WaveformTable'][wave_index]
        waves.append({'track': index, 'samples': wave['NumSamples'], 'sample_rate': wave['SamplingRate'],
                      'seconds': wave['NumSamples'] / wave['SamplingRate']})
    plan = next(p for p in schedule if p['ability'] == ability)
    voice_tick = next(e['tick'] for e in plan['timeline'] if e['kind'] == 'voice')
    hit_tick = next(e['tick'] for e in plan['timeline'] if e['kind'] == 'damage')
    elapsed = (hit_tick - voice_tick) / plan['ticks_per_second'] / 2
    shortest = min(w['seconds'] for w in waves)
    assert shortest > elapsed + 1 / 30, 'Timing boundary requires finer scheduling'
    result.append({'actor': actor, 'cue': cue_name, 'source': source,
                   'sha256': hashlib.sha256(buf).hexdigest(), 'variants': waves,
                   'attack_to_hit_seconds_2x': elapsed, 'shortest_clip_seconds': shortest})
out = {'language': 'English', 'battle_speed_at_first_action': 2,
       'scope': 'Uninterrupted ordinary opening actions; all variants outlast the hit. No CRI draws needed.',
       'actors': result}
(repo / 'data/source/opening_timing.json').write_text(json.dumps(out, indent=2) + '\n')
print('All English variants remain active at the unboosted hit at 2x, with >1/30 s margin.')
