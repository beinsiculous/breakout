#!/usr/bin/env python3
"""Audition the committed closure through the engine, including isolated feedback."""
import array
import math
from pathlib import Path
import subprocess
import wave

SOURCE = Path(__file__).resolve().parent
ENGINE = SOURCE.parents[2] / 'insiculous_2d'
PREVIEWS = SOURCE / 'previews'
ACTIONS = ['brick', 'armor', 'paddle', 'loss', 'clear', 'move']


def render(tool, recipe, seconds, name):
    with (PREVIEWS/f'{name}.log').open('w') as log:
        subprocess.run([str(tool), str(SOURCE.parent/'assets'), 'ai/music/ai_food_pyramid.mid',
                        str(SOURCE/f'{recipe}.tsv'), str(seconds), str(PREVIEWS/f'{name}.wav')],
                       stdout=log, stderr=subprocess.STDOUT, check=True)
    report = (PREVIEWS/f'{name}.log').read_text()
    for counter in ['clipped', 'limited_samples']:
        assert f'{counter}=0 ' in report, report
    for counter in ['non_finite_samples', 'commands_rejected', 'rejected_triggers',
                    'late_triggers', 'stale_triggers', 'silent_triggers']:
        assert f'{counter}: 0,' in report, report


def read_audio(name):
    with wave.open(str(PREVIEWS/f'{name}.wav'), 'rb') as recording:
        return recording.getparams(), array.array('h', recording.readframes(recording.getnframes()))


def rms(samples):
    return math.sqrt(sum((sample/32768)**2 for sample in samples)/len(samples))


def main():
    PREVIEWS.mkdir(exist_ok=True)
    dense = []
    for step in range(1100):
        second = step/10
        if step % 100 < 60:
            dense.extend([(second,'brick',3),(second,'armor',5)])
        if step % 3 == 0:
            dense.extend([(second,'move',1),(second,'move',2)])
        if step % 5 == 0:
            dense.append((second,'paddle',1 if step % 10 == 0 else 2))
        for period, action, source in [(43,'loss',3),(101,'clear',3),(77,'menu',4)]:
            if step % period == 0:
                dense.append((second,action,source))
    (SOURCE/'dense.tsv').write_text(''.join(f'{seconds:.3f} {action} {source}\n' for seconds,action,source in dense))
    subprocess.run(['cargo','build','--manifest-path',str(ENGINE/'Cargo.toml'),'-p','music_midi','--example','audition'],check=True)
    tool = ENGINE/'target/debug/examples/audition'
    for recipe in ['idle','sparse','dense']:
        render(tool,recipe,111,recipe)
    render(tool,'idle',8,'demo-idle')
    parameters, idle = read_audio('demo-idle')
    for action in ACTIONS:
        event_seconds = [float(event.split()[0]) for event in (SOURCE/f'{action}.tsv').read_text().splitlines()
                         if event.strip() and not event.lstrip().startswith('#')]
        if not event_seconds or any(not math.isfinite(second) or not 0 <= second <= 7.9 for second in event_seconds):
            raise ValueError(f'{action} demo needs events within 0–7.9 seconds for complete onset windows')
        render(tool,action,8,f'demo-{action}')
        _, mixed = read_audio(f'demo-{action}')
        # Subtraction is valid because render refuses all limiter activity above.
        isolated = array.array('h',(sample-background for sample,background in zip(mixed,idle)))
        peak = max(abs(sample) for sample in isolated)/32768
        assert 0 < rms(isolated) < peak < 0.8913
        with wave.open(str(PREVIEWS/f'isolated-{action}.wav'),'wb') as recording:
            recording.setparams(parameters)
            recording.writeframes(isolated.tobytes())
        cue_windows, backing_windows = [], []
        for second in event_seconds:
            start = int(second*parameters.framerate)*2
            end = start + int(0.1*parameters.framerate)*2
            cue_windows.extend(isolated[start:end])
            backing_windows.extend(idle[start:end])
        cue_level, backing_level = rms(cue_windows), rms(backing_windows)
        (PREVIEWS/f'isolated-{action}.log').write_text(
            f'peak={peak:.6f} rms={rms(isolated):.6f} onset_rms={cue_level:.6f} '
            f'backing_onset_rms={backing_level:.6f} cue_to_backing_db={20*math.log10(cue_level/backing_level):.2f} '
            'method=engine_mix_minus_identical_idle limiter_inactive=true\n')


if __name__ == '__main__':
    main()
