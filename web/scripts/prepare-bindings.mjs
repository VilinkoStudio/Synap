import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const repoRoot = path.resolve(import.meta.dirname, '..', '..');
const targetDir = path.join(repoRoot, 'target');
const outputDir = path.join(targetDir, 'generated', 'nodejs', 'synap-coreffi');
const manifestPath = path.join(repoRoot, 'coreffi', 'Cargo.toml');
const crateName = 'uniffi_synap_coreffi';
const packageName = '@synap/coreffi';

const libFile = resolveLibraryPath();

runOrThrow('cargo', ['build', '-p', 'synap-coreffi'], repoRoot);
runOrThrow(
  'uniffi-bindgen-node-js',
  [
    'generate',
    libFile,
    '--out-dir',
    outputDir,
    '--crate-name',
    crateName,
    '--package-name',
    packageName,
    '--manifest-path',
    manifestPath,
    '--manual-load'
  ],
  repoRoot
);

const copiedLibPath = path.join(outputDir, path.basename(libFile));
fs.mkdirSync(outputDir, { recursive: true });
fs.copyFileSync(libFile, copiedLibPath);

function resolveLibraryPath() {
  const debugDir = path.join(targetDir, 'debug');
  if (process.platform === 'darwin') {
    return path.join(debugDir, 'libuniffi_synap_coreffi.dylib');
  }
  if (process.platform === 'win32') {
    return path.join(debugDir, 'uniffi_synap_coreffi.dll');
  }
  return path.join(debugDir, 'libuniffi_synap_coreffi.so');
}

function runOrThrow(command, args, cwd) {
  const result = spawnSync(command, args, {
    cwd,
    stdio: 'inherit',
    env: process.env
  });

  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed with code ${result.status}`);
  }
}
