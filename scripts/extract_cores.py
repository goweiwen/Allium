#!/usr/bin/env python

import toml
import re
import os
import json
from os.path import dirname, basename, join

BLACKLIST_EXTENSIONS = ["bin", "rom", "m3u", "cue", "iso", "img", "chd", "ccd", "zip", "7z"]

def extract_file(path):
    with open(path, 'r') as f:
        data = json.load(f)

    launch_path = join(dirname(path), "launch.sh")
    with open(launch_path, 'r') as f:
        launch = f.read().strip()
    match = re.search(".retroarch/cores/(.+)_libretro.so", launch)
    if match is None:
        core = "UNKNOWN"
    else:
        core = match.group(1)

    name = basename(dirname(dirname(dirname(path))))
    pattern = basename(dirname(path))
    label = data["label"]
    extlist = data["extlist"].split("|")

    patterns = [label]
    extensions = [ext for ext in extlist if ext not in BLACKLIST_EXTENSIONS and ext != ""]
    path = os.path.join("/mnt/SDCARD/.allium/cores/mgba", launch)

    data = {
        "patterns": [pattern],
        "extensions": extensions,
        "retroarch_core": core + '_libretro.so',
    }

    return (core, data, name)

def extract_directory(directory):
    cores = {}
    for root, dirs, files in os.walk(directory):
        for file in files:
            if file == "config.json":
                path = os.path.join(root, file)
                core, data, name = extract_file(path)
                if core in cores:
                    (other, names) = cores[core]
                    other["patterns"] += data["patterns"]
                    other["extensions"] += data["extensions"]
                    names.append(name)
                    if other["retroarch_core"] != data["retroarch_core"]:
                        raise Exception("Duplicate core with different retroarch_core: " + core)
                else:
                    cores[core] = (data, [name])

    for (core, (data, names)) in cores.items():
        for name in names:
            print("# " + name)
        print("[cores." + core + "]")
        print(toml.dumps(data))

extract_directory("../../Onion/static/packages/Emu")
