import type { ExtensionAPI } from "@mariozechner/pi-coding-agent";
import registerWizard from '../log_diario/wizard.ts';

export default function(pi: ExtensionAPI){
  // delegate to the wizard implementation
  try{ registerWizard(pi); }catch(e){ /* ignore */ }
}
