# Shelf Breaker

Original AI-authored E-minor breakbeat for The Food Pyramid, CC0-1.0. No borrowed
melodies or recordings. The climbing sixteenth-note hook and descending answers,
broken kick pattern, ghost snares and granular shaker rhythm are specific to this
score. Bars 1–8 introduce, 9–16 extend into the upper register, 17–24 reduce the
texture and alternate hook phrases with space, 25–32 reprise with fills. Tempo is
140 BPM, 4/4, 480 PPQ, 32 bars / 61440 ticks / 54.857 seconds. Background last note-off
is exactly the loop boundary. E/G/B/D permits every cue/backing combination without
semitones or tritones. Background track names do not select any cue part.

`shelf_breaker.evil` is editable in MIDI Evil; `compose.py` recreates it and the eleven
original patches, replacing manual source edits deliberately. `render.json` specifies
seed base 7200 (instrument index added), 32000 Hz, velocity 127, gain 0.8 and each
960-tick render gate. Samples have 50 ms release, exact-key zones and no looping or
transposition. The longest authored note is 600 ticks (0.536 s), below the 0.54-second
zone budget. Rendered samples last 0.907 seconds; no sample is truncated to fit.

Brick/armor walk one melodic part on separate glass and filtered saw patches (programs
56/57). Paddle uses program 58 and a positional bass tiled with contiguous 240-tick
notes across the whole loop. Loss is a low two-note chord (program 47); clear is a
bright four-note chord held 480 ticks on program 56. Menu uses that same glass slot;
movement has its own independent D/E/G/B-fragment line on quiet program 59. All are
quantize Off. Legacy instrument IDs stay compatible; their former orchestral sounds
are replaced. Five triggered slots and one layer per note stay below eight slots and
six layers; each SoundFont slot has the engine's 24-voice budget.

## Rebuild and install

From games/breakout, with the working-set sibling layout:

```sh
python3 music-source/compose.py
python3 music-source/build.py /tmp/breakout-build-a
python3 music-source/build.py /tmp/breakout-build-b
cmp /tmp/breakout-build-a/ai_food_pyramid.mid /tmp/breakout-build-b/ai_food_pyramid.mid
cmp /tmp/breakout-build-a/ai_food_pyramid.sf2 /tmp/breakout-build-b/ai_food_pyramid.sf2
cp /tmp/breakout-build-a/ai_food_pyramid.mid assets/ai/music/ai_food_pyramid.mid
cp /tmp/breakout-build-a/ai_food_pyramid.sf2 assets/soundfonts/ai_food_pyramid.sf2
cargo run --example install_music
```

`build.py` and `scripts/prepare_soundfont.sh` are thin wrappers for engine
`scripts/build_music.py PROJECT.evil OUTPUT_STEM [STAGING_DIRECTORY]`. Default output
is ignored `music-source/rendered/`. Keep the previous `.mid.ron` when exporting;
`install_music` changes bank/provenance and adds movement only through SongDocument
apply/save, with editor as a development dependency. Never serialize the sidecar or
fingerprint manually. Validate the resulting closure through the engine audition.
Staging failures never install assets. No source WAV/project is shipped at runtime.

Toolchain: Rust 1.94.0, Python 3.12.3, x86_64 Linux; Cargo.lock pins dependencies.
MIDI Evil exporter baseline 46ae908; engine packer/audition baseline 3623e73 plus the
shared Python builder in this batch. Same-platform fresh builds must be byte-identical;
cross-platform floating-point identity is not promised. Runtime uses only committed
MIDI, engine sidecar and bank; it does not need MIDI Evil or Python.

## Auditions

```sh
python3 music-source/preview.py
```

The real engine conductor, committed bank, unity runtime buses and production limiter
render 111-second `idle.wav`, `sparse.wav`, `dense.wav` in ignored `previews/`, over two
complete wraps. Sparse and dense exercise all seven bindings; dense adds two tong
sources every 300 ms to 20 brick/armor events per second with recovery gaps, alternating
paddle sources and periodic loss/clear/menu. The short event TSVs remain editable sources; preview.py regenerates the ignored dense TSV.

Eight-second `demo-{brick,armor,paddle,loss,clear,move}.wav` files mix each action with
the same backing. `isolated-*.wav` subtracts the identical idle engine PCM, refusing
if either limiter operated. This is actual-engine signal isolation, with at most one
PCM16 step of quantization difference. Logs report peak/RMS and the first 100 ms of
cue energy against backing at the same event times, making the quieter movement
level explicit. The demos are synthetic event mixes, not gameplay recordings;
perceptual and browser approval remain the planner/user checkpoint.

At dense recipe rates, brick's 85.7 ms gate plus 50 ms release overlaps at most two
successive notes; a clear adds four and menu one in the same slot (seven total).
Armor overlaps at most two, paddle one, loss two, movement two. All are comfortably
below 24 voices per slot; background's measured peak is six voices. No limiting is
needed. The bank stays below 4 MiB and the score below 256 KiB.

Shared build helper revision: engine `7c8fed0`; native MIDI Evil synthesis revalidated
with `3d061af`. Runtime assets remain byte-identical after the helper validation fixes.
