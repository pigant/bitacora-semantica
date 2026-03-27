export function buildCollectorPrompt(template: string, noteBody: string, noteDate: string, relatedHints: string, primeOut?: string, statusOut?: string){
  const base = template
    .replace('{{NOTE_BODY}}', (noteBody||'').replace(/"/g,'\\"'))
    .replace('{{NOTE_DATE}}', noteDate || '')
    .replace('{{RELATED_HINTS}}', relatedHints || '');

  let combined = base;
  if(primeOut) combined += '\n\n[ML PRIME OUTPUT]\n' + primeOut;
  if(statusOut) combined += '\n\n[ML STATUS]\n' + statusOut;
  return combined;
}
