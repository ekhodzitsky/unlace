#!/usr/bin/env python3
"""Optional LLVM coverage-guided fuzzing; all generated state stays in runs/."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import random
import resource
import subprocess
import sys
import time

PROJECT = Path(__file__).resolve().parents[2]
TOKENS = [bytes.fromhex(value) for value in (
    '1f43b675', '1c53bb6b', '18538067', 'a3ff', 'f188ffffffffffffffff',
    '75a2888000000000000000', 'e788ffffffffffffffff',
    '2ad7b188ffffffffffffffff', 'f088ffffffffffffffff',
)]


def read_input(path):
    with path.open("rb") as stream:
        data = stream.read(8194)
    if len(data) > 8193:
        raise ValueError("fuzz input exceeds 8193 bytes: " + str(path))
    return data


def digest(data):
    return hashlib.sha256(data).hexdigest()


def child_limits():
    resource.setrlimit(resource.RLIMIT_AS, (512 * 1024 * 1024,) * 2)
    resource.setrlimit(resource.RLIMIT_CPU, (2, 2))


def checked(command, **options):
    return subprocess.run(command, check=True, capture_output=True, **options).stdout


def mutate(data, rng):
    data = bytearray(data)
    for _ in range(1 + rng.randrange(4)):
        operation = rng.randrange(9)
        if not data:
            data.append(rng.randrange(5))
        index = rng.randrange(len(data))
        if operation < 3:
            data[index] ^= 1 << rng.randrange(8)
        elif operation == 3:
            data[index] = rng.randrange(256)
        elif operation == 4:
            data[index:index] = rng.choice([b'\x00', b'\xff', b'\x80', bytes([rng.randrange(256)])])
        elif operation == 5:
            del data[index:index + 1 + rng.randrange(min(len(data) - index, 16))]
        elif operation == 6:
            token = rng.choice(TOKENS)
            data[index:index + len(token)] = token
        elif operation == 7 and len(data) > 1:
            start = rng.randrange(len(data))
            data[index:index] = data[start:start + 1 + rng.randrange(16)]
        else:
            data = data[:index]
    return bytes(data[:8193])


class Campaign:
    def __init__(self, output, llvm, rng):
        self.output = output
        self.llvm = llvm
        self.rng = rng
        self.seen = set()
        self.pool = []
        self.pool_hashes = set()
        self.runs = 0
        self.novel = 0
        self.started = time.monotonic()

    def execute(self, data):
        if len(data) > 8193:
            raise ValueError('fuzz input exceeds 8193 bytes')
        raw = self.output / 'current.profraw'
        if raw.exists():
            raw.unlink()  # Only this driver's previous transient profile.
        environment = dict(os.environ, LLVM_PROFILE_FILE=str(raw))
        try:
            result = subprocess.run(
                [str(self.output / 'target')], input=data, capture_output=True,
                timeout=3, env=environment, preexec_fn=child_limits,
            )
            return result.returncode, result.stderr
        except subprocess.TimeoutExpired as error:
            return 124, (error.stderr or b'') + b'\nprocess timeout'

    def features(self):
        profile = self.output / 'current.profdata'
        checked([str(self.llvm / 'llvm-profdata'), 'merge', '-sparse',
                 str(self.output / 'current.profraw'), '-o', str(profile)])
        report = json.loads(checked([
            str(self.llvm / 'llvm-cov'), 'export', str(self.output / 'target'),
            '-instr-profile=' + str(profile), '--skip-functions', '--skip-expansions',
        ]))
        features = set()
        for unit in report['data']:
            for entry in unit['files']:
                path = Path(entry['filename']).resolve()
                if not path.is_relative_to(PROJECT / 'src'):
                    continue
                relative = str(path.relative_to(PROJECT / 'src'))
                for line, column, count, has_count, *_ in entry['segments']:
                    if has_count and count:
                        features.add((relative, line, column))
        return features

    def add(self, data):
        key = digest(data)
        if key not in self.pool_hashes:
            self.pool.append(data)
            self.pool_hashes.add(key)

    def run(self, data):
        code, error = self.execute(data)
        self.runs += 1
        if code:
            key = digest(data)
            path = self.output / 'findings' / key
            path.with_suffix('.bin').write_bytes(data)
            path.with_suffix('.stderr').write_bytes(error)
            print(json.dumps({'finding': key, 'status': code, 'run': self.runs}), flush=True)
            raise SystemExit(1)
        features = self.features()
        if features - self.seen:
            self.seen.update(features)
            self.novel += 1
            (self.output / 'corpus' / digest(data)).write_bytes(data)
            self.add(data)
        if self.runs % 100 == 0:
            print(json.dumps(self.summary()), flush=True)

    def summary(self):
        return dict(runs=self.runs, feedback_segments=len(self.seen), novel=self.novel,
                    seconds=round(time.monotonic() - self.started, 2), pool=len(self.pool))

    @staticmethod
    def fingerprint(code, error):
        lines = error.decode(errors='replace').splitlines()
        location = next((line for line in lines if 'panicked at' in line), '')
        return code, location

    def minimize(self, data):
        code, error = self.execute(data)
        if not code:
            raise ValueError('input does not reproduce a failure')
        signature = self.fingerprint(code, error)
        original = digest(data)
        width = max(1, len(data) // 2)
        attempts = 0
        deadline = time.monotonic() + 30
        while width and time.monotonic() < deadline and attempts < 1000:
            changed = False
            for start in range(0, len(data), width):
                candidate = data[:start] + data[start + width:]
                status, message = self.execute(candidate)
                attempts += 1
                if status and self.fingerprint(status, message) == signature:
                    data, error, changed = candidate, message, True
                    break
                if time.monotonic() >= deadline or attempts >= 1000:
                    break
            if not changed:
                width //= 2
        destination = self.output / 'findings' / (original + '.min')
        destination.with_suffix('.min.bin').write_bytes(data)
        destination.with_suffix('.min.stderr').write_bytes(error)
        print(json.dumps(dict(original=original, minimized=digest(data), bytes=len(data),
                              attempts=attempts, bounded_reduction=True)), flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', required=True, type=Path, help='fresh directory below runs/')
    parser.add_argument('--llvm-tools', type=Path, required=True, help='compatible llvm-cov/profdata directory')
    parser.add_argument('--toolchain', default='1.89.0')
    parser.add_argument('--cases', type=int, default=10000)
    parser.add_argument('--seed', type=int, default=6012028)
    parser.add_argument('--replay', type=Path, help='additional input directory in the five-mode format')
    parser.add_argument('--minimize', type=Path, help='reduce a reproducing failure; no mutation campaign')
    args = parser.parse_args()
    if sys.platform != 'linux':
        parser.error('this optional driver currently qualifies Linux process limits only')
    if args.cases < 0:
        parser.error('cases must be nonnegative')
    output = args.out.resolve()
    if not output.is_relative_to(PROJECT / 'runs') or output == PROJECT / 'runs':
        parser.error('output must be a fresh directory below the ignored project runs/')
    replay = args.replay.resolve() if args.replay else None
    minimize = args.minimize.resolve() if args.minimize else None
    llvm = args.llvm_tools.resolve()
    versions = {name: checked([str(llvm / name), '--version']).decode() for name in ('llvm-cov', 'llvm-profdata')}
    compiler = ['rustc', '+' + args.toolchain]
    versions['rustc'] = checked(compiler + ['-Vv']).decode()
    versions['python'] = sys.version
    output.mkdir(parents=True, exist_ok=False)  # Never overwrite an earlier campaign.
    (output / 'corpus').mkdir()
    (output / 'findings').mkdir()
    os.chdir(PROJECT)
    source_hashes = {str(path.relative_to(PROJECT)): digest(path.read_bytes())
                     for directory in ('src', 'tools/fuzz')
                     for path in sorted((PROJECT / directory).rglob('*')) if path.is_file() and path.suffix in ('.py', '.rs')}
    manifest = dict(tool_versions=versions, source_sha256=source_hashes,
                    seed=args.seed, mutations=args.cases, input_limit=8193,
                    address_space_bytes=512 * 1024 * 1024, cpu_seconds=2, wall_seconds=3,
                    note='Source-segment feedback only. Not branch coverage or proof of safety.')
    (output / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    flags = ['--edition', '2024', '-C', 'instrument-coverage', '-C', 'opt-level=1',
             '-C', 'overflow-checks=yes', '-C', 'debuginfo=1', '-F', 'unsafe_code']
    checked(compiler + flags + ['--crate-type', 'rlib', '--crate-name', 'unlace', 'src/lib.rs',
                                '-o', str(output / 'libunlace.rlib')])
    checked(compiler + flags + ['--cfg', 'test', 'tools/fuzz/target.rs', '--extern',
                                'unlace=' + str(output / 'libunlace.rlib'), '-o', str(output / 'target')])
    campaign = Campaign(output, llvm, random.Random(args.seed))
    if minimize:
        campaign.minimize(read_input(minimize))
        return
    checked([sys.executable, 'tools/fuzz/seeds.py', str(output / 'corpus')])
    for directory in [output / 'corpus'] + ([replay] if replay else []):
        for path in sorted(directory.iterdir()):
            if path.is_file():
                campaign.add(read_input(path))
    inputs = [dict(sha256=digest(data), bytes=len(data)) for data in campaign.pool]
    (output / "inputs.json").write_text(json.dumps(inputs, indent=2) + "\n")
    for data in campaign.pool:
        (output / "corpus" / digest(data)).write_bytes(data)
    for data in list(campaign.pool):
        campaign.run(data)
    initial = campaign.runs
    for _ in range(args.cases):
        campaign.run(mutate(campaign.rng.choice(campaign.pool), campaign.rng))
    summary = dict(campaign.summary(), seed=args.seed, mutations=args.cases, seed_replays=initial)
    (output / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
    print(json.dumps(summary), flush=True)


if __name__ == '__main__':
    main()
