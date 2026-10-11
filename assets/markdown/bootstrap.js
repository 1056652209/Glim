import { renderToString } from './plantuml.js';
window.PLANTUML_RENDER = renderToString;
await import('./renderer.js');
