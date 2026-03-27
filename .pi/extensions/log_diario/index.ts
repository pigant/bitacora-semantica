import type { ExtensionAPI, ExtensionContext } from "@mariozechner/pi-coding-agent";
import { Type } from "@sinclair/typebox";

const COLLECTOR_PROMPT = `
# NOTA IMPORTANTE PARA EL MODELO: Si tu respuesta requiere invocar herramientas externas (p.ej. `ml learn`, `ml record`, etc.), NO ejecutes ni llames esas herramientas directamente. Primero debes solicitar explícitamente la ejecución de `ml prime` (por ejemplo indicando `tool_call: ml prime`) y esperar a que la extensión ejecute `ml prime` y le pase su contexto. Solo después de eso se podrán proponer comandos para ejecutar. Nunca llames herramientas desde la respuesta del modelo.

Eres un recolector de conocimiento para Mulch. Recibirás una nota diaria y contexto corto extraído de la base Mulch (ml prime / ml status). Para cada hecho relevante en la nota debes generar UN OBJETO JSON. Devuelve SOLO UN ARRAY JSON ([], sin texto adicional ni explicaciones).

Instrucciones:
- Devuelve un array de objetos, uno por cada hecho relevante identificado.
- Campos por objeto:
  - date: fecha del evento en ISO 8601 (YYYY-MM-DD) si puede inferirse; si no, devuelve "".
  - domain: una sola palabra en minúsculas (si no puedes identificar, "general").
  - title: título corto y descriptivo (máx 12 palabras).
  - description: un párrafo conciso con contexto, decisiones y próximos pasos.
  - participants: lista de nombres/roles separados por comas o "desconocido".
  - files: globs separados por comas o "".
  - type: una de [meeting, decision, tradeoff, incident, reference, guide, failure].
  - ml_command: comando ml record listo para ejecutar (con comillas shell escapadas).
  - related: array (posible vacío) de objetos { id?: string, title?: string, snippet?: string, reason?: string } enlazando eventos Mulch relacionados.
  - diagnostics: objeto opcional con información de resolución (ej: date_conflict).

Reglas estrictas:
1) date debe ser ISO YYYY-MM-DD cuando sea posible; si la nota usa términos relativos (ayer, antes de ayer, la semana pasada), resuélvelos usando la fecha de referencia proporcionada más abajo; si no está claro, devuelve "".
2) domain sólo permite [a-z0-9-]; si el contexto proporciona dominios relacionados ({{RELATED_HINTS}}), úsalos preferentemente.
3) Si no hay archivos mencionados, files debe ser "".
4) El ml_command debe tener la forma:
   ml record <domain> --type <type> --name "<title>" --description "<description>" --files "<files>"
5) NO incluyas texto fuera del JSON. Si no hay objetos relevantes, devuelve [].

Contexto adicional (hints de Mulch):
{{RELATED_HINTS}}

Fecha de referencia (si existe): {{NOTE_DATE}}

Nota del usuario:
"""{{NOTE_BODY}}"""
`;

import { safeJsonParse, writeTempSession, cleanupTemp, resolveThinking, extractDateFromNote, extractDateFromPrime, detectRelativeDate, findRelatedEvents, normalizeDateIso } from './helpers.ts';
import { runPiCli, runMl } from './cli.ts';
import { spawn } from 'child_process';

export default function (pi: ExtensionAPI) {
  // In-memory pending proposals per sessionId
  const pending = new Map<string, { parsed: any; ts: number }>();
  const AUTO_VALIDATE_AND_SYNC = true; // run ml validate and ml sync after ml record
  const DEFAULT_EXPIRATION_MS = 30 * 60 * 1000; // 30 minutes
  const DOMAIN_KEYWORDS: Record<string,string[]> = {
    reportes: ['metabase','reporte','reportes','kiosco','report'],
    kiosco: ['kiosco','kiosk','instalacion','instalación','kioskos'],
    operaciones: ['instalación','apoyo','soporte','operaciones','deploy']
  };

  function inferDomain(text: string): string {
    const t = text.toLowerCase();
    for(const d of Object.keys(DOMAIN_KEYWORDS)){
      for(const kw of DOMAIN_KEYWORDS[d]) if(t.includes(kw)) return d;
    }
    return 'general';
  }

  function splitIntoProposals(note: string): string[] {
    // Split by lines that start with common markers (para , - , •) or by sentences if short
    const lines = note.split(/\r?\n/).map(l=>l.trim()).filter(Boolean);
    const proposals: string[] = [];
    for(const l of lines){
      // if line contains multiple clauses separated by ':' treat after ':' as separate
      const m = l.match(/^(?:para\s+[^:]+:)?\s*(.+)$/i);
      if(m) proposals.push(m[1]);
    }
    // if only one proposal but contains multiple clauses separated by ';' or '.' try split
    if(proposals.length===1){
      const parts = proposals[0].split(/[;\.]\s+/).map(p=>p.trim()).filter(Boolean);
      if(parts.length>1) return parts;
    }
    return proposals;
  }

  async function buildMlCommand(parsed: any): Promise<string> {
    // Ensure domain is valid
    const domain = String(parsed.domain || inferDomain(parsed.description || parsed.title || ''))
      .toLowerCase().replace(/[^a-z0-9-]/g,'') || 'general';
    // Map types to allowed mulch types and required flags
    const typeMap: Record<string,string> = {
      meeting: 'meeting', decision: 'decision', tradeoff:'tradeoff', incident:'failure', reference:'reference', guide:'guide', failure:'failure'
    };
    const mlType = typeMap[parsed.type] || 'reference';

    // For decision ensure title and rationale; for failure ensure resolution
    const title = (parsed.title || '').replace(/"/g,'\"') || ('Registro ' + Date.now());
    const description = (parsed.description || '').replace(/"/g,'\"');
    const files = (parsed.files || '').replace(/"/g,'\"');

    let parts = ['ml','record', domain, '--type', mlType];
    if(mlType==='decision'){
      const rationale = (parsed.rationale || parsed.rationale_text || parsed.description.split(/\.|,|;/)[0] || 'Decisión tomada').replace(/"/g,'\"');
      parts.push('--title', title, '--rationale', rationale, '--description', description, '--files', files);
    } else if(mlType==='failure'){
      const resolution = (parsed.resolution || parsed.resolution_text || '').replace(/"/g,'\"') || 'Por resolver';
      parts.push('--name', title, '--description', description, '--resolution', resolution, '--files', files);
    } else {
      parts.push('--name', title, '--description', description, '--files', files);
    }

    // join into a shell-escaped string
    return parts.map(p=>/\s/.test(p)?`"${String(p).replace(/"/g,'\\"') }"`:p).join(' ');
  }

  const canSend = typeof pi.sendUserMessage === "function";

  // Registrar comando /log_diario
  try {
    // registerCommand using examples' handler signature
    pi.registerCommand("log_diario", {
      description: "Crear un registro Mulch a partir de una nota diaria",
      handler: async (args: string, ctx: any) => {
        // args contains text after the slash command; fall back to ctx.input
        let noteBody = (args && String(args).trim()) || (ctx?.input && String(ctx.input).trim()) || "";

        if (!noteBody) {
          await pi.sendUserMessage?.("Por favor pega aquí la nota que quieres convertir en registros Mulch. Luego ejecuta /log_diario de nuevo sin argumentos.");
          return;
        }

        // Split note into atomic proposals
        const proposals = splitIntoProposals(noteBody);
        if(!proposals || proposals.length===0){
          await pi.sendUserMessage?.("No pude extraer propuestas de la nota. Asegúrate de incluir acciones o frases como 'para ...:' en líneas separadas.");
          return;
        }

        const previews: string[] = [];
        const parsedList: any[] = [];
        await pi.sendUserMessage?.(`He detectado ${proposals.length} propuesta(s). Analizando cada una...`);

        for(const p of proposals){
          // resolve dates and related hints for the fragment
          const explicitDate = extractDateFromNote(p);
          const primeDate = null; // prime not available here in the command flow
          const rel = detectRelativeDate(p, primeDate || undefined);
          const noteDateResolved = explicitDate || rel.iso || '';
          const relatedHints = [] as any[]; // no prime in this flow; keep empty

          const prompt = COLLECTOR_PROMPT
            .replace("{{NOTE_BODY}}", p.replace(/"/g, '\\"'))
            .replace("{{NOTE_DATE}}", noteDateResolved || '')
            .replace("{{RELATED_HINTS}}", '');

          if (!canSend) { await pi.sendUserMessage?.("La API de mensajería no está disponible en este entorno."); return; }
          try{
            const response = await pi.sendMessage?.({ role: "user", content: prompt } as any);
            const text = Array.isArray(response?.content)
              ? response.content.map((c: any) => c.text || "").join("\n")
              : response?.content?.text || String(response);

            let parsed: any = null;
            try { parsed = JSON.parse(text); } catch (e) {
              const match = text.match(/\{[\s\S]*\}/);
              if (match) { try { parsed = JSON.parse(match[0]); } catch {} }
            }
            if(!parsed){
              // fallback: minimal parsing
              parsed = { date: noteDateResolved || '', domain: inferDomain(p), title: p.split(/[\.,;\-\(\)]+/)[0].slice(0,80), description: p, participants: 'desconocido', files: '' , type: 'reference' };
            }
            // ensure domain and files and ml_command
            parsed.domain = parsed.domain || inferDomain(p);
            parsed.files = parsed.files || '';
            parsed.date = parsed.date || noteDateResolved || '';
            parsed.related = parsed.related || [];
            parsed.ml_command = await buildMlCommand(parsed);

            const preview = `Dominio: ${parsed.domain}\nTitulo: ${parsed.title}\nFecha: ${parsed.date || ''}\nTipo: ${parsed.type || ''}\nParticipantes: ${parsed.participants || ''}\nArchivos: ${parsed.files || ''}\n\nDescripcion:\n${parsed.description || ''}\n\nComando ml propuesto:\n${parsed.ml_command}`;
            previews.push(preview);
            parsedList.push(parsed);
          } catch(err){
            await pi.sendUserMessage?.('Error al procesar una propuesta: '+String(err));
          }
        }

        // show combined preview and ask for confirmation per-proposal or all
        const combined = previews.map((p,i)=>`--- Propuesta ${i+1} ---\n${p}`).join("\n\n");
        await pi.sendUserMessage?.("Propuestas de registros Mulch:\n" + combined + "\n\nResponde 'sí' para crear todas, 'parcial N,M' para crear índices (ej: 'parcial 1,3'), 'no' para cancelar, o 'preview' para recibir los comandos sin ejecutar.");

        const sessionId = (ctx.session && (ctx.session.id || ctx.session.sessionId)) || String(Date.now());
        pending.set(sessionId, { parsed: parsedList, ts: Date.now() });

        await pi.sendUserMessage?.("He guardado las propuestas. Responde según lo indicado. Esta propuesta expira en 30 minutos.");

      }
    });
  } catch (err) {
    pi.sendUserMessage?.("No se pudo registrar el comando /log_diario: " + String(err));
  }

  // No longer needed: /log_diario_confirm command. Confirmation handled via input listener below.

  // Comando que ejecuta el comando ml record (se asume que ml está en PATH) y usa la skill mulch
  try {
    // Use example-style registration for run command
    pi.registerCommand("log_diario_run", {
      description: "Ejecuta el ml_command dado para crear el registro Mulch",
      handler: async (args: string, ctx: any) => {
        const cmd = (args && String(args).trim()) || (ctx?.input && String(ctx.input).trim()) || "";
        if (!cmd) {
          await pi.sendUserMessage?.("Uso: /log_diario_run <ml_command>");
          return;
        }

        // Ejecutar el comando ml (shell out) — sólo si se permite
        try {
          const parts = String(cmd).split(/\s+/);
          // Básico: usar spawn (importado estáticamente) para ejecutar
          const proc = spawn(parts[0], parts.slice(1), { stdio: 'pipe' });

          let stdout = '';
          let stderr = '';
          proc.stdout.on('data', (d) => { stdout += String(d); });
          proc.stderr.on('data', (d) => { stderr += String(d); });

          const code: number = await new Promise((res) => proc.on('close', res as any));

          if (code === 0) {
            await pi.sendUserMessage?.('ml record ejecutado correctamente. stdout:\n' + stdout);
            // Opcional: validar y sync
            // spawn('ml', ['validate']); spawn('ml', ['sync']);
            return;
          } else {
            await pi.sendUserMessage?.('Error ejecutando ml record. stderr:\n' + stderr);
            return;
          }
        } catch (err) {
          await pi.sendUserMessage?.('Excepción al ejecutar el comando: ' + String(err));
          return;
        }
      }
    });
  } catch (e) {
    pi.sendUserMessage?.("No se pudo registrar /log_diario_run: " + String(e));
  }

  // Register a tool as well for programmatic use
  try {
    pi.registerTool?.({
      name: "log_diario_collect",
      label: "Log Diario: Collect",
      description: "Siempre que entre una nota, analiza una nota y propone un registro Mulch (devuelve JSON)",
      parameters: Type.Object({ note: Type.String() }),
      async execute(id: string, params: { note: string }, signal: AbortSignal | undefined, onUpdate: (update: any) => void, ctx: ExtensionContext, meta: Record<string, any>) : Promise<{ content: Array<{ type: string; text: string }> }> {
        const noteBody = String((params as any).note || "");
        if (!noteBody) return { content: [{ type: 'text', text: 'Proporciona la nota.' }] };
        // Build date/related context using ml prime/status
        const primeRes = runMl(['prime']);
        const primeOut = primeRes.stdout + (primeRes.error? '\n[ml prime error] '+String(primeRes.error): '');
        const primeDate = extractDateFromPrime(primeOut);
        const statusRes = runMl(['status']);
        const statusOut = statusRes.stdout + (statusRes.error? '\n[ml status error] '+String(statusRes.error): '');
        // mark that prime has been executed in this extension (for tool_call gate)
        try{ (pi as any).__log_diario_prime_ran = true; }catch{};
        try{ (globalThis as any).__log_diario_prime_ran = true; }catch{};

        const explicitDate = extractDateFromNote(noteBody);
        const rel = detectRelativeDate(noteBody, primeDate || undefined);
        // priority: explicit > relative resolved with prime > primeDate > ''
        const noteDateResolved = explicitDate || rel.iso || primeDate || '';
        const relatedHintsList = findRelatedEvents(noteBody, primeOut, statusOut);
        const relatedHints = relatedHintsList.map(r=> r.id ? `${r.title || ''}(${r.id})` : `${r.title || ''}`).slice(0,8).join('; ');

        const prompt = COLLECTOR_PROMPT
          .replace('{{NOTE_BODY}}', noteBody.replace(/"/g, '\\"'))
          .replace('{{NOTE_DATE}}', noteDateResolved || '')
          .replace('{{RELATED_HINTS}}', relatedHints);

        // combined prompt for subagent includes prime/status for context
        const combinedPrompt = prompt + '\n\n[ML PRIME OUTPUT]\n' + (primeOut || '') + '\n\n[ML STATUS]\n' + (statusOut || '');

        // Assume pi.sendMessage does not return a response (it injects messages). Use fallback:
        // call `pi` CLI in json mode with the prompt + prime output to get a response

        try{
          console.log('log_diario_collect invoked, note length:', noteBody.length);
        }catch(e){}

        // combinedPrompt already built above; proceed to invoke pi CLI
        try{
          const model = (meta && meta.model) ? `${meta.model.provider}/${meta.model.id}` : undefined;


          const { val: thinkingVal, src: thinkingSource } = await resolveThinking(meta, ctx, pi);
          console.log('thinkingVal resolved from', thinkingSource, ':', String(thinkingVal));

          const argsBase = ['--mode','json','-p'];
          if(typeof thinkingVal === 'string' && thinkingVal.length>0) { argsBase.push('--thinking', String(thinkingVal)); }
          if(model) { argsBase.push('--model', model); }

          let out = '';
          let err = '';
          let assistantOnly = '';
          let parsed = null;
          let parseError: string | null = null;

          // allow one retry: if model requests a tool_call for ml prime, execute ml prime and re-run
          let attempt = 0;
          let usedPrimeAfterToolCall = false;
          while(attempt < 2){
            const args = [...argsBase, combinedPrompt];
            const spawnArgs = [...args, '--no-extensions', '--no-session'];
            console.log('invoking pi CLI with args (attempt', attempt, '):', JSON.stringify(spawnArgs));
            const piRes = runPiCli(spawnArgs, process.env, 120000);
            out = piRes.stdout || '';
            err = piRes.stderr || '';
            console.log('spawn pi exit=', piRes.status, 'stdoutLen=', out.length, 'stderrLen=', err.length);

            assistantOnly = '';
            try{
              const lines = out.split(/\r?\n/).filter(Boolean);
              for(const l of lines){
                try{
                  const ev = JSON.parse(l);
                  // ignore explicit reasoning/thinking events emitted by the runtime
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
            }catch(e){ console.log('error parsing pi output', String(e)); }

            // detect explicit tool_call request for ml prime in assistantOnly
            const wantsPrime = Boolean(assistantOnly && /tool_call\s*[:=]?\s*"?ml\s+prime"?/i.test(assistantOnly)) || Boolean(assistantOnly && /call\s+ml\s+prime/i.test(assistantOnly));

            if(wantsPrime && attempt===0){
              console.log('Assistant requested ml prime (tool_call). Executing ml prime locally and retrying.');
              try{
                const primeRun = runMl(['prime']);
                const primeOut2 = primeRun.stdout + (primeRun.error? '\n[ml prime error] '+String(primeRun.error): '');
                // append prime output to combinedPrompt for next attempt
                combinedPrompt += '\n\n[ML PRIME OUTPUT]\n' + primeOut2;
                usedPrimeAfterToolCall = true;
              }catch(e){ console.log('error executing ml prime on request', String(e)); }
              attempt++;
              continue; // retry
            }

            // try extract first JSON object from assistantOnly
            if(assistantOnly){
              const match = assistantOnly.match(/\{[\s\S]*\}/);
              if(match){
                try{ parsed = JSON.parse(match[0]); }catch(e){ parseError = String(e); }
              }
            }

            break; // exit loop if no tool_call requested or after retry
          }

          // Build payload with only relevant fields
          const payload: any = { tool: 'log_diario_collect', note: noteBody };
          if(parsed) payload.parsed = parsed;
          else payload.assistant = assistantOnly || (out || err || '');
          // include diagnostics truncated to reasonable size (kept internal)
          payload.diagnostics = { stdout_preview: (out||'').slice(0,2000), stderr_preview: (err||'').slice(0,2000), parseError };
          // include ml prime/status full internally
          payload.ml_prime = primeOut;
          payload.ml_status = statusOut;

          // Normalize parsed into an array of proposals
          let parsedArray: any[] = [];
          if(payload.parsed){
            if(Array.isArray(payload.parsed)) parsedArray = payload.parsed;
            else if(typeof payload.parsed === 'object') parsedArray = [payload.parsed];
            else parsedArray = [];
          }

          // If no parsed JSON from assistant, try to build a minimal fallback
          if(parsedArray.length===0 && payload.assistant){
            // try array first
            const arrMatch = String(payload.assistant).match(/\[[\s\S]*\]/);
            if(arrMatch){ try{ const a = JSON.parse(arrMatch[0]); if(Array.isArray(a)) parsedArray = a; }catch(e){} }
            // try object next, but validate it's not a thinking/debug object
            if(parsedArray.length===0){
              const objMatch = String(payload.assistant).match(/\{[\s\S]*\}/);
              if(objMatch){ try{ const o = JSON.parse(objMatch[0]); if(o && typeof o === 'object' && (o.title || o.description || o.domain)) parsedArray = [o]; }catch(e){} }
            }
          }

          // Final fallback: build a single reference record
          if(parsedArray.length===0){
            parsedArray = [{ date: noteDateResolved || '', domain: inferDomain(noteBody), title: (noteBody||'').split(/\n|\.|;|,|:/)[0].slice(0,80), description: noteBody, participants: 'desconocido', files: '', type: 'reference' }];
          }

          // Enrich each parsed item with defaults and relations
          for(const item of parsedArray){
            try{
              item.date = item.date || noteDateResolved || '';
              item.domain = (item.domain || inferDomain(item.description || item.title || noteBody || '')).toLowerCase().replace(/[^a-z0-9-]/g,'') || 'general';
              item.files = item.files || '';
              item.related = item.related || relatedHintsList || [];
              item.ml_command = item.ml_command || await buildMlCommand(item);
            }catch(e){ /* ignore per-item */ }
          }

          // Return only the parsed JSON array
          return { content: [{ type: 'text', text: JSON.stringify(parsedArray) }] };
        }catch(e){
          console.log('error running pi fallback', String(e));
          const payload = { tool: 'log_diario_collect', note: noteBody, error: String(e) };
          return { content: [{ type: 'text', text: JSON.stringify(payload) }] };
        }
      }
    });
  } catch (e) {
    // ignore
  }

  // Tool call gate: require ml prime to be executed before arbitrary tool calls (pattern extracted from reference)
  try {
    let primeRan = false; // flips to true when we execute runMl(['prime']) in this extension

    // Mark primeRan when we call ml prime inside the tool (we already call it there)
    // We'll set it in the execute function when prime is run.

    pi.on?.('tool_call', async (event: any) => {
      // allow internal log_diario tool calls
      if (!event || !event.toolName) return { block: false };
      if (event.toolName === 'log_diario_collect') return { block: false };

      // consider global flags (in case prime was run elsewhere)
      const primeFlag = ((pi as any).__log_diario_prime_ran) || ((globalThis as any).__log_diario_prime_ran) || primeRan;
      if (!primeFlag) {
        return {
          block: true,
          reason: "🚨 Antes de usar herramientas externas debes ejecutar 'ml prime' para cargar el contexto Mulch. Indica 'tool_call: ml prime' en tu respuesta o ejecuta `ml prime` y reintenta. No invoques herramientas directamente desde la respuesta del modelo.",
        };
      }

      return { block: false };
    });
  } catch (e) {
    // ignore
  }

  // Listen for user input to confirm/cancel pending proposals
  try {
    // Listen to standard 'input' events (safer). Do not return objects—just perform side-effects.
    pi.on?.('input', async (event: any) => {
      try {
        console.log('input entrante', JSON.stringify(event));
        // Only handle interactive user messages
        if (event?.source !== 'interactive') {
          return; // only handle interactive user inputs
        }
        const raw = event?.text ?? event?.message ?? '';
        const text = typeof raw === 'string' ? raw.trim().toLowerCase() : '';
        const sessionId = (event?.session && (event.session.id || event.session.sessionId)) || String(Date.now());
        if (!pending.has(sessionId)) return;
        const entry = pending.get(sessionId)!;
        // expire after configured expiration
        if (Date.now() - entry.ts > DEFAULT_EXPIRATION_MS) { pending.delete(sessionId); await pi.sendUserMessage?.('La propuesta ha expirado.'); return; }

        // Normalize commands: confirm all, confirm 1,2, preview, edit N, cancel
        if (text === 'confirm all') {
          const list = Array.isArray(entry.parsed) ? entry.parsed : [entry.parsed];
          await pi.sendUserMessage?.('Ejecutando todos los comandos ml (uno por propuesta)...');
          for(const item of list){
            const cmdObj = item._cmdObj || { cmd: item.ml_command };
            if(!cmdObj || !cmdObj.cmd){ await pi.sendUserMessage?.('Falta ml_command en una propuesta, omitiendo.'); continue; }
            await pi.sendUserMessage?.('Ejecutando: '+cmdObj.cmd);
            try{
              const proc = Array.isArray(cmdObj.argv) && cmdObj.argv.length>0 ? spawn(cmdObj.argv[0], cmdObj.argv.slice(1), { stdio: 'pipe' }) : spawn(String(cmdObj.cmd).split(/\s+/)[0], String(cmdObj.cmd).split(/\s+/).slice(1), { stdio: 'pipe' });
              let stdout=''; let stderr='';
              proc.stdout.on('data', d=>{ stdout+=String(d); }); proc.stderr.on('data', d=>{ stderr+=String(d); });
              const code: number = await new Promise((res)=> proc.on('close', res as any));
              if(code===0){ await pi.sendUserMessage?.('Registro creado correctamente. stdout:\n'+stdout); }
              else { await pi.sendUserMessage?.('Error creando registro. stderr:\n'+stderr); }
            } catch(err){ await pi.sendUserMessage?.('Excepción al ejecutar ml: '+String(err)); }
          }
          if(AUTO_VALIDATE_AND_SYNC){ try{ await pi.sendUserMessage?.('Ejecutando ml validate y ml sync...'); spawn('ml',['validate']); spawn('ml',['sync']); }catch{} }
          pending.delete(sessionId); return;
        } else if (text.startsWith('confirm ')) {
          // confirm specific indices: e.g. 'confirm 1,3'
          const nums = text.replace('confirm','').trim().split(/[ ,]+/).map(s=>parseInt(s,10)).filter(n=>!Number.isNaN(n));
          if(nums.length===0){ await pi.sendUserMessage?.('No entendí los índices. Usa "confirm 1,3"'); return; }
          const list = Array.isArray(entry.parsed) ? entry.parsed : [entry.parsed];
          for(const idx of nums){ const item = list[idx-1]; if(!item){ await pi.sendUserMessage?.('Índice '+idx+' inválido.'); continue; }
            const cmdObj = item._cmdObj || { cmd: item.ml_command };
            await pi.sendUserMessage?.('Ejecutando: '+cmdObj.cmd);
            try{ const proc = Array.isArray(cmdObj.argv) && cmdObj.argv.length>0 ? spawn(cmdObj.argv[0], cmdObj.argv.slice(1), { stdio: 'pipe' }) : spawn(String(cmdObj.cmd).split(/\s+/)[0], String(cmdObj.cmd).split(/\s+/).slice(1), { stdio: 'pipe' }); let stdout=''; let stderr=''; proc.stdout.on('data', d=>{ stdout+=String(d); }); proc.stderr.on('data', d=>{ stderr+=String(d); }); const code: number = await new Promise((res)=> proc.on('close', res as any)); if(code===0){ await pi.sendUserMessage?.('Registro creado correctamente. stdout:\n'+stdout); } else { await pi.sendUserMessage?.('Error creando registro. stderr:\n'+stderr); } } catch(err){ await pi.sendUserMessage?.('Excepción al ejecutar ml: '+String(err)); }
          }
          if(AUTO_VALIDATE_AND_SYNC){ try{ await pi.sendUserMessage?.('Ejecutando ml validate y ml sync...'); spawn('ml',['validate']); spawn('ml',['sync']); }catch{} }
          pending.delete(sessionId); return;
        } else if (text === 'preview') {
          const list = Array.isArray(entry.parsed) ? entry.parsed : [entry.parsed];
          const cmds = list.map((i:any,idx:number)=>`--- ${idx+1} ---\n${i.ml_command}`).join('\n\n');
          await pi.sendUserMessage?.('Comandos ml preparados:\n'+cmds);
          return;
        } else if (text.startsWith('edit ')) {
          const n = parseInt(text.replace('edit','').trim(),10);
          if(Number.isNaN(n) || n<1){ await pi.sendUserMessage?.('Usa "edit N" donde N es el índice de la propuesta.'); return; }
          const list = Array.isArray(entry.parsed) ? entry.parsed : [entry.parsed];
          const item = list[n-1]; if(!item){ await pi.sendUserMessage?.('Índice inválido.'); return; }
          await pi.sendUserMessage?.(`Por favor envía un JSON con los campos a sobrescribir para la propuesta ${n}. Campos posibles: title,description,participants,files,type,domain,rationale,resolution`);
          // wait for next input containing JSON
          const next = await new Promise<string>((res)=>{
            const handler = (ev:any)=>{
              try{ if(ev?.session && ev.session.sessionId===event.session.sessionId){ pi.off?.('input', handler as any); res(ev.text || ev.message || ''); } }catch(e){}
            };
            pi.on?.('input', handler as any);
            setTimeout(()=>{ pi.off?.('input', handler as any); res(''); }, 1000*60*5);
          });
          if(!next){ await pi.sendUserMessage?.('Edición cancelada o tiempo expirado.'); return; }
          let patch = null;
          try{ patch = JSON.parse(next); }catch(e){ await pi.sendUserMessage?.('JSON inválido. Cancelando.'); return; }
          Object.assign(item, patch);
          // rebuild ml_command
          item.ml_command = await buildMlCommand(item);
          item._cmdObj = { cmd: item.ml_command, argv: Array.isArray(item.ml_command)? item.ml_command : undefined };
          await pi.sendUserMessage?.('Propuesta actualizada. Nuevo comando:\n'+item.ml_command);
          return;
        } else if (text === 'cancel' || text === 'no') {
          pending.delete(sessionId);
          await pi.sendUserMessage?.('Propuesta cancelada.');
          return;
        } else {
          await pi.sendUserMessage?.('Comando no reconocido. Usa: "confirm all", "confirm 1,2", "preview", "edit N" o "cancel".');
        }
      } catch (e) {
        // ignore
      }
    });
  } catch (e) {
    // ignore
  }

  // Informational message: defer until session_start to avoid runtime-not-initialized errors
  try {
    pi.on?.('session_start', async () => {
      await pi.sendUserMessage?.('Extensión log_diario cargada. Usa /log_diario para generar registros Mulch desde notas en español.');
    });
  } catch (e) {
    // ignore
  }
}

