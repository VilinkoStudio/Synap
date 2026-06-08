import { defaultDbPath } from './env';
import { bindingPackageDir } from './env';
import { coreffiLibraryPath, getCoreffiModule, pickCallable } from './coreffi';
import {
  connectAndSync,
  ensureSyncListenerStarted,
  getSyncRuntimeOverview
} from './sync-runtime';
import type {
  LocalIdentity,
  Note,
  Peer,
  SyncOverview,
  SyncSessionRecord,
  TimelineNotesPage
} from '$lib/types';

let cachedService: Promise<any> | undefined;

async function getServiceRaw() {
  cachedService ??= openLiveService();
  try {
    return await cachedService;
  } catch (error) {
    cachedService = undefined;
    throw error;
  }
}

async function openLiveService() {
  const module = await getCoreffiModule();
  const open = pickCallable<typeof module, (dbPath: string) => any>(module, ['open']);

  try {
    return open(defaultDbPath);
  } catch (error) {
    await restoreBackupAfterOpenFailure(error);
    return open(defaultDbPath);
  }
}

function callMethod<T>(service: any, keys: string[], ...args: unknown[]): T {
  const method = pickCallable<any, (...inner: unknown[]) => T>(service, keys);
  return method.call(service, ...args);
}

function normalizeNote(raw: any): Note {
  return {
    id: raw.id,
    content: raw.content,
    tags: raw.tags ?? [],
    created_at: BigInt(raw.created_at),
    deleted: Boolean(raw.deleted),
    reply_to: raw.reply_to,
    edited_from: raw.edited_from
  };
}

function normalizeNotesPage(raw: any): TimelineNotesPage {
  return {
    notes: (raw.notes ?? []).map(normalizeNote).filter((note: Note) => !note.deleted),
    next_cursor: raw.next_cursor ?? undefined
  };
}

function normalizePeer(raw: any): Peer {
  return {
    ...raw,
    public_key: raw.public_key instanceof Uint8Array ? raw.public_key : new Uint8Array(raw.public_key ?? []),
    fingerprint: raw.fingerprint instanceof Uint8Array ? raw.fingerprint : new Uint8Array(raw.fingerprint ?? []),
    avatar_png: raw.avatar_png instanceof Uint8Array ? raw.avatar_png : new Uint8Array(raw.avatar_png ?? [])
  };
}

function normalizeSyncSessionRecord(raw: any): SyncSessionRecord {
  return {
    ...raw,
    peer_public_key: raw.peer_public_key instanceof Uint8Array ? raw.peer_public_key : new Uint8Array(raw.peer_public_key ?? []),
    peer_fingerprint: raw.peer_fingerprint instanceof Uint8Array ? raw.peer_fingerprint : new Uint8Array(raw.peer_fingerprint ?? []),
    started_at_ms: BigInt(raw.started_at_ms),
    finished_at_ms: BigInt(raw.finished_at_ms),
    records_sent: BigInt(raw.records_sent),
    records_received: BigInt(raw.records_received),
    records_applied: BigInt(raw.records_applied),
    records_skipped: BigInt(raw.records_skipped),
    bytes_sent: BigInt(raw.bytes_sent),
    bytes_received: BigInt(raw.bytes_received),
    duration_ms: BigInt(raw.duration_ms)
  };
}

export async function listNotes(query: string, cursor?: string): Promise<TimelineNotesPage> {
  const service = await getServiceRaw();
  const trimmed = query.trim();
  if (!trimmed) {
    return normalizeNotesPage(
      callMethod(service, ['getRecentNotesPage', 'get_recent_notes_page'], cursor, 'Older', 50)
    );
  }

  return {
    notes: (callMethod<any[]>(service, ['search'], trimmed, 50) ?? []).map(normalizeNote).filter((note: Note) => !note.deleted),
    next_cursor: undefined
  };
}

export async function createNote(content: string, tags: string[]) {
  const service = await getServiceRaw();
  return normalizeNote(callMethod(service, ['createNote', 'create_note'], content, tags));
}

export async function editNote(noteId: string, content: string, tags: string[]) {
  const service = await getServiceRaw();
  return normalizeNote(callMethod(service, ['editNote', 'edit_note'], noteId, content, tags));
}

export async function deleteNote(noteId: string) {
  const service = await getServiceRaw();
  callMethod(service, ['deleteNote', 'delete_note'], noteId);
}

export async function recommendTags(content: string): Promise<string[]> {
  const service = await getServiceRaw();
  return callMethod(service, ['recommendTag', 'recommend_tag'], content, 6) ?? [];
}

export async function getSyncOverview(): Promise<{
  localIdentity: LocalIdentity;
  peers: Peer[];
  recentSyncSessions: SyncSessionRecord[];
} & Pick<SyncOverview, 'listener' | 'discovery' | 'discoveredPeers'>> {
  const service = await getServiceRaw();
  await ensureSyncListenerStarted(service);
  const runtime = getSyncRuntimeOverview();
  return {
    localIdentity: callMethod(service, ['getLocalIdentity', 'get_local_identity']),
    peers: (callMethod<any[]>(service, ['getPeers', 'get_peers']) ?? []).map(normalizePeer),
    recentSyncSessions: (
      callMethod<any[]>(service, ['getRecentSyncSessions', 'get_recent_sync_sessions'], 10) ?? []
    ).map(normalizeSyncSessionRecord),
    listener: runtime.listener,
    discovery: runtime.discovery,
    discoveredPeers: runtime.discoveredPeers
  };
}

export async function pairEndpoint(host: string, port: number) {
  const service = await getServiceRaw();
  return await connectAndSync(service, host, port);
}

export async function updatePeerNote(peerId: string, note?: string) {
  const service = await getServiceRaw();
  return normalizePeer(callMethod(service, ['updatePeerNote', 'update_peer_note'], peerId, note));
}

export async function setPeerStatus(peerId: string, status: string) {
  const service = await getServiceRaw();
  return normalizePeer(callMethod(service, ['setPeerStatus', 'set_peer_status'], peerId, status));
}

export async function approvePeer(peerId: string, note?: string) {
  const service = await getServiceRaw();
  if (note?.trim()) {
    callMethod(service, ['updatePeerNote', 'update_peer_note'], peerId, note.trim());
  }
  return normalizePeer(callMethod(service, ['setPeerStatus', 'set_peer_status'], peerId, 'Trusted'));
}

export async function removePeer(peerId: string) {
  const service = await getServiceRaw();
  callMethod(service, ['deletePeer', 'delete_peer'], peerId);
}

export async function exportDatabase(): Promise<Buffer> {
  return await import('node:fs/promises').then((fs) => fs.readFile(defaultDbPath));
}

export async function importDatabase(bytes: Uint8Array) {
  const fs = await import('node:fs/promises');
  const path = await import('node:path');
  const tempPath = path.join(path.dirname(defaultDbPath), 'synap-web.import.redb');
  const backupPath = path.join(path.dirname(defaultDbPath), 'synap-web.backup.redb');
  const failedPath = path.join(path.dirname(defaultDbPath), 'synap-web.import.failed.redb');

  // Drop the cached service before touching the live database file.
  cachedService = undefined;
  await fs.writeFile(tempPath, bytes);
  const stat = await fs.stat(tempPath);
  const header = Buffer.from(bytes.subarray(0, 8)).toString('hex');
  console.info(`[synap-web] import candidate path=${tempPath} bytes=${stat.size} header=${header}`);

  if (stat.size < 8 || header !== '726564621a0aa90d') {
    await preserveImportCandidate(fs, tempPath, failedPath);
    throw new Error(`Uploaded file is not a redb database: bytes=${stat.size} header=${header}`);
  }

  try {
    await validateDatabaseWithCoreffi(tempPath);

    try {
      await fs.rm(backupPath, { force: true });
    } catch {}

    try {
      await fs.rename(defaultDbPath, backupPath);
    } catch (error: any) {
      if (error?.code !== 'ENOENT') {
        throw error;
      }
    }

    await fs.rename(tempPath, defaultDbPath);
  } catch (error) {
    await preserveImportCandidate(fs, tempPath, failedPath);
    console.error(`[synap-web] import validation failed; preserved candidate at ${failedPath}`, error);
    throw error;
  } finally {
    await fs.rm(tempPath, { force: true });
  }
}

async function restoreBackupAfterOpenFailure(originalError: unknown) {
  const fs = await import('node:fs/promises');
  const path = await import('node:path');
  const backupPath = path.join(path.dirname(defaultDbPath), 'synap-web.backup.redb');
  const failedPath = path.join(path.dirname(defaultDbPath), 'synap-web.open.failed.redb');

  try {
    await fs.access(backupPath);
  } catch {
    throw originalError;
  }

  console.error(`[synap-web] live database open failed; restoring backup ${backupPath}`, originalError);
  await fs.rm(failedPath, { force: true });
  await fs.rename(defaultDbPath, failedPath).catch(async (error: any) => {
    if (error?.code !== 'ENOENT') {
      throw error;
    }
  });
  await fs.copyFile(backupPath, defaultDbPath);
}

async function validateDatabaseWithCoreffi(dbPath: string) {
  const { spawn } = await import('node:child_process');
  const script = `
    import path from 'node:path';
    import { pathToFileURL } from 'node:url';
    const [bindingPackageDir, libraryPath, dbPath] = process.argv.slice(1);
    const entry = path.join(bindingPackageDir, 'index.js');
    const module = await import(pathToFileURL(entry).href);
    module.load(libraryPath);
    module.open(dbPath);
  `;

  await new Promise<void>((resolve, reject) => {
    const child = spawn(process.execPath, ['--input-type=module', '--eval', script, bindingPackageDir, coreffiLibraryPath(), dbPath], {
      stdio: ['ignore', 'pipe', 'pipe']
    });

    let stderr = '';
    child.stderr.on('data', (chunk) => {
      stderr += chunk.toString();
    });
    child.on('error', reject);
    child.on('close', (code) => {
      if (code === 0) {
        resolve();
      } else {
        reject(new Error(`Uploaded database cannot be opened by coreffi (${code}): ${stderr.trim()}`));
      }
    });
  });
}

async function preserveImportCandidate(
  fs: typeof import('node:fs/promises'),
  tempPath: string,
  failedPath: string
) {
  try {
    await fs.copyFile(tempPath, failedPath);
  } catch (error: any) {
    if (error?.code !== 'ENOENT') {
      throw error;
    }
  }
}
