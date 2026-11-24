#!/usr/bin/env python3
"""Author Shelf Breaker: original E-minor breakbeat, with independent game cues."""
import json
from pathlib import Path

SOURCE = Path(__file__).resolve().parent


def save(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


def note(key, tick, duration=120, velocity=96):
    return dict(pitch=key, start_tick=tick, duration_ticks=duration, velocity=velocity)


def main():
    specifications = [
        ('shelf_bass', 0, 'saw', 0.28, 0.22, True),
        ('rising_glass', 1, 'triangle', 0.23, 0.14, True),
        ('dust_chord', 2, 'square', 0.12, 0.20, True),
        ('break_kick', 0, 'sine', 0.80, 0.13, False),
        ('break_snare', 0, 'noise', 0.48, 0.09, False),
        ('shaker', 0, 'noise', 0.17, 0.025, False),
        ('brick_glass', 56, 'triangle', 0.85, 0.22, True),
        ('foil_crunch', 57, 'saw', 0.92, 0.13, True),
        ('tong_bounce', 58, 'sine', 0.85, 0.10, True),
        ('fallen_shelf', 47, 'triangle', 1.0, 0.30, True),
        ('tong_slide', 59, 'sine', 0.22, 0.032, True),
    ]
    instruments = []
    for index, (name, program, waveform, gain, decay, pitched) in enumerate(specifications):
        frequency = 110.0 if name == 'fallen_shelf' else 440.0
        oscillator = dict(waveform=waveform, start_frequency_hz=frequency if pitched else 180.0,
                          end_frequency_hz=frequency if pitched else 45.0, amplitude=1.0)
        patch = dict(name=name, sample_rate=32000, seed=7200+index, gain=gain,
                     duration_seconds=1.2, pitch_tracking=pitched, oscillators=[oscillator],
                     envelope=dict(attack_seconds=0.001, decay_seconds=decay,
                                   sustain_level=0.0, release_seconds=0.05))
        if name in ('shelf_bass', 'foil_crunch'):
            patch['filter'] = dict(cutoff_hz=900.0 if name == 'shelf_bass' else 2200.0,
                                   envelope_amount=1.5, resonance=0.2, stages=2)
        if name in ('rising_glass', 'brick_glass'):
            patch['oscillators'].append(dict(waveform='sine', start_frequency_hz=1320.0,
                                             end_frequency_hz=1320.0, amplitude=0.24))
        if name in ('break_snare', 'shaker'):
            patch['filter'] = dict(mode='high_pass', cutoff_hz=1200.0 if name == 'break_snare' else 6200.0, stages=1)
        save(SOURCE / 'patches' / f'{name}.json', patch)
        instruments.append(dict(name=name, source=dict(type='synth', patch=f'patches/{name}.json'),
                                midi_program=program, gain=1.0, pan=0.0))
    tracks = [dict(name=name, instrument=instrument, notes=[]) for name, instrument in
              [('Foundation', 0), ('Staircase', 1), ('Shelves', 2), ('Kick', 3), ('Snare', 4),
               ('Shaker', 5), ('melody', 6), ('bass', 8), ('dirge', 9), ('stinger', 6),
               ('jam', 6), ('movement', 10)]]
    # The hook climbs in clipped sixteenths, then answers with long descending thirds.
    hooks = [
        [(0,64,120),(120,67,120),(240,71,120),(480,74,300),(1080,71,120),(1320,67,360)],
        [(240,64,120),(360,67,120),(480,71,360),(1080,74,180),(1440,76,360)],
        [(0,79,180),(360,76,180),(720,74,360),(1320,71,120),(1560,67,240)],
        [(120,71,120),(240,74,120),(480,76,480),(1200,67,240),(1680,64,240)],
        [(0,76,120),(120,79,120),(240,83,120),(480,86,360),(1200,83,180),(1560,79,180)],
        [(240,74,180),(600,79,300),(1080,76,180),(1440,71,360)],
        [(0,83,240),(480,79,120),(600,76,120),(960,74,360),(1560,71,240)],
        [(0,67,120),(120,71,120),(240,74,120),(480,76,600),(1320,71,240),(1680,64,240)],
    ]
    for bar in range(32):
        start = bar * 1920
        reduced = 16 <= bar < 24
        for offset, key, duration in [(0,40,240),(600,47,120),(840,40,120),(1200,50,180),(1560,43,120),(1800,47,120)]:
            if not reduced or offset in (0,1200):
                tracks[0]['notes'].append(note(key,start+offset,duration,108 if offset==0 else 88))
        if not reduced or bar % 2 == 0:
            for offset, key, duration in hooks[bar % 4 + (4 if 8 <= bar < 16 else 0)]:
                tracks[1]['notes'].append(note(key,start+offset,duration,92 if bar>=24 else 80))
        for offset in ([960] if reduced else [360,1320]):
            for key in [52,59,62]:
                tracks[2]['notes'].append(note(key,start+offset,240,60))
        for offset in ([0,1200] if reduced else [0,360,840,1200,1680]):
            tracks[3]['notes'].append(note(36,start+offset,90,116 if offset in (0,1200) else 85))
        for offset in [480,1440]:
            tracks[4]['notes'].append(note(38,start+offset,90,115))
        if not reduced:
            for offset in [660,1740]:
                tracks[4]['notes'].append(note(38,start+offset,45,48))
        for step in range(16):
            if (not reduced and step % 4 != 3) or step in (2,6,10,14):
                tracks[5]['notes'].append(note(42,start+step*120,35,68 if step%2==0 else 42))
        if bar % 8 == 7:
            for offset in [1560,1680,1800,1860]:
                tracks[4]['notes'].append(note(38,start+offset,45,65+(offset-1560)//10))
        for step in range(8):
            tracks[7]['notes'].append(note([52,55,59,62][(step+bar)%4],start+step*240,240,105))
    tracks[0]['notes'].append(note(40,61320,120,110))
    for index, key in enumerate([76,79,83,86,83,79,74,76]):
        tracks[6]['notes'].append(note(key,index*240,96,112))
    for key in [40,47]:
        tracks[8]['notes'].append(note(key,0,360,110))
    for key in [76,79,83,86]:
        tracks[9]['notes'].append(note(key,0,480,112))
    for index, key in enumerate([71,74,79,76]):
        tracks[10]['notes'].append(note(key,index*240,100,90))
    for index, key in enumerate([76,79,83,74]):
        tracks[11]['notes'].append(note(key,index*240,35,75))
    project = dict(format='midi-evil-project',version=2,name='Shelf Breaker',tempo_bpm=140,
                   beats_per_bar=4,beat_unit=4,ticks_per_quarter=480,sample_rate=32000,
                   master_gain=0.8,instruments=instruments,tracks=tracks)
    (SOURCE/'shelf_breaker.evil').write_text(json.dumps(project,separators=(',',':'))+'\n')
    presets, renders = [], []
    for index, (name, program, _, _, _, pitched) in enumerate(specifications):
        keys = sorted({entry['pitch'] for track in tracks if track['instrument']==index for entry in track['notes']})
        if name == 'foil_crunch':
            keys = sorted({entry['pitch'] for entry in tracks[6]['notes']})
        zones = []
        for key in keys:
            filename = f'rendered/{name}_{key}.wav'
            renders.append(dict(instrument=index,key=key,ticks=960,wav=filename))
            zones.append(dict(wav=filename,keys=[key,key],root=key,release_seconds=0.05,longest_note_seconds=0.54))
        if pitched:
            presets.append(dict(name=name,bank=0,program=program,zones=zones))
        else:
            percussion = next((preset for preset in presets if preset['bank']==128),None)
            if percussion is None:
                percussion = dict(name='Shelf drums',bank=128,program=0,zones=[])
                presets.append(percussion)
            percussion['zones'].extend(zones)
    save(SOURCE/'bank.json',dict(version=1,name='Shelf Breaker',copyright='Original AI synthesis; Be Insiculous; CC0-1.0',presets=presets))
    save(SOURCE/'render.json',dict(seed=7200,sample_rate=32000,velocity=127,gain=0.8,
                                  backing_pitch_classes=[2,4,7,11],cue_pitch_classes=[2,4,7,11],renders=renders))


if __name__ == '__main__':
    main()
