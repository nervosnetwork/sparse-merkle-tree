// build.ts
import { spawnSync } from 'node:child_process';
import { createRequire } from 'node:module';
import fs from 'node:fs';
import path from 'node:path';
import { build as esbuild } from 'esbuild';

const require = createRequire(import.meta.url);

function findPackageRootFromEntry(entryPath: string, pkgName: string): string {
  let dir = path.dirname(entryPath);
  const root = path.parse(dir).root;

  while (true) {
    const pj = path.join(dir, 'package.json');
    if (fs.existsSync(pj)) {
      try {
        const json = JSON.parse(fs.readFileSync(pj, 'utf8'));
        if (json && json.name === pkgName) {
          return dir;
        }
      } catch {
        // ignore JSON parse errors and keep walking up
      }
    }
    if (dir === root) break;
    dir = path.dirname(dir);
  }
  throw new Error(`Cannot locate package root for ${pkgName} starting from ${entryPath}`);
}

function resolveCkbJsVmPath(): string {
  const entry = require.resolve('ckb-testtool');
  const pkgRoot = findPackageRootFromEntry(entry, 'ckb-testtool');
  return path.join(pkgRoot, 'src/unittest/defaultScript/ckb-js-vm');
}

function runTSCNoEmit(): void {
  let r;
  try {
    const tsPkgJson = require.resolve('typescript/package.json');
    const tsBin = path.join(path.dirname(tsPkgJson), 'bin', 'tsc');
    r = spawnSync(process.execPath, [tsBin, '--noEmit'], { stdio: 'inherit' });
  } catch {
    r = spawnSync('tsc', ['--noEmit'], {
      stdio: 'inherit',
      shell: process.platform === 'win32',
    });
  }
  if (r.status !== 0) {
    throw new Error(`tsc --noEmit failed with code ${r.status}`);
  }
}

async function runEsbuild(): Promise<void> {
  await esbuild({
    platform: 'neutral',
    minify: true,
    bundle: true,
    target: 'es2022',
    entryPoints: ['src/index.ts'],
    outfile: 'dist/ckb-test-smt-wasm.js',
    external: ['@ckb-js-std/bindings'],
  });
}

function runCkbDebugger(ckbJsVmPath: string): void {
  const args = [
    '--read-file',
    'dist/ckb-test-smt-wasm.js',
    '--bin',
    ckbJsVmPath,
    '--',
    '-c',
    'dist/ckb-test-smt-wasm.bc',
  ];
  const r = spawnSync('ckb-debugger', args, {
    stdio: 'inherit',
    shell: process.platform === 'win32', // 兼容 Windows
  });
  if (r.status !== 0) {
    throw new Error(`ckb-debugger failed with code ${r.status}`);
  }
}

async function main() {
  const ckbJsVmPath = resolveCkbJsVmPath();
  console.log('[build] ckb-js-vm path:', ckbJsVmPath);

  runTSCNoEmit();
  await runEsbuild();
  runCkbDebugger(ckbJsVmPath);

  console.log('[build] done.');
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
