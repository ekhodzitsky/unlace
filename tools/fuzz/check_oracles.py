#!/usr/bin/env python3
"""Check writer-oracle sensitivity against intentional isolated source mutations."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

from run import PROJECT, checked, child_limits
from writer_seeds import header, block


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', required=True, type=Path, help='fresh directory below runs/')
    args = parser.parse_args()
    root = args.out.resolve()
    if not root.is_relative_to(PROJECT / 'runs') or root == PROJECT / 'runs':
        parser.error('output must be a fresh directory below project runs/')
    root.mkdir(parents=True, exist_ok=False)
    source = (PROJECT / 'src/write/mod.rs').read_text()
    begin = ('    fn write_block(&mut self, b: EncodedBlock<\'_>) -> Result<(), WriteError> {\n'
             '        self.out.begin()?;')
    controls = {
        'clean': None,
        'false-refusal': (begin, begin + '\n        if b.track_number != 0 {\n'
                          '            return Err(WriteError::Invalid("intentional false refusal"));\n        }'),
        'payload': ('self.pending\n                    .raw(b.frames.get(i).ok_or(WriteError::Invalid("frame bounds"))?)?;',
                    'let mut changed = b.frames.get(i).ok_or(WriteError::Invalid("frame bounds"))?.to_vec();\n'
                    '                if let Some(first) = changed.first_mut() { *first ^= 1; }\n'
                    '                self.pending.raw(&changed)?;'),
        'live-finish': ('        self.core.out.state = WriteState::Finished;',
                        '        // Intentional missing terminal transition.'),
        'indexed-finish': ('        c.out.state = WriteState::Finished;',
                           '        // Intentional missing terminal transition.'),
        'wrong-failure-state': ('fn fail(&mut self, error: io::Error) -> WriteError {\n        self.state = WriteState::Failed;',
                                'fn fail(&mut self, error: io::Error) -> WriteError {\n        self.state = WriteState::Finished;'),
        'preflight-poison': (begin, begin + '\n        if b.track_number == 3 {\n'
                             '            self.out.state = WriteState::Failed;\n'
                             '            return Err(WriteError::Invalid("intentional preflight poison"));\n        }'),
        'state': ('fn fail(&mut self, error: io::Error) -> WriteError {\n        self.state = WriteState::Failed;',
                  'fn fail(&mut self, error: io::Error) -> WriteError {'),
    }
    compiler = ['rustc', '+1.89.0', '--edition=2024', '-C', 'opt-level=1',
                '-C', 'overflow-checks=yes', '-D', 'warnings', '-F', 'unsafe_code']
    manifest = {'rustc': checked(['rustc', '+1.89.0', '-Vv']).decode(),
                'source_sha256': {str(p.relative_to(PROJECT)): hashlib.sha256(p.read_bytes()).hexdigest()
                                  for d in ['src', 'tools/fuzz']
                                  for p in sorted((PROJECT / d).rglob('*'))
                                  if p.is_file() and p.suffix in ['.rs', '.py']},
                'note': 'Intentional mutants; not production findings or coverage campaigns.'}
    (root / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    results = []
    for name, mutation in controls.items():
        work = root / name
        shutil.copytree(PROJECT / 'src', work / 'src')
        if mutation:
            old, new = mutation
            if source.count(old) != 1:
                raise ValueError('control requires exactly one source match: ' + name)
            (work / 'src/write/mod.rs').write_text(source.replace(old, new))
        library, target = work / 'libunlace.rlib', work / 'target'
        checked(compiler + ['--crate-name', 'unlace', '--crate-type', 'rlib',
                            str(work / 'src/lib.rs'), '-o', str(library)])
        checked(compiler + ['--cfg', 'test', str(PROJECT / 'tools/fuzz/write_target.rs'),
                            '--extern', 'unlace=' + str(library), '-o', str(target)])
        control = header()
        if name in ['state', 'wrong-failure-state']:
            control[1], control[2] = 1, 2  # Third write, after successful construction.
        data = bytes(control) + block() + block(20) + bytes([1]) + block(40) + bytes([2])
        if name == 'preflight-poison':
            data = bytes(control) + block(track=2) + block() + bytes([2])
        result = subprocess.run([str(target)], input=data, capture_output=True,
                                timeout=3, preexec_fn=child_limits)
        (work / 'stderr.txt').write_bytes(result.stderr)
        expected = 0 if name == 'clean' else 101
        results.append({'control': name, 'exit': result.returncode, 'expected': expected,
                        'detected': result.returncode == expected})
        (root / 'results.json').write_text(json.dumps(results, indent=2) + '\n')
        if result.returncode != expected:
            raise AssertionError('writer oracle control failed: ' + name)
    print(json.dumps(results))


if __name__ == '__main__':
    main()
