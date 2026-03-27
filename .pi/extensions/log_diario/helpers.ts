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
