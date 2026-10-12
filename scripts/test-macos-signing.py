#!/usr/bin/env python3
"""Ensure unsigned Intel bundles and nested Sparkle components sign inside-out."""
import os
from pathlib import Path
import plistlib
import subprocess
import tempfile

with tempfile.TemporaryDirectory(prefix='glim-signing-test-') as tmp:
    root = Path(tmp)
    source = root / 'main.c'
    source.write_text('int main(void) { return 0; }\n')
    bundle = root / 'Glim.app'
    helper = bundle / 'Contents/Helpers/GlimRenderer.app'
    for app, name in [(bundle, 'glim'), (helper, 'glim-renderer')]:
        executable = app / 'Contents/MacOS' / name
        executable.parent.mkdir(parents=True)
        (app / 'Contents/Info.plist').write_bytes(plistlib.dumps({
            'CFBundleExecutable': name,
            'CFBundleIdentifier': f'io.github.neopanic.signing-test.{name}',
            'CFBundlePackageType': 'APPL',
            'CFBundleVersion': '1',
        }))
        subprocess.run(['xcrun', 'clang', '-target', 'x86_64-apple-macos11.0',
                        '-Wl,-no_adhoc_codesign', str(source), '-o', str(executable)],
                       check=True, capture_output=True)
        assert subprocess.run(['codesign', '--verify', str(executable)], capture_output=True).returncode != 0
    sdk = Path(os.environ.get('GLIM_SPARKLE_DIR', 'target/sparkle/2.10.0'))
    framework = bundle / 'Contents/Frameworks/Sparkle.framework'
    subprocess.run(['ditto', str(sdk / 'Sparkle.framework'), str(framework)], check=True)
    env = os.environ.copy()
    env['GLIM_CODESIGN_IDENTITY'] = '-'
    subprocess.run(['python3', str(Path(__file__).with_name('sign-macos.py')), str(bundle)],
                   env=env, check=True)
    for app in (helper, bundle):
        subprocess.run(['codesign', '--verify', '--deep', '--strict', str(app)], check=True)
print('Signing: unsigned Intel app, helper and nested Sparkle components verified.')
