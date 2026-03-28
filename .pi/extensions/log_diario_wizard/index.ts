import type { ExtensionAPI } from "@mariozechner/pi-coding-agent";
import registerWizard from './wizard_impl.ts';

export default function(pi: ExtensionAPI){
  try{ registerWizard(pi); }catch(e){ /* ignore */ }
}
