#!/usr/bin/env python3
"""Generate self-contained README icon walls using the app's licensed file icons."""
from pathlib import Path
import copy
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
NS = 'http://www.w3.org/2000/svg'
ET.register_namespace('', NS)
FORMATS = [
    ('PDF', 'pdf'), ('DOCX', 'word'), ('XLSX', 'sheet'), ('PPTX', 'slides'),
    ('Markdown', 'markdown'), ('HTML', 'html'), ('XMind', 'mindmap'), ('EPUB†', 'book'),
    ('PNG', 'image'), ('JPEG', 'image'), ('GIF', 'image'), ('SVG', 'image'),
    ('TIFF', 'image'), ('WebP', 'image'), ('EXR', 'image'), ('HDR', 'image'),
    ('MP3', 'audio'), ('MP4', 'video'), ('MOV', 'video'), ('FLAC', 'audio'),
    ('TTF', 'font'), ('SQLite', 'database'), ('CSV', 'sheet'), ('JSON', 'json'),
    ('YAML', 'yaml'), ('TOML', 'toml'), ('XML', 'xml'), ('AsciiDoc', 'document'),
    ('Org', 'document'), ('RST', 'document'), ('ZIP†', 'zip'), ('TAR†', 'zip'),
    ('GZ†', 'zip'), ('BZ2†', 'zip'), ('XZ†', 'zip'), ('ODT†', 'document'),
    ('NPY†', 'matrix'), ('OBJ†', 'mesh'), ('STL†', 'mesh'), ('safetensors', 'matrix'),
    ('Rust', 'rust'), ('Python', 'python'), ('JS / TS', 'typescript'), ('Go', 'go'),
    ('Java', 'java'), ('C / C++', 'cpp'), ('CSS', 'css'), ('Vue', 'vue'),
]

CUSTOM = {
    'word': '<path fill="#2b79d0" d="M4 2h11l5 5v15H4z"/><path fill="#fff" d="m6 10 2 8h2l2-5 2 5h2l2-8h-2l-1 5-2-5h-2l-2 5-1-5z"/>',
    'sheet': '<rect x="3" y="2" width="18" height="20" rx="2" fill="#229b69"/><path stroke="#fff" stroke-width="1.5" d="M6 8h12M6 12h12M6 16h12M10 5v14M14 5v14"/>',
    'slides': '<rect x="2" y="4" width="20" height="15" rx="2" fill="#ed7847"/><path stroke="#ed7847" stroke-width="2" d="M12 19v3"/><path fill="#fff" d="M7 8v8h3v-3h2a2.5 2.5 0 0 0 0-5zm3 2h2v1h-2z"/>',
    'mindmap': '<path fill="none" stroke="#27a99b" stroke-width="2" d="M8 12h5V5h4M13 12h4M13 12v7h4"/><rect x="1" y="8" width="8" height="8" rx="2" fill="#27a99b"/><circle cx="19" cy="5" r="3" fill="#65cabb"/><circle cx="19" cy="12" r="3" fill="#65cabb"/><circle cx="19" cy="19" r="3" fill="#65cabb"/>',
    'book': '<path fill="#9c70d4" d="M2 3c4 0 7 1 10 3 3-2 6-3 10-3v17c-4 0-7 1-10 3-3-2-6-3-10-3z"/><path stroke="#fff" stroke-width="1.5" d="M12 6v14M5 8l4 2M5 12l4 2M15 10l4-2M15 14l4-2"/>',
    'font': '<rect x="2" y="2" width="20" height="20" rx="3" fill="#8d78cf"/><text x="12" y="17" text-anchor="middle" fill="#fff" font-family="Arial, sans-serif" font-size="12" font-weight="700">Aa</text>',
    'matrix': '<path fill="none" stroke="#5985ce" stroke-width="2" d="M5 2H2v20h3M19 2h3v20h-3"/><g fill="#5985ce"><rect x="7" y="4" width="4" height="4"/><rect x="13" y="4" width="4" height="4"/><rect x="7" y="10" width="4" height="4"/><rect x="13" y="10" width="4" height="4"/><rect x="7" y="16" width="4" height="4"/><rect x="13" y="16" width="4" height="4"/></g>',
    'mesh': '<path fill="none" stroke="#3d9fc2" stroke-width="1.7" stroke-linejoin="round" d="m12 2 10 5v10l-10 5-10-5V7zM2 7l10 5 10-5M12 12v10M2 17l10-5 10 5"/>',
}

def element(parent, tag, **attributes):
    return ET.SubElement(parent, f'{{{NS}}}{tag}', {key.replace('_', '-'): str(value) for key, value in attributes.items()})

def build(dark=False):
    columns, width, height, gap, margin = 8, 120, 76, 10, 16
    canvas_width = columns * width + (columns - 1) * gap + margin * 2
    rows = (len(FORMATS) + columns - 1) // columns
    canvas_height = rows * height + (rows - 1) * gap + margin * 2
    svg = ET.Element(f'{{{NS}}}svg', {'width': str(canvas_width), 'height': str(canvas_height), 'viewBox': f'0 0 {canvas_width} {canvas_height}', 'role': 'img', 'aria-labelledby': 'title desc'})
    element(svg, 'title', id='title').text = 'Glim file format previews'
    element(svg, 'desc', id='desc').text = 'Selected macOS formats. † means a limited preview. ' + ', '.join(label for label, _ in FORMATS)
    element(svg, 'metadata').text = (ROOT / 'assets/file-icons/LICENSE').read_text()
    defs = element(svg, 'defs')
    for icon in sorted({icon for _, icon in FORMATS}):
        original = ET.fromstring(f'<svg xmlns="{NS}" viewBox="0 0 24 24">{CUSTOM[icon]}</svg>') if icon in CUSTOM else ET.parse(ROOT / f'assets/file-icons/{icon}.svg').getroot()
        symbol = element(defs, 'g', id=f'icon-{icon}')
        for name, value in original.attrib.items():
            if name not in ('width', 'height', 'viewBox', 'id'):
                symbol.set(name, value)
        min_x, min_y, source_width, source_height = map(float, original.attrib.get('viewBox', '0 0 24 24').split())
        scale = min(34 / source_width, 34 / source_height)
        x = (34 - source_width * scale) / 2 - min_x * scale
        y = (34 - source_height * scale) / 2 - min_y * scale
        drawing = element(symbol, 'g', transform=f'matrix({scale} 0 0 {scale} {x} {y})')
        for child in original:
            drawing.append(copy.deepcopy(child))
    background, card, border, foreground = ('#0d1117', '#161b22', '#30363d', '#e6edf3') if dark else ('#f6f8fa', '#ffffff', '#d8dee4', '#24292f')
    element(svg, 'rect', width=canvas_width, height=canvas_height, rx=16, fill=background)
    for index, (label, icon) in enumerate(FORMATS):
        x, y = margin + index % columns * (width + gap), margin + index // columns * (height + gap)
        group = element(svg, 'g', transform=f'translate({x} {y})')
        element(group, 'rect', width=width, height=height, rx=10, fill=card, stroke=border)
        element(group, 'use', href=f'#icon-{icon}', x=43, y=10)
        element(group, 'text', x=width / 2, y=62, text_anchor='middle', font_family='-apple-system, BlinkMacSystemFont, Arial, sans-serif', font_size=12, font_weight=600, fill=foreground).text = label
    return ET.tostring(svg, encoding='unicode') + '\n'

if __name__ == '__main__':
    for dark, name in [(False, 'formats.svg'), (True, 'formats-dark.svg')]:
        (ROOT / 'assets/readme' / name).write_text(build(dark), encoding='utf-8')
