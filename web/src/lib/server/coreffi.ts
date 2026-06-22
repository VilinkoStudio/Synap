import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { bindingPackageDir } from './env';

type CoreffiModule = {
  load?: (libPath?: string) => void;
  unload?: () => void;
  open?: (dbPath: string) => SynapService;
  openMemory?: () => SynapService;
  open_memory?: () => SynapService;
  FfiErrorIo?: new (message?: string) => Error;
  verifyMdnsDiscovery?: (signingPublicKey: Uint8Array, signature: Uint8Array) => boolean;
  verify_mdns_discovery?: (signingPublicKey: Uint8Array, signature: Uint8Array) => boolean;
};

export type SynapService = {
  getRecentNotesPage: (cursor?: string, direction?: 'Older' | 'Newer', limit?: number) => unknown;
  search: (query: string, limit: number) => unknown;
  createNote: (content: string, tags: string[]) => unknown;
  editNote: (targetId: string, content: string, tags: string[]) => unknown;
  deleteNote: (targetId: string) => void;
  recommendTag: (content: string, limit: number) => unknown;
  getLocalIdentity: () => unknown;
  getPeers: () => unknown;
  getRecentSyncSessions: (limit?: number) => unknown;
  updatePeerNote: (peerId: string, note?: string) => unknown;
  setPeerStatus: (peerId: string, status: string) => unknown;
  deletePeer: (peerId: string) => void;
  signMdnsDiscovery?: () => { signingPublicKey: Uint8Array; signature: Uint8Array };
  sign_mdns_discovery?: () => { signingPublicKey: Uint8Array; signature: Uint8Array };
};

let cachedModule: Promise<CoreffiModule> | undefined;
let didLoadLibrary = false;

function candidateEntryFiles() {
  return [
    path.join(bindingPackageDir, 'index.js'),
    path.join(bindingPackageDir, 'dist', 'index.js')
  ];
}

async function importBindingModule(): Promise<CoreffiModule> {
  for (const file of candidateEntryFiles()) {
    if (fs.existsSync(file)) {
      return import(/* @vite-ignore */ pathToFileURL(file).href) as Promise<CoreffiModule>;
    }
  }

  throw new Error(`coreffi Node bindings not found in ${bindingPackageDir}. Run pnpm prepare:bindings first.`);
}

export async function getCoreffiModule() {
  cachedModule ??= importBindingModule();
  const module = await cachedModule;

  if (!didLoadLibrary && module.load) {
    module.load(coreffiLibraryPath());
    didLoadLibrary = true;
  }

  return module;
}

export function pickCallable<T extends object, TValue>(
  target: T,
  keys: string[]
): TValue {
  for (const key of keys) {
    if (key in target) {
      return (target as Record<string, TValue>)[key];
    }
  }

  throw new Error(`Missing binding member. Tried: ${keys.join(', ')}`);
}

export function coreffiLibraryPath() {
  return path.join(bindingPackageDir, resolveLibraryFilename());
}

function resolveLibraryFilename() {
  if (process.platform === 'darwin') {
    return 'libuniffi_synap_coreffi.dylib';
  }
  if (process.platform === 'win32') {
    return 'uniffi_synap_coreffi.dll';
  }
  return 'libuniffi_synap_coreffi.so';
}
