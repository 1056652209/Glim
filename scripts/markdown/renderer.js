import mermaid from 'mermaid';
import zenuml from '@mermaid-js/mermaid-zenuml';
import { renderSalt } from './salt.js';
import { chenToDot } from './chen.js';
import { instance } from '@viz-js/viz';
import flowchart from 'flowchart.js';
import WaveDrom from 'wavedrom';
import JSON5 from 'json5';
import { mathjax } from 'mathjax-full/js/mathjax.js';
import { TeX } from 'mathjax-full/js/input/tex.js';
import { SVG } from 'mathjax-full/js/output/svg.js';
import { liteAdaptor } from 'mathjax-full/js/adaptors/liteAdaptor.js';
import { RegisterHTMLHandler } from 'mathjax-full/js/handlers/html.js';
import { AllPackages } from 'mathjax-full/js/input/tex/AllPackages.js';

// Diagram source is data, never executable JavaScript. All engine resources
// are served by the private, in-memory protocol; CSP blocks remote resources.
mermaid.initialize({
  startOnLoad: false,
  securityLevel: 'strict',
  theme: 'default',
  fontFamily: 'Arial, sans-serif',
  flowchart: { htmlLabels: false },
  journey: { textPlacement: 'svg' },
  htmlLabels: false,
  maxTextSize: 262144,
  maxEdges: 1000,
});
const externalDiagramsReady = mermaid.registerExternalDiagrams([zenuml]);
window.Viz = { instance };
window.WaveSkin = WaveDrom.waveSkin;
window.PLANTUML_STDLIB_BASE = './';

const adaptor = liteAdaptor();
RegisterHTMLHandler(adaptor);
const mathDocument = mathjax.document('', {
  InputJax: new TeX({
    // noundefined paints unknown commands red instead of reporting a parse
    // error, which would make malformed or disabled commands look successful.
    packages: AllPackages.filter(name => !['require', 'autoload', 'html', 'noundefined'].includes(name)),
    maxBuffer: 262144,
    maxMacros: 1000,
  }),
  OutputJax: new SVG({ fontCache: 'none' }),
});
let viz;

function normalize(svg) {
  const root = new DOMParser().parseFromString(svg, 'image/svg+xml').documentElement;
  if (root.localName !== 'svg' || root.querySelector('parsererror')) {
    throw new Error('图表引擎没有返回有效 SVG');
  }
  root.setAttribute('xmlns', 'http://www.w3.org/2000/svg');
  if ((!root.hasAttribute('width') || root.getAttribute('width').includes('%')) && root.viewBox.baseVal.width > 0) {
    root.setAttribute('width', String(root.viewBox.baseVal.width));
    root.setAttribute('height', String(root.viewBox.baseVal.height));
  }
  // Resolve CSS and currentColor while the browser has the engine styles.
  // Explicit pixel sizes also make MathJax's ex/em SVG units portable to resvg.
  const host = document.getElementById('output');
  host.replaceChildren(document.importNode(root, true));
  const mounted = host.firstElementChild;
  const box = mounted.getBoundingClientRect();
  const alignment = parseFloat(getComputedStyle(mounted).verticalAlign) || 0;
  if (box.width > 0 && box.height > 0) {
    mounted.setAttribute('width', String(box.width));
    mounted.setAttribute('height', String(box.height));
  }
  for (const element of mounted.querySelectorAll('*')) {
    if (element.localName === 'script' || element.localName === 'foreignObject') {
      throw new Error('此图表使用了当前原生预览不支持的 HTML 标签');
    }
    for (const attr of [...element.attributes]) {
      if (attr.name.startsWith('on')) element.removeAttribute(attr.name);
      if (['href', 'xlink:href'].includes(attr.name) && !attr.value.startsWith('#')) {
        element.removeAttribute(attr.name);
      }
    }
    const style = getComputedStyle(element);
    for (const name of ['fill', 'stroke', 'font-family', 'font-size', 'font-weight', 'text-anchor']) {
      let value = style.getPropertyValue(name);
      // Browser CSSOM expands fragment paint URLs to absolute protocol URLs.
      value = value.replace(/url\(["']?[^#)]*#([^)'" ]+)["']?\)/g, 'url(#$1)');
      if (value) element.setAttribute(name, value);
    }
  }
  // Give dark native themes the same legible engine canvas as light themes.
  mounted.style.background = 'white';
  const background = document.createElementNS('http://www.w3.org/2000/svg', 'rect');
  const viewBox = mounted.viewBox.baseVal;
  background.setAttribute('x', String(viewBox.x));
  background.setAttribute('y', String(viewBox.y));
  background.setAttribute('width', String(viewBox.width || box.width));
  background.setAttribute('height', String(viewBox.height || box.height));
  background.setAttribute('fill', 'white');
  mounted.insertBefore(background, mounted.firstChild);
  return {
    svg: new XMLSerializer().serializeToString(mounted),
    baseline: Math.max(0, Math.min(box.height, box.height + alignment)),
  };
}

async function render(request, index) {
  const host = document.getElementById('output');
  host.replaceChildren();
  const { kind, source } = request;
  switch (kind) {
    case 'zenuml':
    case 'mermaid': {
      await externalDiagramsReady;
      const input = kind === 'zenuml' && !/^\s*zenuml\b/.test(source) ? `zenuml\n${source}` : source;
      const result = await mermaid.render(`glim-diagram-${index}`, input);
      return result.svg;
    }
    case 'dot':
      viz ??= await instance();
      return viz.renderString(source, { format: 'svg' });
    case 'plantuml': {
      if (/^\s*@startchen\b/m.test(source)) {
        viz ??= await instance();
        return viz.renderString(chenToDot(source), { format: 'svg' });
      }
      if (/^\s*@startsalt\b/m.test(source) || /^\s*(?:@startuml\s*\n)?salt\b/m.test(source)) {
        return renderSalt(source);
      }
      if (/^\s*@startdot\b/m.test(source)) {
        viz ??= await instance();
        return viz.renderString(source.replace(/^\s*@startdot[^\n]*\n/m, '').replace(/^\s*@enddot\s*$/m, ''), { format: 'svg' });
      }
      if (/^\s*!include(?:url|_once|_many)?\s+(?!<)/mi.test(source)) {
        throw new Error('离线预览只支持随应用提供的 PlantUML 标准库 include');
      }
      const wrapped = /^\s*@start\w+/m.test(source) ? source : `@startuml\n${source}\n@enduml`;
      const svg = await new Promise((resolve, reject) => window.PLANTUML_RENDER(wrapped.split(/\r?\n/), resolve, reject));
      const document = new DOMParser().parseFromString(svg, 'image/svg+xml');
      const message = document.documentElement.textContent || '';
      if (/Diagram not supported by this release of PlantUML|Syntax Error\?|Syntax Error|Cannot include|Cannot load theme/i.test(message)) {
        throw new Error(message.trim().slice(0, 1200));
      }
      return svg;
    }
    case 'flowchart': {
      host.id = `flowchart-${index}`;
      try {
        flowchart.parse(source).drawSVG(host.id);
        const svg = host.querySelector('svg');
        if (!svg) throw new Error('流程图没有可显示的节点');
        return svg.outerHTML;
      } finally { host.id = 'output'; }
    }
    case 'wavedrom': {
      const container = document.createElement('div');
      container.id = `wave-${index}`;
      host.appendChild(container);
      WaveDrom.renderWaveForm(index, JSON5.parse(source), 'wave-');
      const svg = container.querySelector('svg');
      if (!svg) throw new Error('WaveDrom 没有可显示的波形');
      return svg.outerHTML;
    }
    case 'math':
    case 'inline-math': {
      const node = mathDocument.convert(source, {
        display: kind !== 'inline-math', em: 16, ex: 8, containerWidth: 1000,
      });
      const markup = adaptor.outerHTML(node);
      host.innerHTML = markup;
      const error = host.querySelector('[data-mjx-error]');
      if (error) throw new Error(`数学公式语法无法识别：${error.getAttribute('data-mjx-error') || '解析错误'}`);
      const svg = host.querySelector('svg');
      if (!svg) throw new Error('数学公式没有返回 SVG');
      return svg.outerHTML;
    }
    default: throw new Error('未知图表类型');
  }
}

async function main() {
  const requests = window.GLIM_REQUESTS;
  for (let index = 0; index < requests.length; index++) {
    try {
      const result = normalize(await Promise.race([
        render(requests[index], index),
        new Promise((_, reject) => setTimeout(() => reject(new Error('此图表渲染超过 8 秒')), 8000)),
      ]));
      if (new TextEncoder().encode(result.svg).byteLength > 8 * 1024 * 1024) throw new Error('图表 SVG 超过 8 MiB');
      window.ipc.postMessage(JSON.stringify({ index, ...result }));
    } catch (error) {
      window.ipc.postMessage(JSON.stringify({ index, error: String(error).slice(0, 2000) }));
    }
  }
  window.ipc.postMessage(JSON.stringify({ done: true }));
}
main().catch(error => window.ipc.postMessage(JSON.stringify({ fatal: String(error).slice(0, 2000) })));
