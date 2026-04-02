import fs from 'fs';
import path from 'path';
import { runMl } from './cli.ts';

// Registry path (relative to repo root) to avoid reliance on __dirname
const REGISTRY_DIR = path.join(process.cwd(), '.pi', 'extensions', 'log_diario');
const REGISTRY_FILE = path.join(REGISTRY_DIR, 'heuristics_registry.json');

function ensureRegistry(){
  try{ fs.mkdirSync(REGISTRY_DIR, { recursive: true }); }catch(e){}
  if(!fs.existsSync(REGISTRY_FILE)){
    try{ fs.writeFileSync(REGISTRY_FILE, JSON.stringify({ heuristics: [] }, null, 2), 'utf8'); }catch(e){}
  }
}

function loadRegistry(): Set<string>{
  try{
    ensureRegistry();
    const raw = fs.readFileSync(REGISTRY_FILE, 'utf8');
    const json = JSON.parse(raw || '{}');
    const arr = Array.isArray(json.heuristics) ? json.heuristics : [];
    return new Set(arr);
  }catch(e){ return new Set(); }
}

function saveRegistry(set: Set<string>){
  try{
    ensureRegistry();
    const arr = Array.from(set.values());
    fs.writeFileSync(REGISTRY_FILE, JSON.stringify({ heuristics: arr }, null, 2), 'utf8');
  }catch(e){ /* ignore */ }
}

// Basic heuristic definitions. Each heuristic has an id and a short description.
const HEURISTICS: Array<{ id: string; name: string; description: string }> = [
  { id: 'colon_para', name: "Para-colon split", description: "Líneas con patrón 'para ...:' que introducen una proposición (ej: 'Para el jueves 26 de marzo de 2026: ...')" },
  { id: 'bullet_list', name: "Bullet list split", description: "Listas con viñetas o guiones que representan ítems separados (ej: '-', '*', '•')." },
  { id: 'numbered_list', name: "Numbered list split", description: "Líneas numeradas que introducen ítems separados (ej: '1. ', '2) ')." },
  { id: 'semicolon_split', name: "Semicolon split", description: "Uso de punto y coma para separar frases o acciones en una misma línea." },
  { id: 'sentence_split', name: "Sentence boundary split", description: "Segmentación por oraciones cuando una línea contiene múltiples oraciones largas (fallback)." }
];

function detectHeuristics(note: string): string[]{
  const used: Set<string> = new Set();
  const lines = String(note || '').split(/\r?\n/).map(l=>l.trim()).filter(Boolean);
  // check bullet list
  const bulletCount = lines.filter(l=>/^[\-\*•·]\s+/.test(l)).length;
  if(bulletCount >= 2) used.add('bullet_list');
  // check numbered list
  const numberedCount = lines.filter(l=>/^\d+[\).\]]\s+/.test(l)).length;
  if(numberedCount >= 2) used.add('numbered_list');
  // colon para
  const colonParaCount = lines.filter(l=>/^(?:para\s+[^:]+:)/i.test(l)).length;
  if(colonParaCount >= 1) used.add('colon_para');
  // semicolon
  if(/;/.test(note)) used.add('semicolon_split');
  // sentences (naive): if a single line contains multiple sentences
  const manySentences = lines.some(l=> (l.match(/[\.!?]\s+/g)||[]).length >= 2 );
  if(manySentences) used.add('sentence_split');
  return Array.from(used.values());
}

function applyHeuristics(note: string): string[]{
  const lines = String(note||'').split(/\r?\n/).map(l=>l.trim()).filter(Boolean);
  // bullet lists
  if(lines.length>1){
    const bulletLines = lines.filter(l=>/^[\-\*•·]\s+/.test(l));
    if(bulletLines.length>=2) return bulletLines.map(l=> l.replace(/^[\-\*•·]\s+/, '').trim());
    const numberedLines = lines.filter(l=>/^\d+[\).\]]\s+/.test(l));
    if(numberedLines.length>=2) return numberedLines.map(l=> l.replace(/^\d+[\).\]]\s+/, '').trim());
  }

  // colon-based splitting (para ...: )
  const colonLines = lines.filter(l=>/^(?:para\s+[^:]+:)/i.test(l));
  if(colonLines.length>=1) return colonLines.map(l=> l.replace(/^(?:para\s+[^:]+:\s*)/i,'').trim());

  // if single long line with semicolons or many sentences, split
  if(lines.length===1){
    const single = lines[0];
    if(single.indexOf(';')>=0){
      const parts = single.split(';').map(p=>p.trim()).filter(Boolean);
      if(parts.length>1) return parts;
    }
    // split by sentences if long
    const sentences = single.split(/[\.\!\?]\s+/).map(s=>s.trim()).filter(Boolean);
    if(sentences.length>1 && sentences.every(s=>s.length>10)) return sentences;
  }

  // fallback: return each non-empty line as proposal, or whole note if a single chunk
  if(lines.length>1) return lines;
  return [ String(note||'').trim() ];
}

/**
 * separateNote: split a free-form note into an array of proposal strings using
 * a set of local heuristics. Optionally, new heuristics discovered can be
 * recorded in Mulch by creating a record under domain "log-diario-heuristics".
 *
 * Behavior is synchronous and conservative: registration is optional and only
 * performed when the environment variable LOG_DIARIO_REGISTER_HEURISTICS=1.
 */
export function separateNote(note: string): { proposals: string[], heuristicsUsed: string[], newlyRegistered: string[] }{
  const heuristicsApplied = detectHeuristics(note);
  const proposals = applyHeuristics(note);

  const registry = loadRegistry();
  const newlyFound = heuristicsApplied.filter(h=> !registry.has(h));
  const newlyRegistered: string[] = [];

  const doRegister = String(process.env.LOG_DIARIO_REGISTER_HEURISTICS || '').trim() === '1';

  if(newlyFound.length>0){
    // update local registry now to avoid race registering multiple times
    for(const h of newlyFound) registry.add(h);
    saveRegistry(registry);

    if(doRegister){
      // For each new heuristic, create a Mulch record to document it
      for(const hid of newlyFound){
        const meta = HEURISTICS.find(x=>x.id===hid);
        const name = meta ? meta.name : `heuristic:${hid}`;
        const desc = meta ? meta.description : `Heuristic detected automatically: ${hid}`;
        try{
          // create a pattern record in Mulch under domain log-diario-heuristics
          const mlArgs = ['record','log-diario-heuristics','--type','pattern','--name', name, '--description', desc, '--files', ''];
          const res = runMl(mlArgs);
          if(res && (res.status === 0 || res.status === null)){ // spawnSync returns status 0 on success; some environments use undefined
            newlyRegistered.push(hid);
          }
        }catch(e){ /* ignore failures */ }
      }
    }
  }

  return { proposals, heuristicsUsed: heuristicsApplied, newlyRegistered };
}
