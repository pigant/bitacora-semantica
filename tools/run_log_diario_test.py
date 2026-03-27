#!/usr/bin/env python3
"""
Script to automate testing of the log_diario prompt via `pi` CLI.

Usage:
  ./run_log_diario_test.py --note-file note.txt --out out.json

What it does:
- Reads a note text (or uses a built-in example)
- Builds the enriched prompt with placeholders NOTE_DATE and RELATED_HINTS (empty summary by default)
- Invokes `pi --mode json -p <prompt> --no-session --no-extensions --rpc`
- Captures stdout/stderr, saves them to files and attempts to extract the first JSON ARRAY found in the output
- Writes the cleaned JSON array to the specified output file (or prints it)

Requirements: python3 on PATH, `pi` available in PATH.
"""

import argparse
import subprocess
import sys
import tempfile
import re
import json
from pathlib import Path

COLLECTOR_PROMPT_TEMPLATE = r"""
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
  - related: array (posible vacío) de objetos {{ID_PLACEHOLDER}} enlazando eventos Mulch relacionados.
  - diagnostics: objeto opcional con información de resolución (ej: date_conflict).

Reglas estrictas:
1) date debe ser ISO YYYY-MM-DD cuando sea posible; si la nota usa términos relativos (ayer, antes de ayer, la semana pasada), resuélvelos usando la fecha de referencia proporcionada más abajo; si no está claro, devuelve "".
2) domain sólo permite [a-z0-9-]; si el contexto proporciona dominios relacionados ({{RELATED_HINTS}}), úsalos preferentemente.
3) Si no hay archivos mencionados, files debe ser "".
4) El ml_command debe tener la forma:
   ml record <domain> --type <type> --name "<title>" --description "<description>" --files "<files>"
5) NO incluyas texto fuera del JSON. Si no hay objetos relevantes, devuelve [].

Contexto adicional (hints de Mulch):
{RELATED_HINTS}

Fecha de referencia (si existe): {NOTE_DATE}

Nota del usuario:
{NOTE_BODY}
"""

DEFAULT_NOTE = """Para el jueves 26 de marzo de 2026
SAI: Maria Nicole avisa a Paolo Castillo que hay unas garantias que no estan cruzando bien desde el reporte de Cesar Follert (Se cambia garantia por robo por una normal). Paolo queda de revisar que puede estar pasando
general: Me reuno con Marcia Luengo de WITI para hacer una evaluación de desempeño de todos los integrantes WITI en el equipo
Pos movil: Me junto con Roberto Valenzuela, Silvana Vitali y Patricio Villarroel, a revisar los avances de pos movil, quedamos en que hay que hacer algunas mejoras: Mostrar garantias en el historial de ordenes;
mostrar en el control panel
POS Movil: tambien de la reunion se descubre que Sales Orders tiene problemas con su graphql de staging para mostrar las ordenes que contemplen garantias, se les entrego la informacion y quedaron en reparar"""


JSON_ARRAY_RE = re.compile(r"\[\s*\{[\s\S]*?\}\s*\]", re.MULTILINE)


def run_pi_with_prompt(prompt: str, timeout: int = 120) -> (str, str, int):
    cmd = ["pi", "--mode", "json", "-p", prompt, "--no-session", "--no-extensions", "--rpc"]
    # run via subprocess and capture output
    proc = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, timeout=timeout)
    return proc.stdout, proc.stderr, proc.returncode


def extract_json_array(text: str):
    # Find all candidate JSON arrays in the stream and pick the most likely one
    candidates = re.findall(r"\[\s*\{[\s\S]*?\}\s*\]", text, re.MULTILINE)
    valid = []
    for c in candidates:
        try:
            parsed = json.loads(c)
            # accept arrays of objects that look like proposals (have domain/title or date)
            if isinstance(parsed, list) and parsed:
                score = 0
                for it in parsed[:3]:
                    if isinstance(it, dict):
                        if 'domain' in it or 'title' in it or 'date' in it:
                            score += 1
                if score>0:
                    valid.append((score, parsed))
                else:
                    valid.append((0, parsed))
        except Exception:
            # try minor cleanup: remove trailing commas
            s2 = re.sub(r",\s*([}\]])", r"\1", c)
            try:
                parsed = json.loads(s2)
                if isinstance(parsed, list):
                    valid.append((0, parsed))
            except Exception:
                continue
    if not valid:
        return None
    # pick highest score, tie -> last occurrence
    valid.sort(key=lambda x: x[0])
    return valid[-1][1]


def main():
    p = argparse.ArgumentParser()
    p.add_argument('--note-file', '-n', type=Path, help='Path to file with the note to test')
    p.add_argument('--out', '-o', type=Path, default=Path('logdiario_test_out.json'), help='Where to write parsed JSON array')
    p.add_argument('--related', '-r', default='', help='RELATED_HINTS string (optional)')
    p.add_argument('--date', '-d', default='2026-03-26', help='Reference date to include in prompt (optional)')
    p.add_argument('--raw-out', default='/tmp/logdiario_raw.out', help='Save raw process output')
    p.add_argument('--with-prime', action='store_true', help='Run ml prime and include minimal output in prompt')
    p.add_argument('--with-status', action='store_true', help='Run ml status and include minimal output in prompt')
    p.add_argument('--prime-out-file', default='', help='Save raw ml prime output to file')
    p.add_argument('--status-out-file', default='', help='Save raw ml status output to file')
    args = p.parse_args()

    note = DEFAULT_NOTE
    if args.note_file:
        note = args.note_file.read_text(encoding='utf-8')

    # Optionally run ml prime/status to provide minimal context (kept as small snippets)
    prime_out = ''
    status_out = ''
    if args.with_prime:
        try:
            pprime = subprocess.run(['ml','prime'], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, timeout=60)
            prime_out = (pprime.stdout or '') + (('\n[ml prime error] ' + pprime.stderr) if pprime.stderr else '')
            if args.prime_out_file:
                Path(args.prime_out_file).write_text(prime_out, encoding='utf-8')
            # keep minimal: first 2000 chars
            prime_out = (prime_out or '')[:2000]
        except Exception:
            prime_out = ''
    if args.with_status:
        try:
            pst = subprocess.run(['ml','status'], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, timeout=20)
            status_out = (pst.stdout or '') + (('\n[ml status error] ' + pst.stderr) if pst.stderr else '')
            if args.status_out_file:
                Path(args.status_out_file).write_text(status_out, encoding='utf-8')
            status_out = (status_out or '')[:2000]
        except Exception:
            status_out = ''

    prompt = COLLECTOR_PROMPT_TEMPLATE.replace('{RELATED_HINTS}', args.related or '').replace('{NOTE_DATE}', args.date or '').replace('{NOTE_BODY}', note)
    if prime_out:
        prompt += '\n\n[ML PRIME OUTPUT]\n' + prime_out
    if status_out:
        prompt += '\n\n[ML STATUS]\n' + status_out

    print('Invoking pi CLI...')
    stdout, stderr, code = run_pi_with_prompt(prompt)

    # save raw outputs
    Path(args.raw_out).write_text(stdout + '\n\n=== STDERR ===\n\n' + stderr, encoding='utf-8')

    if code != 0:
        print('pi exited with code', code, file=sys.stderr)
        print('stderr:\n', stderr, file=sys.stderr)

    parsed = extract_json_array(stdout)
    if parsed is None:
        print('No clean JSON array found in pi output. Raw output saved to', args.raw_out, file=sys.stderr)
        sys.exit(2)

    # post-process: prefer the textual assistant output and ignore thinking/signature
    final_obj = None
    try:
        # if parsed is an array of assistant events (with 'type' and 'text'), prefer concatenated text fields of type 'text'
        if isinstance(parsed, list):
            text_parts = []
            for it in parsed:
                if isinstance(it, dict) and it.get('type') == 'text' and it.get('text'):
                    text_parts.append(it.get('text'))
            joined = '\n'.join(text_parts).strip()
            if joined:
                # try extract JSON array from joined text
                m = JSON_ARRAY_RE.search(joined)
                if m:
                    try:
                        final_obj = json.loads(m.group(0))
                    except Exception:
                        # fallback: keep joined as single text object
                        final_obj = joined
                else:
                    # maybe joined already is the array literal
                    try:
                        cand = json.loads(joined)
                        final_obj = cand
                    except Exception:
                        final_obj = joined
        if final_obj is None:
            final_obj = parsed
    except Exception:
        final_obj = parsed

    # pretty write final result
    if isinstance(final_obj, (list, dict)):
        args.out.write_text(json.dumps(final_obj, ensure_ascii=False, indent=2), encoding='utf-8')
    else:
        # write textual fallback
        args.out.write_text(str(final_obj), encoding='utf-8')
    print('Extracted JSON array saved to', str(args.out))


if __name__ == '__main__':
    main()
