#!/usr/bin/env python3
"""
Convert existing /tmp and logs log files to logs/ with ISO-8601 timestamps.

Usage:
  python3 tools/convert_logs_to_iso.py

What it does:
- Looks for a set of known log files under /tmp and ./logs
- For each existing source file, writes (appends) a converted version into ./logs/<basename>
  where each input line that does NOT already start with an ISO timestamp is prefixed with
  an UTC ISO8601 timestamp with millisecond precision: 2026-03-31T12:34:56.123Z

This is safe to run multiple times; it will append converted lines to the target file.
"""

from pathlib import Path
from datetime import datetime, timezone
import re

# sources to consider (in order). If present in /tmp they will be converted; logs/ files are left as-is.
SRC_CANDIDATES = [
    Path('/tmp/inference_local_matches.log'),
    Path('/tmp/inference_agent_end.log'),
    Path('/tmp/inference_followup.log'),
    Path('/tmp/inference_agent_end_raw_text.log'),
    Path('/tmp/pi_rpc_test.log'),
    Path('/tmp/pi_rpc_ping_full.log'),
]

LOG_DIR = Path('logs')
LOG_DIR.mkdir(parents=True, exist_ok=True)

# regex to heuristically detect an ISO timestamp at line start (e.g. [2026-03-31T12:34:56.123Z]) or ISO without brackets
ISO_PREFIX_RE = re.compile(r"^\[?\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,6})?Z\]?\s*")


def iso_now_utc():
    # returns ISO 8601 with milliseconds and trailing Z
    return datetime.now(timezone.utc).isoformat(timespec='milliseconds').replace('+00:00', 'Z')


def convert_file(src: Path, dst: Path):
    if not src.exists():
        return 0
    count = 0
    with src.open('r', encoding='utf-8', errors='replace') as sf, dst.open('a', encoding='utf-8') as df:
        for line in sf:
            raw = line.rstrip('\n')
            if not raw.strip():
                # write blank lines as-is (optionally with timestamp?) keep blank
                df.write('\n')
                continue
            if ISO_PREFIX_RE.match(raw):
                # already has ISO-like prefix -> write as-is
                df.write(raw + '\n')
            else:
                df.write(f"[{iso_now_utc()}] {raw}\n")
            count += 1
    return count


def main():
    total = 0
    for src in SRC_CANDIDATES:
        dst = LOG_DIR / src.name
        n = convert_file(src, dst)
        if n:
            print(f"Converted {n} lines from {src} -> {dst}")
            total += n
    if total == 0:
        print("No candidate /tmp logs found to convert. Nothing done.")
    else:
        print(f"Converted total {total} lines. Logs are in {LOG_DIR}/")


if __name__ == '__main__':
    main()
