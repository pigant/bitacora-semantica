import type { ExtensionAPI, ExtensionContext } from "@mariozechner/pi-coding-agent";
import { Type } from "@sinclair/typebox";
import { Text } from "@mariozechner/pi-tui";
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
        // set persistent status while wizard is active
        try{ ctx.ui.setStatus && ctx.ui.setStatus('log_diario_wizard', 'Wizard activo'); }catch{}
        const result = await ctx.ui.custom((tui, theme, keybindings, done) => {
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
                    // Ask for confirmation before executing
                    (async ()=>{
                      try{
                        const confirmed = await ctx.ui.confirm?.('Confirmar ejecución', `Ejecutar: ${cmd}\n\n¿Continuar?`);
                        if(!confirmed){ done({ executed: false, cancelled: true }); return; }

                        // Parse command naively; consider improving parsing in future
                        const parts = String(cmd).split(/\s+/);
                        const bin = parts[0];
                        const args = parts.slice(1);

                        // Use a BorderedLoader during execution so user can cancel
                        const loaderResult = await ctx.ui.custom((tui2, theme2, _kb2, done2) => {
                          const { BorderedLoader } = require('@mariozechner/pi-coding-agent');
                          const loader = new BorderedLoader(tui2, theme2, 'Ejecutando...');
                          loader.onAbort = () => { done2({ cancelled: true }); };

                          (async () => {
                            try{
                              const r = await pi.exec(bin, args, { signal });
                              done2({ executed: true, stdout: r.stdout, stderr: r.stderr, status: r.code });
                            }catch(e){ done2({ error: String(e) }); }
                          })();

                          return loader;
                        }, { overlay: true, overlayOptions: { width: '40%', minWidth: 40, anchor: 'center', margin: 1 } });

                        done(loaderResult);
                      }catch(e){ done({ error: String(e) }); }
                    })();
                    return;
                  }catch(e){ done({ error: String(e) }); return; }
                }
              }
              // Use simple Input/Edit modal per-field for better UX (non-invasive)
              // When entering a step that requires text, open a small Input editor
              const openFieldEditor = async (fieldName: string, initial: string, multiline = false) => {
                const res = await ctx.ui.custom((tui2, theme2, _kb2, done2) => {
                  const { Editor, Input, Text } = require('@mariozechner/pi-tui');
                  const comp = multiline ? new Editor(initial) : new Input(initial);
                  // wrap in minimal container-like object
                  return {
                    render: (w:number) => {
                      const lines: string[] = [];
                      lines.push(theme2.fg('accent', `Editar: ${fieldName}`));
                      lines.push('');
                      const content = comp.render ? comp.render(w) : [String(initial)];
                      return [...lines, ...content, '', theme2.fg('dim', 'Enter = aceptar • Esc = cancelar')];
                    },
                    invalidate: () => { if(comp.invalidate) comp.invalidate(); },
                    handleInput: (d:string) => {
                      if (comp.handleInput) comp.handleInput(d);
                      // Enter in single-line Input should commit
                      if(d === '\r' || d === '\n'){
                        const value = comp.getValue ? comp.getValue() : undefined;
                        done2(value ?? initial);
                      }
                      if(d === '\x1b') { done2(undefined); }
                      tui2.requestRender();
                    }
                  };
                }, { overlay: true, overlayOptions: { width: '50%', minWidth: 40, anchor: 'center', margin: 1 } });
                return res;
              };

              if(data && data.length ===1 && data.charCodeAt(0) >=32){
                // If user types printable, open field editor for the current step
                if(step===1){ (async ()=>{ const v = await openFieldEditor('Title', form.title, false); if(typeof v === 'string') { form.title = v; tui.requestRender(); } })(); }
                else if(step===2){ (async ()=>{ const v = await openFieldEditor('Date (YYYY-MM-DD)', form.date, false); if(typeof v === 'string') { form.date = v; tui.requestRender(); } })(); }
                else if(step===3){ (async ()=>{ const v = await openFieldEditor('Participants', form.participants, false); if(typeof v === 'string') { form.participants = v; tui.requestRender(); } })(); }
                else if(step===4){ (async ()=>{ const v = await openFieldEditor('Files (globs)', form.files, false); if(typeof v === 'string') { form.files = v; tui.requestRender(); } })(); }
                else if(step===5){ (async ()=>{ const v = await openFieldEditor('Description', form.description, true); if(typeof v === 'string') { form.description = v; tui.requestRender(); } })(); }
              }
              // backspace handled inside editors now
              if(data === "\x7f"){ /* ignore */ }
            }
          };

          // request initial render
          setTimeout(()=> tui.requestRender(), 20);
          return comp;
        }, { overlay: true, overlayOptions: { width: '60%', minWidth: 60, maxHeight: '70%', anchor: 'center', margin: 1 } });
        // clear status after wizard closes
        try{ ctx.ui.setStatus && ctx.ui.setStatus('log_diario_wizard', undefined); }catch{}

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
          // invoke wizard by sending the slash command as a user message
          await pi.sendUserMessage?.('/log_diario_wizard ' + (args || ''), { deliverAs: 'nextTurn' } as any);
        }catch(e){ try{ ctx.ui.notify && ctx.ui.notify('No se pudo invocar el wizard automáticamente. Ejecuta /log_diario_wizard manualmente.', 'warning'); }catch{} }
      }
    });
  }catch(e){}
}
