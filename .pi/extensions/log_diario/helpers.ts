import fs from 'fs';
import os from 'os';
import path from 'path';

export function safeJsonParse(s: string){
  try{ return JSON.parse(s); }catch{ return null; }
}

export function writeTempSession(content: string){
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'logdiario-'));
  const file = path.join(dir, 'session.jsonl');
  fs.writeFileSync(file, content, 'utf8');
  return { dir, file };
}

export function cleanupTemp(dir: string){
  try{ fs.rmSync(dir, { recursive: true, force: true }); }catch(e){}
}

// Resolve thinking value from canonical sources: meta -> ctx -> pi.getState() -> env
export async function resolveThinking(meta: any, ctx: any, pi: any): Promise<{val: string | undefined, src: string}>{
  let val: any = undefined;
  let src = 'none';

  // 1) meta
  try{
    if (meta) {
      if (typeof meta.thinking === 'string' && meta.thinking.trim()) {
        val = meta.thinking;
        src = 'meta.thinking';
        return { val, src };
      }
      if (meta.modelSettings && typeof meta.modelSettings.thinking === 'string' && meta.modelSettings.thinking.trim()) {
        val = meta.modelSettings.thinking;
        src = 'meta.modelSettings.thinking';
        return { val, src };
      }
    }
  }catch(e){ /* ignore */ }

  // 2) ctx.model
  try{
    if (ctx && ctx.model && typeof ctx.model.thinking === 'string' && ctx.model.thinking.trim()){
      val = ctx.model.thinking;
      src = 'ctx.model';
      return { val, src };
    }
  }catch(e){ /* ignore */ }

  // 3) ctx.sessionManager.getState()
  try{
    if (ctx && ctx.sessionManager && typeof (ctx.sessionManager as any).getState === 'function'){
      const state = await (ctx.sessionManager as any).getState();
      if (state){
        if (typeof state.thinking === 'string' && state.thinking.trim()){
          val = state.thinking; src = 'ctx.sessionManager.getState.thinking'; return { val, src };
        }
        if (state.model && typeof state.model.thinking === 'string' && state.model.thinking.trim()){
          val = state.model.thinking; src = 'ctx.sessionManager.getState.model.thinking'; return { val, src };
        }
      }
    }
  }catch(e){ /* ignore */ }

  // 4) pi.getState()
  try{
    if (pi && typeof (pi as any).getState === 'function'){
      const s = await (pi as any).getState();
      if (s){
        if (typeof s.thinking === 'string' && s.thinking.trim()){
          val = s.thinking; src = 'pi.getState.thinking'; return { val, src };
        }
        if (s.model && typeof s.model.thinking === 'string' && s.model.thinking.trim()){
          val = s.model.thinking; src = 'pi.getState.model.thinking'; return { val, src };
        }
      }
    }
  }catch(e){ /* ignore */ }

  // 5) environment
  try{
    const envVal = process.env.PI_THINKING || process.env.ML_THINKING;
    if (typeof envVal === 'string' && envVal.trim()){
      val = envVal; src = 'env'; return { val, src };
    }
  }catch(e){}

  return { val: undefined, src: 'none' };
}

const MONTHS: Record<string,string> = {
  enero:'01', febrero:'02', marzo:'03', abril:'04', mayo:'05', junio:'06',
  julio:'07', agosto:'08', septiembre:'09', octubre:'10', noviembre:'11', diciembre:'12'
};

export function normalizeDateIso(d: string | null | undefined): string {
  if(!d) return '';
  const s = String(d).trim();
  if(/^\d{4}-\d{2}-\d{2}$/.test(s)) return s;
  // dd/mm/yyyy or dd-mm-yyyy
  const dmy = s.match(/^(\d{1,2})[\/\-](\d{1,2})[\/\-](\d{2,4})$/);
  if (dmy) {
    let day = dmy[1].padStart(2,'0');
    let month = dmy[2].padStart(2,'0');
    let year = dmy[3];
    if (year.length === 2) year = '20'+year;
    return `${year}-${month}-${day}`;
  }
  // long form like 26 de marzo de 2026
  const long = s.match(/(\d{1,2})\s+de\s+([a-zñ]+)\s+de\s+(\d{4})/i);
  if(long){ const day=String(long[1]).padStart(2,'0'); const month = MONTHS[(long[2]||'').toLowerCase()]||'01'; const year = long[3]; return `${year}-${month}-${day}`; }
  return s;
}

function formatDateIso(d: Date){ return d.toISOString().slice(0,10); }

export function extractDateFromNote(note: string): string | null {
  if(!note || typeof note !== 'string') return null;
  // ISO
  const iso = note.match(/\b(\d{4}-\d{2}-\d{2})\b/);
  if(iso) return iso[1];
  // dd/mm/yyyy
  const dmy = note.match(/\b(\d{1,2})\s*[\/\-]\s*(\d{1,2})\s*[\/\-]\s*(\d{2,4})\b/);
  if(dmy){ let day=dmy[1].padStart(2,'0'), month=dmy[2].padStart(2,'0'), year=dmy[3]; if(year.length===2) year='20'+year; return `${year}-${month}-${day}`; }
  // long form
  const long = note.match(/\b(\d{1,2})\s+de\s+([a-zñ]+)\s+de\s+(\d{4})\b/i);
  if(long){ const day=String(long[1]).padStart(2,'0'); const month=MONTHS[(long[2]||'').toLowerCase()]||'01'; const year=long[3]; return `${year}-${month}-${day}`; }
  // phrases like 'Para el jueves 26 de marzo de 2026'
  const phrase = note.match(/para\s+(?:el\s+)?[a-z]+\s+(\d{1,2})\s+de\s+([a-zñ]+)\s+de\s+(\d{4})/i);
  if(phrase){ const day=String(phrase[1]).padStart(2,'0'); const month=MONTHS[(phrase[2]||'').toLowerCase()]||'01'; const year=phrase[3]; return `${year}-${month}-${day}`; }
  return null;
}

export function extractDateFromPrime(primeText: string): string | null {
  if(!primeText) return null;
  const iso = primeText.match(/\b(\d{4}-\d{2}-\d{2})\b/);
  if(iso) return iso[1];
  const m = primeText.match(/hoy\s+es\s+(\d{1,2})\s+de\s+([a-zñ]+)\s+de\s+(\d{4})/i);
  if(m){ const day=String(m[1]).padStart(2,'0'); const month=MONTHS[(m[2]||'').toLowerCase()]||'01'; const year=m[3]; return `${year}-${month}-${day}`; }
  // also detect patterns like 'miércoles 25 de marzo 2026' without 'de'
  const mm = primeText.match(/\b(?:lunes|martes|miércoles|miercoles|jueves|viernes|sábado|sabado|domingo)\s+(\d{1,2})\s+de\s+([a-zñ]+)\s+de\s+(\d{4})/i);
  if(mm){ const day=String(mm[1]).padStart(2,'0'); const month=MONTHS[(mm[2]||'').toLowerCase()]||'01'; const year=mm[3]; return `${year}-${month}-${day}`; }
  return null;
}

export function detectRelativeDate(note: string, refIso?: string): { type: 'absolute'|'relative'|'none', iso?: string, text?: string } {
  if(!note) return { type:'none' };
  const ref = refIso ? new Date(refIso+'T00:00:00Z') : new Date();
  const txt = note.toLowerCase();
  if(/\bhoy\b/.test(txt)) return { type:'relative', iso: formatDateIso(ref), text:'hoy' };
  if(/\bayer\b/.test(txt)) { const d=new Date(ref); d.setDate(ref.getDate()-1); return { type:'relative', iso: formatDateIso(d), text:'ayer' }; }
  if(/\b(antes de ayer|anteayer)\b/.test(txt)){ const d=new Date(ref); d.setDate(ref.getDate()-2); return { type:'relative', iso: formatDateIso(d), text:'anteayer' }; }
  const m = txt.match(/hace\s+(\d{1,2})\s+d[ií]as?/);
  if(m){ const n=parseInt(m[1],10); const d=new Date(ref); d.setDate(ref.getDate()-n); return { type:'relative', iso: formatDateIso(d), text:`hace ${n} dias` }; }
  // la semana pasada => 7 dias atrás
  if(/la semana pasada/.test(txt)){ const d=new Date(ref); d.setDate(ref.getDate()-7); return { type:'relative', iso: formatDateIso(d), text:'la semana pasada' }; }
  // try explicit absolute
  const explicit = extractDateFromNote(note);
  if(explicit) return { type:'absolute', iso: explicit, text: explicit };
  return { type:'none' };
}

export function findRelatedEvents(note: string, primeText: string, statusText: string): Array<{ id?: string; title?: string; snippet?: string; reason: string }> {
  const out: Array<{ id?: string; title?: string; snippet?: string; reason: string }> = [];
  if(!note) return out;
  const searchSpace = ((primeText||'') + '\n' + (statusText||'')).toLowerCase();
  // extract candidate tokens: words with capital letters or technical tokens
  const caps = Array.from(new Set((note.match(/\b[A-ZÁÉÍÓÚÑ][a-zA-Z0-9_\.\-]{2,}\b/g)||[]).map(s=>s.toLowerCase())));
  const words = Array.from(new Set((note.match(/\b\w{4,}\b/g)||[]).map(s=>s.toLowerCase()).slice(0,40)));
  const tokens = caps.concat(words).slice(0,40);
  for(const kw of tokens){
    const idx = searchSpace.indexOf(kw);
    if(idx>=0){ const snippet = searchSpace.substr(Math.max(0, idx-60), 160).replace(/\n+/g,' '); out.push({ title: kw, snippet, reason: `match token '${kw}' in prime/status` }); }
  }
  // extract explicit bead ids like mx-abc123
  const ids = Array.from(new Set(((primeText||'').match(/mx-[a-z0-9]+/g)||[])));
  for(const id of ids){
    // try find title nearby
    const idx = (primeText||'').toLowerCase().indexOf(id);
    if(idx>=0){ const snippet = (primeText||'').substr(Math.max(0, idx-80), 160).replace(/\n+/g,' '); out.push({ id, snippet, reason: 'id found in prime' }); }
  }
  // dedupe by title+id
  const uniq = new Map<string, any>();
  for(const r of out) uniq.set((r.id||r.title||'')+ '|' + r.reason, r);
  return Array.from(uniq.values()).slice(0,10);
}
