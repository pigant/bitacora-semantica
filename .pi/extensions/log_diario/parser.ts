import { safeJsonParse } from './helpers.ts';

export function extractAssistantTextFromPiOutput(out: string){
  if(!out) return '';
  let assistantOnly = '';
  const lines = out.split(/\r?\n/).filter(Boolean);
  for(const l of lines){
    try{
      const ev = JSON.parse(l);
      if(ev?.type === 'thinking' || ev?.assistantMessageEvent?.type === 'thinking') continue;
      if(ev?.assistantMessageEvent && ev.assistantMessageEvent?.type === 'final' && typeof ev.assistantMessageEvent?.text === 'string'){
        assistantOnly += ev.assistantMessageEvent.text + '\n';
      } else if(ev?.assistantMessageEvent && ev.assistantMessageEvent?.type === 'text_delta'){
        assistantOnly += ev.assistantMessageEvent.delta || '';
      } else if(ev?.message && ev.message.role === 'assistant' && ev.message.content){
        const content = ev.message.content;
        if(Array.isArray(content)) assistantOnly += content.map((c:any)=>c.text||c.value||'').join('\n');
        else if(typeof content === 'string') assistantOnly += content;
        else if(content.text) assistantOnly += content.text;
      }
    }catch(e){ /* ignore non-json lines */ }
  }
  return assistantOnly;
}

export function parseAssistantTextToArray(assistantText: string){
  if(!assistantText) return { parsedArray: [], parseError: 'empty' };
  // try full JSON
  try{ const j = JSON.parse(assistantText); if(Array.isArray(j)) return { parsedArray: j, parseError: null }; if(j && typeof j === 'object') return { parsedArray: [j], parseError: null }; }catch(e){}
  // try extract array/object
  const arrMatch = String(assistantText).match(/\[[\s\S]*\]/);
  if(arrMatch){ try{ const a = JSON.parse(arrMatch[0]); if(Array.isArray(a)) return { parsedArray: a, parseError: null }; }catch(e){} }
  const objMatch = String(assistantText).match(/\{[\s\S]*\}/);
  if(objMatch){ try{ const o = JSON.parse(objMatch[0]); if(o && typeof o === 'object' && (o.title || o.description || o.domain)) return { parsedArray: [o], parseError: null }; }catch(e){} }
  return { parsedArray: [], parseError: 'no-json' };
}

export function normalizeParsedArray(parsedArray: any[], noteBody: string, noteDateResolved: string, relatedHintsList: any[], buildMlCommand: (p:any)=>Promise<string>, inferDomain: (t:string)=>string){
  return Promise.all((parsedArray||[]).map(async (item:any)=>{
    try{
      item.date = item.date || noteDateResolved || '';
      item.domain = (item.domain || inferDomain(item.description || item.title || noteBody || '')).toLowerCase().replace(/[^a-z0-9-]/g,'') || 'general';
      item.files = item.files || '';
      item.related = item.related || relatedHintsList || [];
      item.ml_command = item.ml_command || await buildMlCommand(item);
      return item;
    }catch(e){ return null; }
  })).then(arr=> (arr||[]).filter(Boolean).filter((it:any)=>{ if(!it || typeof it !== 'object') return false; if(it.type === 'thinking') return false; if(!it.title && !it.description && !it.domain) return false; return true; }).then? (arr as any) : arr);
}
