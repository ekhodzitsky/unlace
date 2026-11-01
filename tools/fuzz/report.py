#!/usr/bin/env python3
"""Replay a retained corpus and report library-only LLVM source coverage."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
from run import PROJECT, checked, child_limits, read_input


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('campaign', type=Path)
    parser.add_argument('--llvm-tools', required=True, type=Path)
    args = parser.parse_args()
    campaign = args.campaign.resolve()
    llvm = args.llvm_tools.resolve()
    os.chdir(PROJECT)
    manifest = json.loads((campaign / 'manifest.json').read_text())
    for relative, expected in manifest['source_sha256'].items():
        if relative.startswith('src/'):
            actual = hashlib.sha256((PROJECT / relative).read_bytes()).hexdigest()
            if actual != expected:
                raise ValueError('source changed since build: ' + relative)
    destination = campaign / 'report'
    destination.mkdir(exist_ok=False)
    profiles = []
    seen = set()
    for path in sorted((campaign / 'corpus').iterdir()):
        data = read_input(path)
        key = hashlib.sha256(data).hexdigest()
        if key in seen:
            continue
        seen.add(key)
        profile = destination / (key + '.profraw')
        environment = dict(os.environ, LLVM_PROFILE_FILE=str(profile))
        subprocess.run([str(campaign / 'target')], input=data, check=True,
                       capture_output=True, timeout=3, env=environment,
                       preexec_fn=child_limits)
        profiles.append(str(profile))
    indexed = destination / 'coverage.profdata'
    checked([str(llvm / 'llvm-profdata'), 'merge', '-sparse', *profiles, '-o', str(indexed)])
    raw = checked([str(llvm / 'llvm-cov'), 'export', str(campaign / 'target'),
                   '-instr-profile=' + str(indexed), '--skip-functions', '--skip-expansions'])
    (destination / 'coverage.json').write_bytes(raw)
    rows = []
    totals = {kind: dict(count=0, covered=0) for kind in ('regions', 'lines', 'functions')}
    for unit in json.loads(raw)['data']:
        for entry in unit['files']:
            source = Path(entry['filename']).resolve()
            if not source.is_relative_to(PROJECT / 'src'):
                continue
            row = dict(file=str(source.relative_to(PROJECT / 'src')))
            for kind, total in totals.items():
                row[kind] = entry['summary'][kind]
                for field in total:
                    total[field] += row[kind][field]
            rows.append(row)
    summary = dict(replayed=len(seen), files=rows, totals=totals,
                   note='Source region/line/function coverage, not branch coverage or safety proof.')
    (destination / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
    print(json.dumps(summary))


if __name__ == '__main__':
    main()
