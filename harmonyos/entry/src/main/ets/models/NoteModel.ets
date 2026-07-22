// 数据模型定义 - 对应 core/src/dto.rs

export interface NoteBrief {
  id: string;
  contentPreview: string;
  createdAt: number;
}

export interface TimelineGroup {
  startsGroup: boolean;
  startedAt: number;
  endedAt: number;
  noteCount: number;
}

export interface Note {
  id: string;
  content: string;
  tags: string[];
  createdAt: number;
  deleted: boolean;
  replyTo?: NoteBrief;
  editedFrom?: NoteBrief;
  timelineGroup?: TimelineGroup;
}

export enum NoteTextChangeKind {
  Equal = 'Equal',
  Insert = 'Insert',
  Delete = 'Delete'
}

export interface NoteTextChange {
  kind: NoteTextChangeKind;
  value: string;
}

export interface NoteTagDiff {
  added: string[];
  removed: string[];
}

export interface NoteContentDiffStats {
  insertedChars: number;
  deletedChars: number;
  insertedLines: number;
  deletedLines: number;
}

export interface NoteVersionDiff {
  tags: NoteTagDiff;
  content: NoteTextChange[];
  contentSummary: NoteTextChange[];
  contentStats: NoteContentDiffStats;
}

export interface NoteVersion {
  note: Note;
  diff: NoteVersionDiff;
}

export enum SearchSource {
  Fuzzy = 'Fuzzy',
  Semantic = 'Semantic'
}

export interface SearchResult {
  note: Note;
  score: number;
  sources: SearchSource[];
}

export interface TimelineNotesPage {
  notes: Note[];
  nextCursor?: string;
}

export interface TimelineDensityPoint {
  startedAt: number;
  endedAt: number;
  noteCount: number;
}

export interface TimelineSession {
  startedAt: number;
  endedAt: number;
  noteCount: number;
  notes: Note[];
}

export interface TimelineSessionsPage {
  sessions: TimelineSession[];
  nextCursor?: string;
}

export enum NoteSegmentDirection {
  Forward = 'Forward',
  Backward = 'Backward'
}

export interface NoteSegmentBranchChoice {
  note: Note;
  weight: number;
}

export interface NoteNeighborContext {
  note: Note;
  weight: number;
  parents: NoteSegmentBranchChoice[];
  children: NoteSegmentBranchChoice[];
}

export interface NoteNeighbors {
  note: Note;
  parents: NoteSegmentBranchChoice[];
  children: NoteSegmentBranchChoice[];
  parentContexts: NoteNeighborContext[];
  childContexts: NoteNeighborContext[];
}

export interface NoteSegmentStep {
  note: Note;
  nextChoices: NoteSegmentBranchChoice[];
  prevChoices: NoteSegmentBranchChoice[];
  stopsHere: boolean;
}

export interface NoteSegment {
  anchorId: string;
  direction: NoteSegmentDirection;
  steps: NoteSegmentStep[];
}

export enum FilteredNoteStatus {
  All = 'All',
  Normal = 'Normal',
  Deleted = 'Deleted'
}

export enum TimelineDirection {
  Older = 'Older',
  Newer = 'Newer'
}

export interface NoteFeedFilter {
  selectedTags: string[];
  includeUntagged: boolean;
  tagFilterEnabled: boolean;
  status: FilteredNoteStatus;
}