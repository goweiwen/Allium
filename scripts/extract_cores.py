#!/usr/bin/env python

import sys
import toml
import re
import os
import json
from os.path import dirname, basename, join

extensions = {}
cores = {}
consoles = {}
folders = {}

BLACKLIST_EXTENSIONS = ["", "bin", "rom", "m3u", "cue", "iso", "img", "chd", "ccd", "zip", "7z", "dsk", "cas", "mx1", "mx2", "miyoocmd", "bs", "dmg", "fig", "tap"]

def canonicalize_console_name(name):
    CANONICAL_CONSOLE_NAME = {
        "Nintendo - GB": "Nintendo - Game Boy",
        "Nintendo - GBC": "Nintendo - Game Boy Color",
        "Nintendo - GBA": "Nintendo - Game Boy Advance",
        "Nintendo - Super Game Boy": "Nintendo - Game Boy Color",
        ".Java - J2ME": "Java - J2ME",
    }
    return CANONICAL_CONSOLE_NAME[name] if name in CANONICAL_CONSOLE_NAME else name

def extract_file(path):
    with open(path, 'r') as f:
        data = json.load(f)

    launch_path = join(dirname(path), "launch.sh")
    with open(launch_path, 'r') as f:
        launch = f.read().strip()
    match = re.search(".retroarch/cores/(.+)_libretro.so", launch)
    if match is None:
        core = None
    else:
        core = match.group(1)

    name = basename(dirname(dirname(dirname(path))))
    console = canonicalize_console_name(name.rsplit("(", 1)[0].strip())
    folder = basename(dirname(path))
    extensions = [ext for ext in data["extlist"].split("|") if ext not in BLACKLIST_EXTENSIONS]

    return {
        "name": name,
        "console": console,
        "core": core,
        "folder": folder,
        "extensions": extensions,
    }

def extract_directory(directory, whitelist = None):
    for root, dirs, files in os.walk(directory):
        for file in files:
            if file == "config.json":
                path = os.path.join(root, file)
                console = extract_file(path)
                if whitelist is None or console['name'] in whitelist:
                    print(console['console'])
                    if console['console'] in consoles:
                        if console['core'] is not None:
                            consoles[console['console']]['cores'].append(console['core'])
                        if console['folder'] not in consoles[console['console']]['folders']:
                            consoles[console['console']]['folders'].append(console['folder'])
                        for ext in console['extensions']:
                            if ext not in consoles[console['console']]['extensions']:
                                consoles[console['console']]['extensions'].append(ext)
                    else:
                        consoles[console['console']] = {
                            "cores": [console['core']],
                            "folders": [console['folder']],
                            "extensions": console['extensions'],
                        }

def extract():
    extract_directory("../../Onion/static/packages/Emu")
    extract_directory("../../Onion/static/packages/Rapp", [
        ".Java - J2ME (SquirrelJME)",
        "Game engien - EasyRPG",
        "Nintendo - GBA (gpSP)",
        "Nintendo - GB (TGB Dual)",
        "Nintendo - NES (Nestopia)",
        "Nintendo - SNES (Snes9x)",
        "NEC - PC-98 (Neko Project II Kai)",
        "NEC - PC-FX (Mednafen PC-FX)",
    ])

    with open("consoles.toml", "w") as f:
        f.write(toml.dumps(consoles))
    
    print("Written to consoles.toml")

def check():
    with open("consoles.toml", "r") as f:
        consoles = toml.loads(f.read())

    # Check that extensions are not duplicated
    for (name, console) in consoles.items():
        for extension in console['extensions']:
            if extension in extensions:
                extensions[extension].append(name)
            else:
                extensions[extension] = [name]
    for (extension, names) in extensions.items():
        if len(names) > 1:
            print("Duplicate extension: " + extension + "\n- " + "\n- ".join(names) + "\n")

    # Check that folders are not duplicated
    for (name, console) in consoles.items():
        for folder in console['folders']:
            if folder in folders:
                folders[folder].append(name)
            else:
                folders[folder] = [name]
    for (folder, names) in folders.items():
        if len(names) > 1:
            print("Duplicate folder: " + folder + "\n" + "\n- ".join(names) + "\n")

def main():
    if len(sys.argv) < 2:
        print("Usage: extract_cores.py extract|check")
        return
    
    if sys.argv[1] == "extract":
        extract()
    elif sys.argv[1] == "check":
        check()
    else:
        print("Usage: extract_cores.py extract|check")
        return

if __name__ == "__main__":
    main()
def __main__():
    pass
