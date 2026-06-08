export type NoteBrief = {
  id: string;
  content_preview: string;
  created_at: bigint;
};

export type Note = {
  id: string;
  content: string;
  tags: string[];
  created_at: bigint;
  deleted: boolean;
  reply_to?: NoteBrief;
  edited_from?: NoteBrief;
};

export type TimelineNotesPage = {
  notes: Note[];
  next_cursor?: string;
};

export type PeerTrustStatus = 'Pending' | 'Trusted' | 'Retired' | 'Revoked';

export type Peer = {
  id: string;
  algorithm: string;
  public_key: Uint8Array;
  fingerprint: Uint8Array;
  avatar_png: Uint8Array;
  kaomoji_fingerprint: string;
  display_public_key_base64: string;
  note?: string;
  status: PeerTrustStatus;
};

export type SyncSessionRole = 'Initiator' | 'Listener' | 'RelayFetch' | 'RelayPush';
export type SyncStatus = 'Completed' | 'PendingTrust' | 'Failed';
export type SyncTransportKind = 'Direct' | 'RelayFetch' | 'RelayPush';

export type SyncSessionRecord = {
  id: string;
  role: SyncSessionRole;
  status: SyncStatus;
  transport: SyncTransportKind;
  relay_url?: string;
  peer_label?: string;
  peer_public_key: Uint8Array;
  peer_fingerprint: Uint8Array;
  display_peer_fingerprint_base64: string;
  started_at_ms: bigint;
  finished_at_ms: bigint;
  records_sent: bigint;
  records_received: bigint;
  records_applied: bigint;
  records_skipped: bigint;
  bytes_sent: bigint;
  bytes_received: bigint;
  duration_ms: bigint;
  error_message?: string;
};

export type PublicKeyInfo = {
  id: string;
  algorithm: string;
  public_key: Uint8Array;
  fingerprint: Uint8Array;
  avatar_png: Uint8Array;
  display_public_key_base64: string;
  kaomoji_fingerprint: string;
};

export type LocalIdentity = {
  identity: PublicKeyInfo;
  signing: PublicKeyInfo;
};

export type SyncOverview = {
  localIdentity: LocalIdentity;
  peers: Peer[];
  recentSyncSessions: SyncSessionRecord[];
  listener: SyncListenerState;
  discovery: SyncDiscoveryState;
  discoveredPeers: DiscoveredSyncPeer[];
};

export type SyncListenerState = {
  protocol: string;
  backend: string;
  isListening: boolean;
  listenPort?: number;
  localAddresses: string[];
  status: string;
  errorMessage?: string;
};

export type SyncDiscoveryState = {
  serviceType: string;
  advertisedName?: string;
  listenPort?: number;
  isRunning: boolean;
  errorMessage?: string;
};

export type DiscoveredSyncPeer = {
  serviceName: string;
  displayName: string;
  host: string;
  port: number;
  lastSeenAtMs: number;
};
