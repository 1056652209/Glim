#!/usr/bin/env python3
"""Verify the Windows release manifest with disposable keys and bounded package fixtures."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import zipfile

repo = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location('feeds', repo / 'scripts/create-update-feeds.py')
feeds = importlib.util.module_from_spec(spec)
spec.loader.exec_module(feeds)
with tempfile.TemporaryDirectory(prefix='glim-windows-feed-test-') as temporary:
    root = Path(temporary)
    key = root / 'private-key'
    public = root / 'public-key'
    source = root / 'keys.swift'
    source.write_text('''import Foundation
import CryptoKit
let key = Curve25519.Signing.PrivateKey()
try key.rawRepresentation.base64EncodedString().write(toFile: CommandLine.arguments[1], atomically: true, encoding: .utf8)
try key.publicKey.rawRepresentation.base64EncodedString().write(toFile: CommandLine.arguments[2], atomically: true, encoding: .utf8)
''')
    subprocess.run(['xcrun', 'swift', str(source), str(key), str(public)], check=True)
    key.chmod(0o600)
    metadata = {'schema': 1, 'application': 'io.github.neopanic.glim', 'platform': 'windows-x64',
                'version': '0.1.14', 'build': 15, 'public_key': public.read_text(), 'updates_enabled': True}
    header = bytearray(80)
    header[:2] = b'MZ'
    header[60:64] = (64).to_bytes(4, 'little')
    header[64:70] = b'PE\0\0\x64\x86'
    archive = root / 'Glim-0.1.14-windows-x64.zip'
    with zipfile.ZipFile(archive, 'w') as package:
        for name, data in [('Glim.exe', header), ('LICENSE.txt', b'license'),
                           ('Read Me.txt', b'instructions'), ('glim-release.json', json.dumps(metadata).encode())]:
            package.writestr('Glim/' + name, data)
    sdk = Path(os.environ.get('GLIM_SPARKLE_DIR', repo / 'target/sparkle/2.10.0'))
    feeds.create_windows_feed(root, '0.1.14', 15, '## English\nTest\n## 中文\n测试\n', sdk / 'bin', key, public.read_text())
    value = json.loads((root / 'windows-update.json').read_text())
    assert value['sha256'] == feeds.checksum(archive)
    verifier = root / 'verify.swift'
    verifier.write_text('''import Foundation
import CryptoKit
let key = try Curve25519.Signing.PublicKey(rawRepresentation: Data(base64Encoded: String(contentsOfFile: CommandLine.arguments[1], encoding: .utf8))!)
let signature = Data(base64Encoded: try String(contentsOfFile: CommandLine.arguments[3], encoding: .utf8).trimmingCharacters(in: .whitespacesAndNewlines))!
guard key.isValidSignature(signature, for: try Data(contentsOf: URL(fileURLWithPath: CommandLine.arguments[2]))) else { exit(1) }
''')
    subprocess.run(['xcrun', 'swift', str(verifier), str(public), str(root / 'windows-update.json'), str(root / 'windows-update.json.sig')], check=True)
    with (root / 'windows-update.json').open('a') as manifest:
        manifest.write('tampered')
    assert subprocess.run(['xcrun', 'swift', str(verifier), str(public), str(root / 'windows-update.json'), str(root / 'windows-update.json.sig')], capture_output=True).returncode != 0
print('Windows feed: identity, metadata, digest, signature interoperability and tamper rejection passed.')
