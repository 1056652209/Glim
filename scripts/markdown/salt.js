// Offline Salt wireframes rendered as SVG. This adapter deliberately parses
// widget data, never HTML, scripts, includes or PlantUML preprocessor commands.
const ns = 'http://www.w3.org/2000/svg';
const gap = 10;
const padding = 12;
const fontSize = 16;

function element(name, attributes = {}, text) {
  const node = document.createElementNS(ns, name);
  for (const [key, value] of Object.entries(attributes)) node.setAttribute(key, String(value));
  if (text !== undefined) node.textContent = text;
  return node;
}

function tokenize(source) {
  const tokens = [];
  let value = '', quoted = false;
  const flush = () => { if (value.trim()) tokens.push(value.trim()); value = ''; };
  for (let i = 0; i < source.length; i++) {
    const ch = source[i];
    if (ch === '"' && source[i - 1] !== '\\') quoted = !quoted;
    if (!quoted && ['{', '}', '|', '\n'].includes(ch)) {
      flush(); tokens.push(ch);
    } else value += ch;
  }
  if (quoted) throw new Error('Salt 输入框的双引号未闭合');
  flush();
  return tokens;
}

function parse(source) {
  const body = source.replace(/\r/g, '').replace(/^\s*@(?:start|end)(?:salt|uml)[^\n]*$/gm, '')
    .replace(/^\s*salt\b\s*/, '').replace(/^\s*'.*$/gm, '').trim();
  if (/^\s*(?:!|skinparam\b|scale\b)/m.test(body)) {
    throw new Error('Salt 线框预览不支持预处理、include、skinparam 或 scale 指令');
  }
  const tokens = tokenize(body);
  let cursor = 0, widgets = 0;
  const skipLines = () => { while (tokens[cursor] === '\n') cursor++; };
  function group(depth) {
    if (depth > 24) throw new Error('Salt 分组嵌套超过 24 层');
    if (tokens[cursor++] !== '{') throw new Error('Salt 窗口必须以 { 开始');
    let mode = '', title = '';
    const first = tokens[cursor];
    if (first && (/^(?:[+#!=/T]|S[hv]?)$/.test(first) || (first.startsWith('^') && !first.endsWith('^')))) {
      const match = first.match(/^([+#!=/T]|S[hv]?|\^)(.*)$/);
      if (match) {
        mode = match[1]; title = match[2].trim(); cursor++;
        if (title && mode !== '^') throw new Error(`Salt 分组标记后应换行：${first}`);
      }
    }
    const rows = []; let row = [], expectingCell = true;
    while (cursor < tokens.length) {
      const token = tokens[cursor];
      if (token === '}') {
        cursor++; if (row.length) rows.push(row);
        if (!rows.length) throw new Error('Salt 分组没有控件');
        return { type: 'group', mode, title, rows };
      }
      if (token === '\n') {
        cursor++; if (row.length) rows.push(row); row = []; expectingCell = true; continue;
      }
      if (token === '|') {
        cursor++; if (expectingCell) row.push({ type: 'widget', text: '' }); expectingCell = true; continue;
      }
      if (!expectingCell) throw new Error('Salt 控件之间需要 | 或换行');
      if (++widgets > 1000) throw new Error('Salt 控件超过 1000 个');
      row.push(token === '{' ? group(depth + 1) : { type: 'widget', text: tokens[cursor++] });
      expectingCell = false;
    }
    throw new Error('Salt 分组缺少结束括号 }');
  }
  skipLines(); const result = group(0); skipLines();
  if (cursor !== tokens.length) throw new Error('Salt 窗口结束后存在无法识别的内容');
  return result;
}

function widget(text, tree) {
  if (/^\.{2,}$|^-{2,}$|^={2,}$|^_{2,}$/.test(text)) return { kind: 'separator', label: '', width: 50, height: 16, text };
  if (text === '.') return { kind: 'blank', label: '', width: 8, height: 30 };
  let kind = 'label', label = text, checked = false, level = 0;
  if (tree) { const match = text.match(/^(\++)\s*(.*)$/); if (match) { level = match[1].length; label = match[2]; } }
  if (/^\[[^\]]*\]$/.test(label)) { kind = 'button'; label = label.slice(1, -1).trim(); }
  else if (/^".*"$/.test(label)) { kind = 'input'; label = label.slice(1, -1).replace(/\\"/g, '"'); }
  else if (/^\^.*\^$/.test(label)) { kind = 'select'; label = label.slice(1, -1); }
  else if (/^\((?:X|x| )?\)/.test(label)) { kind = 'radio'; checked = /^\([Xx]\)/.test(label); label = label.replace(/^\([^)]*\)\s*/, ''); }
  else if (/^\[(?:X|x| )?\]\s+/.test(label)) { kind = 'checkbox'; checked = /^\[[Xx]\]/.test(label); label = label.replace(/^\[[^\]]*\]\s*/, ''); }
  // Never silently drop unsupported rich markup or widget containers.
  if (/[<>]|\{\{|\}\}/.test(label)) throw new Error('Salt 控件暂不支持富文本、图标或内嵌图像');
  const measure = document.createElement('canvas').getContext('2d');
  measure.font = `${fontSize}px Arial, sans-serif`;
  const width = Math.ceil(measure.measureText(label).width) + 24 + level * 18 + (['select', 'checkbox', 'radio'].includes(kind) ? 24 : 0);
  return { kind, label, checked, level, width: Math.max(width, kind === 'input' ? 100 : 24), height: 34 };
}

function layout(node, tree = false) {
  if (node.type === 'widget') { Object.assign(node, widget(node.text, tree)); return; }
  for (const row of node.rows) for (const cell of row) layout(cell, node.mode === 'T');
  node.columns = [];
  for (const row of node.rows) row.forEach((cell, column) => { node.columns[column] = Math.max(node.columns[column] || 0, cell.width); });
  node.heights = node.rows.map(row => Math.max(...row.map(cell => cell.height)));
  node.top = padding + (node.title ? 22 : 0);
  node.width = padding * 2 + node.columns.reduce((a, b) => a + b, 0) + gap * (node.columns.length - 1);
  node.height = node.top + padding + node.heights.reduce((a, b) => a + b, 0) + gap * (node.rows.length - 1);
  if (node.mode.startsWith('S')) { node.width += 14; node.height += 14; }
}

function draw(node, x, y, target, width = node.width) {
  const rect = (rx, ry, w, h, fill = 'white', radius = 0) => target.appendChild(element('rect', { x: rx, y: ry, width: w, height: h, fill, stroke: '#667085', rx: radius }));
  const line = (x1, y1, x2, y2, attrs = {}) => target.appendChild(element('line', { x1, y1, x2, y2, stroke: '#667085', ...attrs }));
  const text = (label, tx, ty) => target.appendChild(element('text', { x: tx, y: ty, fill: '#202936', 'font-size': fontSize, 'font-family': 'Arial, sans-serif' }, label));
  if (node.type === 'widget') {
    const center = y + node.height / 2;
    let tx = x + 8;
    switch (node.kind) {
      case 'blank': return;
      case 'separator':
        line(x, center, x + width, center, node.text.startsWith('.') ? { 'stroke-dasharray': '2 3' } : {});
        if (node.text.startsWith('=')) line(x, center + 3, x + width, center + 3);
        return;
      case 'button': rect(x, y, width, node.height, '#eef1f5', 4); break;
      case 'input': rect(x, y, width, node.height); break;
      case 'select': rect(x, y, width, node.height); target.appendChild(element('path', { d: `M${x + width - 19},${center - 3} l10,0 l-5,6 z`, fill: '#667085' })); break;
      case 'checkbox':
        rect(x + 3, center - 7, 14, 14);
        if (node.checked) target.appendChild(element('path', { d: `M${x + 5},${center} l4,4 l6,-9`, fill: 'none', stroke: '#202936', 'stroke-width': 2 }));
        tx += 24; break;
      case 'radio':
        target.appendChild(element('circle', { cx: x + 10, cy: center, r: 7, fill: 'white', stroke: '#667085' }));
        if (node.checked) target.appendChild(element('circle', { cx: x + 10, cy: center, r: 3, fill: '#202936' }));
        tx += 24; break;
    }
    if (node.level) { tx += node.level * 18; line(tx - 10, center - 10, tx - 10, center); line(tx - 10, center, tx - 2, center); }
    text(node.label, tx, center + 6); return;
  }
  if (['+', '#', '^', '!', '/'].includes(node.mode) || node.mode.startsWith('S')) rect(x, y, width, node.height, '#fafbfc', 3);
  if (node.title) text(node.title, x + padding, y + 21);
  let cy = y + node.top;
  node.rows.forEach((row, i) => {
    let cx = x + padding;
    row.forEach((cell, j) => {
      draw(cell, cx, cy, target, node.columns[j]);
      cx += node.columns[j] + gap;
      if (node.mode === '#' && j < node.columns.length - 1) line(cx - gap / 2, y, cx - gap / 2, y + node.height);
    });
    cy += node.heights[i] + gap;
    if (['#', '='].includes(node.mode) && i < node.rows.length - 1) line(x, cy - gap / 2, x + width, cy - gap / 2);
  });
  if (node.mode.startsWith('S')) {
    rect(x + width - 13, y + 4, 9, node.height - 8, '#eef1f5');
    rect(x + width - 13, y + node.height / 3, 9, node.height / 3, '#aab2bd');
    rect(x + 4, y + node.height - 13, width - 21, 9, '#eef1f5');
  }
}

export function renderSalt(source) {
  const root = parse(source); layout(root);
  if (root.width > 12000 || root.height > 12000) throw new Error('Salt 布局超过尺寸预算');
  const svg = element('svg', { xmlns: ns, width: root.width + 16, height: root.height + 16, viewBox: `0 0 ${root.width + 16} ${root.height + 16}` });
  draw(root, 8, 8, svg);
  return svg.outerHTML;
}
