import path from 'node:path';
import fs from 'node:fs';

export const repoRoot = process.env.SYNAP_REPO_ROOT ?? findRepoRoot();
export const bindingPackageDir = path.join(repoRoot, 'target', 'generated', 'nodejs', 'synap-coreffi');
export const defaultDbPath = process.env.SYNAP_WEB_DB ?? path.join(repoRoot, 'target', 'synap-web.redb');

function findRepoRoot() {
  const candidates = [
    process.cwd(),
    path.resolve(process.cwd(), '..'),
    path.resolve(import.meta.dirname, '../../../../')
  ];

  for (const candidate of candidates) {
    if (
      fs.existsSync(path.join(candidate, 'Cargo.toml')) &&
      fs.existsSync(path.join(candidate, 'coreffi'))
    ) {
      return candidate;
    }
  }

  return path.resolve(process.cwd(), '..');
}
