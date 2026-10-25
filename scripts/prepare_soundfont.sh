#!/usr/bin/env bash
# Build The Food Pyramid's bank, assets/soundfonts/food_pyramid.sf2, and the LICENSE beside
# it, through the engine's scripts/prepare_game_soundfont.sh: the FluidR3 GM presets the
# Triumphal March plays. The subset is committed; this script is how anyone rebuilds it byte
# for byte.
#
# Usage: scripts/prepare_soundfont.sh [--bank path/to/FluidR3_GM.sf2]
#   The engine's script documents the fetch, the sha256 check and the 4 MiB cap.
#
# The presets, as bank:program: trumpet (the fanfare, the stinger and the jam), trombone (the
# chords, and a foil brick's echo of the fanfare), tuba (the bass line a tong hit plays),
# timpani (the march's drums and the dirge). No strings and no kit: the march is brass and
# drums, and the bank stays near 2 MiB.
set -euo pipefail

GAME_DIR="$(cd "$(dirname "$0")/.." && pwd)"
ENGINE_DIR="$(cd "$GAME_DIR/../../insiculous_2d" && pwd)"
OUTPUT_DIR="$GAME_DIR/assets/soundfonts"

exec "$ENGINE_DIR/scripts/prepare_game_soundfont.sh" "$@" --max-mib 4 "$OUTPUT_DIR" food_pyramid 0:56 0:57 0:58 0:47
