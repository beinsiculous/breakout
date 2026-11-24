#!/usr/bin/env python3
"""Stage Shelf Breaker's exports with the shared MIDI Evil build pipeline."""
from pathlib import Path
import subprocess
import sys

source = Path(__file__).resolve().parent
subprocess.run(['python3', str(source.parents[2]/'insiculous_2d/scripts/build_music.py'),
                str(source/'shelf_breaker.evil'), 'ai_food_pyramid', *sys.argv[1:]], check=True)
