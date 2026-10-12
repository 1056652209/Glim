#!/usr/bin/env python3
"""Sign nested executables/bundles inside-out; preserve framework symlinks."""
import os
from pathlib import Path
import subprocess
import sys

bundle = Path(sys.argv[1]).resolve()
identity = os.environ.get('GLIM_CODESIGN_IDENTITY', '-')
options = ['--force', '--sign', identity]
options += ['--timestamp=none'] if identity == '-' else ['--timestamp', '--options', 'runtime']
paths = list(bundle.rglob('*'))
for path in paths:
    if path.is_file() and not path.is_symlink():
        description = subprocess.check_output(['file', '-b', str(path)], text=True)
        if 'Mach-O' in description:
            subprocess.run(['codesign', *options, str(path)], check=True)
containers = [p for p in paths if p.is_dir() and not p.is_symlink() and p.suffix in ('.app', '.xpc', '.framework')]
for path in sorted(containers, key=lambda p: len(p.parts), reverse=True) + [bundle]:
    subprocess.run(['codesign', *options, str(path)], check=True)
subprocess.run(['codesign', '--verify', '--deep', '--strict', str(bundle)], check=True)
