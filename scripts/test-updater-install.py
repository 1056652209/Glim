#!/usr/bin/env python3
"""Isolated signed 100 -> 101 upgrade; never touches /Applications or real signing keys."""
import functools
import http.server
import json
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import tempfile
import threading
import time
import uuid
import xml.etree.ElementTree as ET

root = Path(__file__).resolve().parent.parent
sdk = Path(os.environ.get('GLIM_SPARKLE_DIR', root / 'target/sparkle/2.10.0')).resolve()


def run(*args, **kwargs):
    return subprocess.run(list(map(str, args)), check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, **kwargs)


class QuietHandler(http.server.SimpleHTTPRequestHandler):
    def log_message(self, *args):
        pass


with tempfile.TemporaryDirectory(prefix='glim-update-install-') as directory:
    stage = Path(directory)
    serving = stage / 'feed'
    serving.mkdir()
    server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), functools.partial(QuietHandler, directory=str(serving)))
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    base = f'http://127.0.0.1:{server.server_port}'
    executable = stage / 'glim'
    run('xcrun', 'swiftc', '-swift-version', '5', '-DGLIM_UPDATER_TESTING', '-F', sdk,
        '-framework', 'Sparkle', '-Xlinker', '-rpath', '-Xlinker', '@executable_path/../Frameworks',
        root / 'native/Updater.swift', root / 'native/tests/UpdaterHost.swift', '-o', executable)
    # Key generation needs the framework before a bundle is staged.
    environment = dict(os.environ, DYLD_FRAMEWORK_PATH=str(sdk))
    run(executable, '--test-key', stage, env=environment)
    public = (stage / 'public-key').read_text()
    identity = 'io.github.neopanic.glim.updater-test.' + uuid.uuid4().hex
    marker = stage / 'success'

    def bundle_at(parent, build):
        bundle = parent / 'GlimUpdateTest.app'
        macos = bundle / 'Contents/MacOS'
        macos.mkdir(parents=True)
        shutil.copy2(executable, macos / 'glim')
        frameworks = bundle / 'Contents/Frameworks'
        frameworks.mkdir()
        run('ditto', sdk / 'Sparkle.framework', frameworks / 'Sparkle.framework')
        info = {
            'CFBundleIdentifier': identity, 'CFBundleExecutable': 'glim', 'CFBundleName': 'GlimUpdateTest',
            'CFBundlePackageType': 'APPL', 'CFBundleVersion': str(build), 'CFBundleShortVersionString': f'1.0.{build}',
            'LSMinimumSystemVersion': '11.0', 'GlimUpdatesEnabled': True, 'GlimUpdaterTestMarker': str(marker),
            'SUFeedURL': base + '/appcast.xml', 'SUPublicEDKey': public,
            'SUEnableAutomaticChecks': False, 'SUAutomaticallyUpdate': False,
            'SUEnableSystemProfiling': False, 'SUVerifyUpdateBeforeExtraction': True,
            'SURequireSignedFeed': True, 'SUSignedFeedFailureExpirationInterval': 0,
            'NSAppTransportSecurity': {'NSAllowsArbitraryLoads': True},
        }
        with (bundle / 'Contents/Info.plist').open('wb') as file:
            plistlib.dump(info, file)
        run('python3', root / 'scripts/sign-macos.py', bundle)
        return bundle

    old = bundle_at(stage / 'installed', 100)
    new = bundle_at(stage / 'new', 101)
    original_bundle = stage / 'original/GlimUpdateTest.app'
    run('ditto', old, original_bundle)
    archive = serving / 'GlimUpdateTest.zip'
    run('ditto', '-c', '-k', '--keepParent', new, archive)
    run(sdk / 'bin/generate_appcast', '--ed-key-file', stage / 'private-key', '--download-url-prefix', base + '/', '--maximum-deltas', '0', serving)
    run(sdk / 'bin/sign_update', '--verify', '--ed-key-file', stage / 'private-key', serving / 'appcast.xml')
    # Release generation must retain old compatible items even when only the
    # new archive and the previous signed feed are available to the CI job.
    history = stage / 'history'
    history.mkdir()
    shutil.copy2(serving / 'appcast.xml', history / 'appcast.xml')
    info_path = new / 'Contents/Info.plist'
    with info_path.open('rb') as file:
        info = plistlib.load(file)
    info.update(CFBundleVersion='102', CFBundleShortVersionString='1.0.102')
    with info_path.open('wb') as file:
        plistlib.dump(info, file)
    run('python3', root / 'scripts/sign-macos.py', new)
    run('ditto', '-c', '-k', '--keepParent', new, history / 'GlimUpdateTest-102.zip')
    run(sdk / 'bin/generate_appcast', '--ed-key-file', stage / 'private-key', '--download-url-prefix', base + '/', '--maximum-deltas', '0', '--maximum-versions', '3', history)
    run(sdk / 'bin/sign_update', '--verify', '--ed-key-file', stage / 'private-key', history / 'appcast.xml')
    versions = {item.findtext('{http://www.andymatuschak.org/xml-namespaces/sparkle}version') for item in ET.parse(history / 'appcast.xml').findall('./channel/item')}
    assert versions == {'101', '102'}, versions

    def attempt(expect_success, automatic=False):
        for file in [marker, Path(str(marker) + '.error')]:
            if file.exists():
                file.unlink()
        with (stage / 'host.log').open('wb') as log:
            process = subprocess.Popen([str(old / 'Contents/MacOS/glim')], stdout=log, stderr=log)
            try:
                deadline = time.monotonic() + 100
                while time.monotonic() < deadline and not marker.exists() and not Path(str(marker) + '.error').exists():
                    if automatic and process.poll() is not None:
                        try:
                            with (old / 'Contents/Info.plist').open('rb') as file:
                                installed = plistlib.load(file)['CFBundleVersion'] == '101'
                        except (OSError, plistlib.InvalidFileException):
                            installed = False
                        if installed:
                            # Normal quit should not force a relaunch. Simulate the
                            # user's next launch only after installation completes.
                            run(old / 'Contents/MacOS/glim', timeout=20)
                            break
                    time.sleep(0.2)
                if expect_success:
                    if Path(str(marker) + '.error').exists():
                        raise RuntimeError(Path(str(marker) + '.error').read_text())
                    if not marker.exists():
                        raise RuntimeError('Signed update did not relaunch: ' + (stage / 'host.log').read_text(errors='replace')[-6000:])
                else:
                    assert not marker.exists(), 'Tampered update must not install'
                    error = json.loads(Path(str(marker) + '.error').read_text())
                    assert any(code in [3001, 3002, 4009] for code in error['codes']), error
                with (old / 'Contents/Info.plist').open('rb') as file:
                    assert plistlib.load(file)['CFBundleVersion'] == ('101' if expect_success else '100')
                run('codesign', '--verify', '--deep', '--strict', old)
            finally:
                if process.poll() is None:
                    process.terminate()
                    process.wait(timeout=10)
    try:
        original = archive.read_bytes()
        tampered = bytearray(original)
        tampered[len(tampered) // 2] ^= 1
        archive.write_bytes(tampered)
        attempt(False)
        archive.write_bytes(original)
        os.utime(archive, (time.time() + 2, time.time() + 2))
        attempt(True)
        shutil.rmtree(old)
        run('ditto', original_bundle, old)
        with (old / 'Contents/Info.plist').open('rb') as file:
            info = plistlib.load(file)
        info['GlimUpdaterTestAutomatic'] = True
        with (old / 'Contents/Info.plist').open('wb') as file:
            plistlib.dump(info, file)
        run('python3', root / 'scripts/sign-macos.py', old)
        attempt(True, automatic=True)
        print('Signed updater: history retained, tampered package rejected, manual restart and automatic install-on-quit both passed.')
    finally:
        server.shutdown()
