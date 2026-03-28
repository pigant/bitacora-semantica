import type { ExtensionAPI, ExtensionContext } from "@mariozechner/pi-coding-agent";
import { Type } from "@sinclair/typebox";
import { Container, Text, Spacer, SelectList, Input, Editor, BorderedLoader } from "@mariozechner/pi-tui";
import { truncateToWidth } from "@mariozechner/pi-tui";
import { buildMlCommand } from '../log_diario/helpers.ts';

// Minimal TUI wizard component skeleton. Uses ctx.ui.custom((tui,theme,kb,done)=>...) pattern.

export default function registerWizard(pi: ExtensionAPI) {
  // register tool
  try{
    pi.registerTool?.({
      name: "log_diario_wizard",
      label: "Log Diario: Wizard",
      description: "Asistente paso a paso para crear registros Mulch (TUI)",
      parameters: Type.Object({ note: Type.Optional(Type.String()), enableLLM: Type.Optional(Type.Boolean()) }),
      async execute(id: string, params: any, signal: AbortSignal | undefined, onUpdate: (upd:any)=>void, ctx: ExtensionContext, meta: Record<string, any>) {
        console.log('[log_diario_wizard] execute called, id=', id, 'params=', params);
        const note = params?.note || '';
        const enableLLM = params?.enableLLM !== undefined ? params.enableLLM : true;

        // simple form state
        const form: any = {
          title: '', domain: '', type: 'reference', date: '', participants: '', files: '', description: '', rationale: '', resolution: '', tags: ''
        };

        if(note && note.length>0){
          form.description = note;
        }

        // open overlay TUI
        console.log('[log_diario_wizard] about to open ctx.ui.custom');
        const result = await ctx.ui.custom((tui, theme, keybindings, done) => {
          console.log('[log_diario_wizard] ctx.ui.custom renderer created');
          // Use a very small interactive wizard built from primitives
          let step = 0;

          function renderStep(): string[] {
            const w = tui.getWidth ? tui.getWidth() : 80;
            const lines: string[] = [];
            lines.push(theme.fg("accent", `Log Diario — Wizard (paso ${step+1}/7)`));
            lines.push('');
            switch(step){
              case 0:
                lines.push('Bienvenido. Pulsa Enter para empezar o Esc para cancelar.');
                break;
              case 1:
                lines.push('Step 1 — Identificación básica');
                lines.push('Title: ' + form.title);
                lines.push('Domain: ' + form.domain);
                lines.push('Type: ' + form.type);
                break;
              case 2:
                lines.push('Step 2 — Fecha y contexto');
                lines.push('Date (YYYY-MM-DD): ' + form.date);
                break;
              case 3:
                lines.push('Step 3 — Participantes y relaciones');
                lines.push('Participants: ' + form.participants);
                lines.push('Related IDs: (agregar en Preview si aplica)');
                break;
              case 4:
                lines.push('Step 4 — Archivos / scope');
                lines.push('Files (globs, coma-sep): ' + form.files);
                break;
              case 5:
                lines.push('Step 5 — Contenido y metadatos');
                lines.push('Description:');
                lines.push('');
                lines.push(...form.description.split('\n').slice(0,10).map(l=>truncateToWidth(l, w)));
                break;
              case 6:
                lines.push('Step 6 — Preview');
                try{
                  const cmd = buildMlCommand(form as any);
                  lines.push('ML command:');
                  lines.push(cmd);
                }catch(e){ lines.push('No se pudo construir el comando: '+String(e)); }
                lines.push('');
                lines.push('Actions: Enter = Confirm & Generate commands, Ctrl+E = Execute ml record, Esc = Cancel');
                break;
            }
            return lines.map(l=>truncateToWidth(l, w));
          }

          const comp = {
            render: (width: number) => renderStep(),
            invalidate: () => {},
            handleInput: (data: string) => {
              // basic key handling
              if(data === "\x1b"){ // Esc
                done(undefined);
                return;
              }
              if(data === "\r" || data === "\n"){ // Enter: advance or confirm
                if(step === 0) { step = 1; tui.requestRender(); return; }
                if(step >=1 && step <6){ step++; tui.requestRender(); return; }
                if(step ===6){ // generate commands (return them)
                  const cmd = buildMlCommand(form as any);
                  done({ ml_command: cmd, form });
                  return;
                }
              }
              // Ctrl+E execute
              if(data === "\x05"){ // Ctrl+E
                if(step ===6){ // execute ml record
                  const cmd = buildMlCommand(form as any);
                  try{
                    const { spawnSync } = require('child_process');
                    const parts = String(cmd).split(/\s+/);
                    const p = spawnSync(parts[0], parts.slice(1), { encoding: 'utf-8' });
                    done({ executed: true, stdout: p.stdout, stderr: p.stderr, status: p.status });
                    return;
                  }catch(e){ done({ error: String(e) }); return; }
                }
              }
              // simple inline edits: if user types text, append to relevant field for demo
              // For production we'd implement focused Input components per field. Here minimal: typing while in a step appends to a field string, Backspace deletes.
              if(data && data.length ===1 && data.charCodeAt(0) >=32){
                // append to current field
                if(step===1) form.title += data;
                else if(step===2) form.date += data;
                else if(step===3) form.participants += data;
                else if(step===4) form.files += data;
                else if(step===5) form.description += data;
                tui.requestRender();
              }
              // backspace
              if(data === "\x7f"){ // DEL
                if(step===1) form.title = form.title.slice(0,-1);
                else if(step===2) form.date = form.date.slice(0,-1);
                else if(step===3) form.participants = form.participants.slice(0,-1);
                else if(step===4) form.files = form.files.slice(0,-1);
                else if(step===5) form.description = form.description.slice(0,-1);
                tui.requestRender();
              }
            }
          };

          // request initial render
          setTimeout(()=> tui.requestRender(), 20);
          return comp;
        }, { overlay: true, overlayOptions: { width: '60%', minWidth: 60 } });

        // result handling
        if(!result) return { content: [{ type:'text', text: 'Wizard cancelled' }] };
        return { content: [{ type:'text', text: JSON.stringify(result) }] };
      }
    });
  }catch(e){ /* ignore */ }

  // also expose command to open wizard
  try{
    pi.registerCommand('log_diario_wizard', {
      description: 'Abrir wizard TUI para crear registros Mulch',
      handler: async (args, ctx) => {
        try{
          console.log('[log_diario_wizard] command handler invoked, args=', args);
          await pi.callTool?.('log_diario_wizard', { note: args || '' } as any);
          console.log('[log_diario_wizard] pi.callTool returned');
        }catch(e){ console.error('[log_diario_wizard] error in command handler', e); try{ await pi.sendUserMessage?.('[log_diario_wizard] error invoking tool: '+String(e)); }catch{} }
      }
    });
  }catch(e){}
}
