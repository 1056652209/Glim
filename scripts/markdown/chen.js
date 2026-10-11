// Chen/EER's common entity, attribute and cardinality syntax, mapped to DOT.
// Source is parsed as data; unsupported directives fail rather than disappearing.
const id = '[\\p{L}_][\\p{L}\\p{N}_]*';
const named = `(?:"([^"\\n]+)"\\s+as\\s+(${id})|(${id}))`;
const quote = value => JSON.stringify(String(value));
const html = value => String(value).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');

function statements(source) {
  const result = []; let text = '', quoted = false;
  for (let i = 0; i < source.length; i++) {
    const ch = source[i];
    if (ch === '"' && source[i - 1] !== '\\') quoted = !quoted;
    if (!quoted && ['{', '}', '\n'].includes(ch)) {
      if (text.trim()) result.push(text.trim()); text = '';
      if (ch !== '\n') result.push(ch);
    } else text += ch;
  }
  if (quoted) throw new Error('Chen 标签引号未闭合');
  if (text.trim()) result.push(text.trim());
  return result;
}

export function chenToDot(source) {
  const lines = statements(source.replace(/\r/g, '').replace(/^\s*@(?:start|end)chen[^\n]*$/gm, '').replace(/^\s*'.*$/gm, ''));
  const nodes = [], edges = [], known = new Set();
  let cursor = 0, count = 0, direction = 'TB', title = '';
  const emit = value => { if (++count > 1000) throw new Error('Chen 图超过 1000 个节点'); nodes.push(value); };
  const declaration = new RegExp(`^(entity|relationship)\\s+${named}\\s*((?:<<[^>]+>>\\s*)*)(#[a-zA-Z0-9]+)?$`, 'u');
  const attribute = new RegExp(`^${named}(?:\\s*:\\s*([^<]+?))?\\s*((?:<<[^>]+>>\\s*)*)$`, 'u');
  const connection = new RegExp(`^(${id})\\s*([-=])([^-=]+)\\2\\s*(${id})$`, 'u');
  function flags(value, allowed) {
    const names = [...value.matchAll(/<<([^>]+)>>/g)].flatMap(m => m[1].split(/[,\s]+/)).filter(Boolean);
    if (names.some(name => !allowed.includes(name.toLowerCase()))) throw new Error(`Chen 不支持该标记：${value}`);
    return names.map(name => name.toLowerCase());
  }
  function attributes(parent, depth) {
    if (depth > 24) throw new Error('Chen 复合属性嵌套超过 24 层');
    while (cursor < lines.length && lines[cursor] !== '}') {
      const line = lines[cursor++]; const match = line.match(attribute);
      if (!match) throw new Error(`Chen 属性语法无法识别：${line}`);
      const label = match[1] || match[3], type = match[4]?.trim();
      const options = flags(match[5], ['key', 'derived', 'multi']);
      const node = `attribute-${count}`;
      const display = label + (type ? ` : ${type}` : '');
      const attrs = ['shape=ellipse', options.includes('key') ? `label=<<U>${html(display)}</U>>` : `label=${quote(display)}`];
      if (options.includes('derived')) attrs.push('style=dashed');
      if (options.includes('multi')) attrs.push('peripheries=2');
      emit(`${quote(node)} [${attrs.join(',')}];`);
      edges.push(`${quote(parent)} -- ${quote(node)};`);
      if (lines[cursor] === '{') { cursor++; attributes(node, depth + 1); }
    }
    if (lines[cursor++] !== '}') throw new Error('Chen 属性块缺少结束括号 }');
  }
  while (cursor < lines.length) {
    const line = lines[cursor++];
    if (line === 'left to right direction') { direction = 'LR'; continue; }
    if (line === 'top to bottom direction') { direction = 'TB'; continue; }
    if (line.startsWith('title ')) { title = line.slice(6); continue; }
    const node = line.match(declaration);
    if (node) {
      const kind = node[1], label = node[2] || node[4], name = node[3] || node[4];
      if (known.has(name)) throw new Error(`Chen 名称重复：${name}`);
      known.add(name);
      const options = flags(node[5], ['weak', 'identifying']);
      const attrs = [`shape=${kind === 'entity' ? 'box' : 'diamond'}`, `label=${quote(label)}`];
      if (options.length) attrs.push('peripheries=2');
      if (node[6]) attrs.push('style=filled', `fillcolor=${quote(node[6])}`);
      emit(`${quote(name)} [${attrs.join(',')}];`);
      if (lines[cursor] === '{') { cursor++; attributes(name, 0); }
      continue;
    }
    const edge = line.match(connection);
    if (edge) { edges.push({ from: edge[1], to: edge[4], label: edge[3].trim(), total: edge[2] === '=' }); continue; }
    throw new Error(`Chen 语法暂不支持：${line}`);
  }
  if (!known.size) throw new Error('Chen 图没有实体或关系');
  const dotEdges = edges.flatMap(edge => {
    if (typeof edge === 'string') return [edge];
    if (!known.has(edge.from) || !known.has(edge.to)) throw new Error(`Chen 连接引用了未定义节点：${edge.from} / ${edge.to}`);
    const result = `${quote(edge.from)} -- ${quote(edge.to)} [label=${quote(edge.label)}];`;
    return edge.total ? [result, `${quote(edge.from)} -- ${quote(edge.to)};`] : [result];
  });
  return `graph Chen { graph [rankdir=${direction}, bgcolor="white", label=${quote(title)}, labelloc=t]; node [fontname="Arial", fontsize=16]; edge [fontname="Arial", fontsize=14]; ${nodes.join('\n')} ${dotEdges.join('\n')} }`;
}
