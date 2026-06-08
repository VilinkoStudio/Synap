import { spawnSync } from 'node:child_process';
import path from 'node:path';

const repoRoot = path.resolve(import.meta.dirname, '..', '..');
const outputDir = path.join(repoRoot, 'target', 'generated', 'nodejs', 'synap-coreffi');
const sharedUdlPath = path.join(repoRoot, 'coreffi-shared', 'src', 'synap.udl');
const adapterConfigPath = path.join(repoRoot, 'coreffi', 'uniffi.toml');
const webNodeModulesDir = path.join(repoRoot, 'web', 'node_modules');

runOrThrow(
  'cargo',
  [
    'run',
    '-p',
    'xtask',
    '--',
    'gen-uniffi-node',
    '--udl',
    sharedUdlPath,
    '--config',
    adapterConfigPath,
    '--out-dir',
    outputDir,
    '--crate-name',
    'uniffi_synap_coreffi',
    '--package-name',
    '@synap/coreffi',
    '--rust-package',
    'synap-coreffi',
    '--node-modules-dir',
    webNodeModulesDir,
    '--manual-load'
  ],
  repoRoot
);

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
