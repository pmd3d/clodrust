#!/usr/bin/env python3
"""Verify production bodies, reproduce historical bugs, or test proof sensitivity."""
import argparse
import json
from pathlib import Path
import os
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
PIN = json.loads((ROOT / 'verification/toolchain.json').read_text())
MUTATIONS = {
    'narrowing': ('Some(value as i32)', 'Some((value as i32) & 255)'),
    'array-size': ('count as i128 * element_size as i128)', 'count as i128 * element_size as i128 + 1)'),
    'rounding': ('let result = quotient * a;', 'let result = magnitude;'),
    'stack-slot': ('current as i64 - size as i64;', 'current as i64;'),
    'struct-offset': ('offsets.push(offset);', 'offsets.push(0);'),
    'frame-saves': ('let saved = saved_count as i128 * 8;', 'let saved = saved_count as i128 * 4;'),
    'pointer-offset': ('index as i128 * scale as i128)', 'index as i128)'),
}

def activate(source):
    """Only uncomment proof annotations and name returns; never rewrite execution."""
    source = '\n'.join(line.replace('// verus: ', '', 1)
                       if line.lstrip().startswith('// verus: ') else line
                       for line in source.splitlines())
    return re.sub(r'(?m)(^pub fn \w+\([\s\S]*?\)\s*->\s*)(Option<[\s\S]*?>)(?=\s*// verus: ensures|\s*ensures)',
                  r'\1(result: \2)', source)

def generate(source, target):
    lemmas = (ROOT / 'verification/lemmas.rs').read_text()
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text('use vstd::prelude::*;\nverus! {\nmod checked_layout {\nuse vstd::prelude::*;\n'
                      + activate(source).replace('//!', '//') + '\n' + lemmas
                      + '\n}\nfn main() {}\n}\n')

def command(verus, target):
    return [verus, '--multiple-errors', '8', '--rlimit', '30', str(target)]

def expect_failure(verus, target):
    result = subprocess.run([verus, '--multiple-errors', '1', '--rlimit', '30', str(target)], text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    print(result.stdout, end='')
    # Syntax/type errors, missing tools, and solver exhaustion do not count as bugs.
    failed_obligations = re.search(r'verification results:: \d+ verified, ([1-9]\d*) errors', result.stdout)
    proper_diagnostic = any(s in result.stdout for s in
                            ('postcondition not satisfied', 'invariant not satisfied',
                             'assertion failed', 'possible arithmetic underflow/overflow',
                             'precondition not satisfied'))
    if result.returncode != 1 or not failed_obligations or not proper_diagnostic or 'Resource limit' in result.stdout:
        print(f'FAIL: expected a verification failure, got exit {result.returncode}: {target.name}', file=sys.stderr)
        return False
    print(f'Caught expected bug: {target.stem}', flush=True)
    return True

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    default = ROOT / '.verus/verus-x86-linux/verus'
    parser.add_argument('--verus', default=os.environ.get('VERUS', str(default) if default.exists() else 'verus'))
    parser.add_argument('--emit-only', action='store_true', help='Generate the exact Verus input without running it')
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument('--bugs', action='store_true', help='Require historical bug fixtures to fail verification')
    mode.add_argument('--mutations', action='store_true', help='Require seven mutations of production bodies to fail')
    args = parser.parse_args()
    # Managed workspaces can store rustup beside the checkout rather than HOME.
    # Preserve explicit user choices; this fallback does nothing on normal hosts.
    workspace_rustup = ROOT.parent / '.rustup'
    workspace_cargo = ROOT.parent / '.cargo'
    if not (Path.home() / '.rustup').exists() and workspace_rustup.is_dir():
        os.environ.setdefault('RUSTUP_HOME', str(workspace_rustup))
        os.environ.setdefault('CARGO_HOME', str(workspace_cargo))
        os.environ['PATH'] = str(workspace_cargo / 'bin') + os.pathsep + os.environ.get('PATH', '')
    source = (ROOT / 'src/util/CheckedLayout.rs').read_text()
    target = ROOT / 'target/verification/checked_layout.rs'
    generate(source, target)
    if args.emit_only:
        print(target)
        return 0
    try:
        version = subprocess.run([args.verus, '--version'], text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        if version.returncode or f"Version: {PIN['verus_version']}" not in version.stdout:
            print(version.stdout, end='')
            print(f"Expected Verus {PIN['verus_version']}. Run verification/install.sh.", file=sys.stderr)
            return 2
        if args.bugs:
            results = [expect_failure(args.verus, path) for path in sorted((ROOT / 'verification/bugs').glob('*.rs'))]
            return 0 if results and all(results) else 1
        print('Verifying production bodies from src/util/CheckedLayout.rs', flush=True)
        baseline = subprocess.call(command(args.verus, target))
        if baseline or not args.mutations:
            return baseline
        results = []
        for name, (old, new) in MUTATIONS.items():
            if source.count(old) != 1:
                print(f'Mutation {name} needs updating: expected exactly one matching body fragment.', file=sys.stderr)
                return 2
            mutant = ROOT / f'target/verification/mutations/{name}.rs'
            generate(source.replace(old, new, 1), mutant)
            results.append(expect_failure(args.verus, mutant))
        return 0 if all(results) else 1
    except FileNotFoundError:
        print('Verus not found. Run verification/install.sh, then use --verus <path> if needed.', file=sys.stderr)
        return 2

if __name__ == '__main__':
    sys.exit(main())
