import { command, query } from '$app/server';
import {
  createNote,
  deleteNote,
  editNote,
  approvePeer,
  getSyncOverview,
  listNotes,
  pairEndpoint,
  recommendTags,
  removePeer,
  setPeerStatus,
  updatePeerNote
} from '$lib/server/service';

type NotesInput = {
  query?: string;
  cursor?: string;
};

type NoteInput = {
  noteId?: string;
  content?: string;
  tags?: string[];
};

type PeerNoteInput = {
  peerId?: string;
  note?: string;
};

type PeerStatusInput = {
  peerId?: string;
  status?: string;
};

type PairEndpointInput = {
  host?: string;
  port?: number | string;
};

function requireText(value: unknown, name: string) {
  const normalized = String(value ?? '').trim();
  if (!normalized) {
    throw new Error(`${name} is required`);
  }
  return normalized;
}

function normalizeTags(value: unknown) {
  return Array.isArray(value)
    ? value.map((tag) => String(tag).trim()).filter(Boolean)
    : [];
}

export const getHome = query('unchecked', async (input: NotesInput = {}) => {
  const search = input.query?.trim() ?? '';
  const cursor = input.cursor?.trim() || undefined;
  const [notesPage, syncOverview] = await Promise.all([
    listNotes(search, cursor),
    getSyncOverview()
  ]);

  return {
    query: search,
    notesPage,
    syncOverview
  };
});

export const getTagRecommendations = query('unchecked', async (input: { content?: string } = {}) => {
  const content = input.content?.trim() ?? '';
  return content ? await recommendTags(content) : [];
});

export const createNoteRemote = command('unchecked', async (input: NoteInput = {}) => {
  return await createNote(requireText(input.content, 'content'), normalizeTags(input.tags));
});

export const editNoteRemote = command('unchecked', async (input: NoteInput = {}) => {
  return await editNote(
    requireText(input.noteId, 'noteId'),
    requireText(input.content, 'content'),
    normalizeTags(input.tags)
  );
});

export const deleteNoteRemote = command('unchecked', async (input: { noteId?: string } = {}) => {
  const noteId = requireText(input.noteId, 'noteId');
  await deleteNote(noteId);
  return { noteId };
});

export const updatePeerNoteRemote = command('unchecked', async (input: PeerNoteInput = {}) => {
  return await updatePeerNote(requireText(input.peerId, 'peerId'), input.note?.trim() || undefined);
});

export const approvePeerRemote = command('unchecked', async (input: PeerNoteInput = {}) => {
  return await approvePeer(requireText(input.peerId, 'peerId'), input.note?.trim() || undefined);
});

export const setPeerStatusRemote = command('unchecked', async (input: PeerStatusInput = {}) => {
  return await setPeerStatus(requireText(input.peerId, 'peerId'), requireText(input.status, 'status'));
});

export const deletePeerRemote = command('unchecked', async (input: { peerId?: string } = {}) => {
  const peerId = requireText(input.peerId, 'peerId');
  await removePeer(peerId);
  return { peerId };
});

export const pairEndpointRemote = command('unchecked', async (input: PairEndpointInput = {}) => {
  const host = requireText(input.host, 'host');
  const port = Number(input.port);
  if (!Number.isInteger(port) || port < 1 || port > 65_535) {
    throw new Error('port must be between 1 and 65535');
  }
  return await pairEndpoint(host, port);
});
