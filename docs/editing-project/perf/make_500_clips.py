#!/usr/bin/env python3
"""Write a Dawesome project with 500 clips, for the frame-rate check (acceptance O1).

Run it from anywhere:

    python3 docs/editing-project/perf/make_500_clips.py

It makes a folder `dawesome-500-clips` in your temporary folder (or the folder
you give as the first argument) holding `500-clips.dawesome` and copies of four
of the app's own samples, so the project opens on its own. Open it with
File > Open. The frame-rate readout is the playlist's debug box, which is off
by default: set `debugMode: true` in `staticDefaults` near the top of
src/views/playlist/sketch/index.ts while you check, then try an edge drag, a
cut hover and a Shift+drag lasso.

400 of the clips are sample clips on ten tracks across all 64 bars, about one
in four of them trimmed; 100 are clips of two patterns.
"""
import json
import os
import shutil
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ASSETS = os.path.normpath(os.path.join(HERE, '..', '..', '..', 'engine', 'assets'))
SAMPLES = ['assets_66-bd-01.wav', 'assets_66-sd-01.wav', 'assets_66-hh-01-or.wav', 'clap.wav']


def main():
    parent = sys.argv[1] if len(sys.argv) > 1 else tempfile.gettempdir()
    folder = os.path.join(parent, 'dawesome-500-clips')
    os.makedirs(os.path.join(folder, 'samples'), exist_ok=True)
    for name in SAMPLES:
        shutil.copyfile(os.path.join(ASSETS, name), os.path.join(folder, 'samples', name))

    clips = []
    for n in range(400):
        clip = {
            # (relative to the project's folder)
            'path': 'samples/' + SAMPLES[n % len(SAMPLES)],
            'trackNumber': n % 10,
            # 40 columns of ten clips, 6.25 beats apart, staggered down the tracks
            'startBeat': (n // 10) * 6.25 + (n % 10) * 0.5,
        }
        if n % 4 == 0:
            clip['window'] = {'start': 0.01, 'end': 0.06}
        if n % 7 == 0:
            clip['settings'] = {'gainDb': -3, 'pan': 0.2, 'pitch': -2, 'attackMs': 5, 'decayMs': 10}
        clips.append(clip)

    pattern_clips = [
        {'patternId': n % 2, 'trackNumber': 10 + n % 2, 'startBeat': (n // 2) * 5.0}
        for n in range(100)
    ]
    project = {
        # version 3: some clips are trimmed
        'version': 3,
        'tempo': 120,
        'clips': clips,
        'sequencer': {
            'rows': [
                {'id': 0, 'path': 'samples/' + SAMPLES[0]},
                {'id': 1, 'path': 'samples/' + SAMPLES[1]},
                {'id': 2, 'path': 'samples/' + SAMPLES[2]},
            ],
            'patterns': [
                {'id': 0, 'name': 'beat', 'length': 16,
                 'steps': {'0': [0, 8, 10], '1': [4, 12], '2': [0, 2, 4, 6, 8, 10, 12, 14]}},
                {'id': 1, 'name': 'fill', 'length': 16,
                 'steps': {'0': [0, 3, 6], '1': [8, 10, 12, 13, 14, 15]}},
            ],
        },
        'patternClips': pattern_clips,
    }

    path = os.path.join(folder, '500-clips.dawesome')
    with open(path, 'w') as out:
        json.dump(project, out, indent=1)
    print(path)


if __name__ == '__main__':
    main()
