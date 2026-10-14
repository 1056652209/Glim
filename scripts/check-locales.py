#!/usr/bin/env python3
"""Check shared catalogs, source message IDs and interpolation tokens."""
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
errors = []

def catalog(path):
    def pairs(items):
        result = {}
        for key, value in items:
            if key in result:
                errors.append(f'{path.name}: duplicate key {key!r}')
            result[key] = value
        return result
    return json.loads(path.read_text(), object_pairs_hook=pairs)

catalogs = {p.stem: catalog(p) for p in (ROOT / 'assets/locales').glob('*.json')}
base = catalogs['en']
for code, messages in catalogs.items():
    if not re.fullmatch(r'[A-Za-z]{2,3}(?:-[A-Za-z0-9]{2,8})*', code):
        errors.append(f'{code}: invalid locale code')
    if not isinstance(messages.get('__language_name'), str) or not messages['__language_name'].strip():
        errors.append(f'{code}: missing language name')
for code, messages in catalogs.items():
    for key in base.keys() - messages.keys():
        errors.append(f'{code}: missing {key!r}')
    for key in messages.keys() - base.keys():
        errors.append(f'{code}: unknown {key!r}')
    for key, value in messages.items():
        if not isinstance(value, str) or not value.strip():
            errors.append(f'{code}: empty/invalid {key!r}')
            continue
        if key != '__language_name':
            tokens = lambda text: sorted(re.findall(r'%[A-Za-z][A-Za-z0-9_]*|\{[A-Za-z][A-Za-z0-9_]*\}', text))
            if tokens(base.get(key, key)) != tokens(value):
                errors.append(f'{code}: placeholder mismatch {key!r}')
        if code == 'en' and re.search(r'[\u4e00-\u9fff]', value):
            errors.append(f'en: Chinese UI text {key!r}')
for folder, suffix in [('crates', '*.rs'), ('native', '*.swift')]:
    for path in (ROOT / folder).rglob(suffix):
        if 'tests' in path.parts:
            continue
        source = path.read_text()
        # User content and diagnostic strings are excluded; direct control labels
        # must use the shared lookup (product/format names are invariant).
        if path.suffix == '.rs':
            controls = r'(?:\.(?:label|tooltip|accessibility_label|menu|menu_with_disabled)|PopupMenuItem::new)\(\s*"([^"\n]+)"'
        else:
            controls = r'(?:labelWithString:|wrappingLabelWithString:|checkboxWithTitle:|withTitle:|title:|messageText =|column.title =|window.title =)\s*"([^"\n]+)"'
        for label in re.findall(controls, source):
            if re.search(r'[A-Za-z\u4e00-\u9fff]', label) and label not in {'Glim', 'SQL', 'File', 'Edit', 'A−', 'A+', r'\(Int(size)) pt'}:
                errors.append(f'{path.relative_to(ROOT)}: unlocalized control {label!r}')
        for raw in re.findall(r'\b(?:tr|L)\(\s*"((?:[^"\\]|\\.)*)"', path.read_text()):
            try:
                key = json.loads('"' + raw + '"')
            except ValueError:
                continue
            if key not in base:
                errors.append(f'{path.relative_to(ROOT)}: missing message ID {key!r}')
if errors:
    raise SystemExit('\n'.join(errors))
print(f'Locale catalogs OK: {len(catalogs)} languages, {len(base) - 1} message IDs')
