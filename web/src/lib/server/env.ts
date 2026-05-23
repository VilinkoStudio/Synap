import path from 'node:path';

export const repoRoot = path.resolve(import.meta.dirname, '../../../../');
export const bindingPackageDir = path.join(repoRoot, 'target', 'generated', 'nodejs', 'synap-coreffi');
export const defaultDbPath = process.env.SYNAP_WEB_DB ?? path.join(repoRoot, 'target', 'synap-web.redb');
