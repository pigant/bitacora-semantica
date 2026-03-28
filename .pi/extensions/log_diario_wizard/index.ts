import type { ExtensionAPI } from "@mariozechner/pi-coding-agent";
import registerWizard from './wizard_impl.ts';

export default function(pi: ExtensionAPI){
  console.log('[log_diario_wizard] registering wizard extension');
  try{ registerWizard(pi); console.log('[log_diario_wizard] registerWizard executed'); }catch(e){ console.error('[log_diario_wizard] error registering wizard', e); try{ pi.sendUserMessage?.('[log_diario_wizard] error registering wizard: '+String(e)); }catch{} }
}
