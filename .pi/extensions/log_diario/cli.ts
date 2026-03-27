import { spawnSync } from 'child_process';

export function runPiCli(args: string[], env = process.env, timeout = 120000){
  const proc = spawnSync('pi', args, { encoding: 'utf-8', env, stdio: ['ignore','pipe','pipe'], timeout });
  return { stdout: proc.stdout?.toString() || '', stderr: proc.stderr?.toString() || '', status: proc.status };
}

export function runMl(cmd: string[]){
  const p = spawnSync('ml', cmd, { encoding: 'utf-8', env: process.env, stdio: ['ignore','pipe','pipe'] });
  return { stdout: p.stdout?.toString() || '', stderr: p.stderr?.toString() || '', status: p.status, error: p.error };
}
