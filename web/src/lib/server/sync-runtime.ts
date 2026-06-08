import fs from 'node:fs';
import net from 'node:net';
import os from 'node:os';
import Bonjour from 'bonjour-service';
import { getCoreffiModule, pickCallable } from './coreffi';

const DEFAULT_SYNC_PORT = 45_172;
const SOCKET_READ_TIMEOUT_MS = 30_000;
const SOCKET_RETRY_SLEEP_MS = 2;
const SERVICE_TYPE = 'synap';

type ListenerState = {
  protocol: string;
  backend: string;
  isListening: boolean;
  listenPort?: number;
  localAddresses: string[];
  status: string;
  errorMessage?: string;
};

type DiscoveryState = {
  serviceType: string;
  advertisedName?: string;
  listenPort?: number;
  isRunning: boolean;
  errorMessage?: string;
};

type DiscoveredPeer = {
  serviceName: string;
  displayName: string;
  host: string;
  port: number;
  lastSeenAtMs: number;
};

type SyncOverviewRuntime = {
  listener: ListenerState;
  discovery: DiscoveryState;
  discoveredPeers: DiscoveredPeer[];
};

type SyncTransport = {
  read: (maxBytes: number) => Uint8Array;
  write: (payload: Uint8Array) => void;
  close: () => void;
};

type FfiErrorIoConstructor = new (message?: string) => Error;

let server: net.Server | undefined;
let listenerState: ListenerState = stoppedListenerState();
let bonjour: Bonjour | undefined;
let publishedService: ReturnType<Bonjour['publish']> | undefined;
let browser: ReturnType<Bonjour['find']> | undefined;
let discoveryState: DiscoveryState = stoppedDiscoveryState();
let discoveredPeers = new Map<string, DiscoveredPeer>();
let acceptHandler: ((socket: net.Socket) => void) | undefined;

export async function ensureSyncListenerStarted(
  service: any,
  port = DEFAULT_SYNC_PORT
): Promise<SyncOverviewRuntime> {
  if (server?.listening) {
    await ensureDiscoveryStarted(listenerState.listenPort ?? port);
    return getSyncRuntimeOverview();
  }

  try {
    await startServer(service, port);
  } catch (error) {
    if (port !== 0) {
      await startServer(service, 0);
    } else {
      throw error;
    }
  }

  if (listenerState.listenPort) {
    await ensureDiscoveryStarted(listenerState.listenPort);
  }
  return getSyncRuntimeOverview();
}

export function getSyncRuntimeOverview(): SyncOverviewRuntime {
  return {
    listener: { ...listenerState, localAddresses: [...listenerState.localAddresses] },
    discovery: { ...discoveryState },
    discoveredPeers: [...discoveredPeers.values()].sort((left, right) =>
      left.displayName.localeCompare(right.displayName)
    )
  };
}

export async function connectAndSync(service: any, host: string, port: number) {
  console.info(`[synap-web] outbound sync connecting ${host}:${port}`);
  const socket = await connectSocket(host, port);
  const module = await getCoreffiModule();
  const transport = socketTransport(socket, module.FfiErrorIo as FfiErrorIoConstructor | undefined);
  try {
    const session = callSyncMethod(service, ['initiateSync', 'initiate_sync'], transport);
    console.info(`[synap-web] outbound sync completed ${host}:${port}`, summarizeSession(session));
    return session;
  } catch (error) {
    console.error(`[synap-web] outbound sync failed ${host}:${port}`, error);
    throw error;
  } finally {
    transport.close();
  }
}

function startServer(service: any, port: number) {
  return new Promise<void>((resolve, reject) => {
    const nextServer = net.createServer((socket) => {
      acceptHandler?.(socket);
    });
    let settled = false;

    acceptHandler = (socket) => {
      void handleIncomingSocket(service, socket);
    };

    nextServer.once('error', (error) => {
      listenerState = failedListenerState(error);
      if (!settled) {
        settled = true;
        reject(error);
      }
    });
    nextServer.once('listening', () => {
      server = nextServer;
      listenerState = {
        protocol: 'TCP',
        backend: 'Node net + fd sync transport',
        isListening: true,
        listenPort: addressPort(nextServer.address()),
        localAddresses: currentLocalAddresses(),
        status: '已监听',
        errorMessage: undefined
      };
      settled = true;
      resolve();
    });
    nextServer.listen({ host: '0.0.0.0', port });
  });
}

async function handleIncomingSocket(service: any, socket: net.Socket) {
  const remote = `${socket.remoteAddress ?? 'unknown'}:${socket.remotePort ?? 0}`;
  console.info(`[synap-web] inbound sync accepted ${remote}`);
  const module = await getCoreffiModule();
  const transport = socketTransport(socket, module.FfiErrorIo as FfiErrorIoConstructor | undefined);
  try {
    const session = callSyncMethod(service, ['listenSync', 'listen_sync'], transport);
    console.info(`[synap-web] inbound sync completed ${remote}`, summarizeSession(session));
  } catch (error) {
    console.error(`[synap-web] inbound sync failed ${remote}`, error);
  } finally {
    transport.close();
  }
}

function connectSocket(host: string, port: number) {
  return new Promise<net.Socket>((resolve, reject) => {
    const socket = net.createConnection({ host, port });
    socket.setTimeout(SOCKET_READ_TIMEOUT_MS);
    const onError = (error: Error) => reject(error);
    const cleanup = () => {
      socket.off('error', onError);
      socket.off('timeout', onTimeout);
    };
    const onTimeout = () => {
      socket.destroy();
      reject(new Error(`sync connection timed out: ${host}:${port}`));
    };
    socket.once('connect', () => {
      cleanup();
      socket.on('error', (error) => {
        console.error('[synap-web] sync socket error', error);
      });
      resolve(socket);
    });
    socket.once('timeout', onTimeout);
    socket.once('error', onError);
  });
}

function socketTransport(socket: net.Socket, FfiErrorIo?: FfiErrorIoConstructor): SyncTransport {
  socket.pause();
  socket.setNoDelay(true);
  socket.setTimeout(0);

  return {
    read(maxBytes) {
      try {
        const fd = socketFd(socket);
        const buffer = Buffer.alloc(Math.max(1, maxBytes));
        const bytesRead = readSyncRetry(fd, buffer, SOCKET_READ_TIMEOUT_MS);
        return buffer.subarray(0, bytesRead);
      } catch (error) {
        throw toFfiIoError(error, FfiErrorIo);
      }
    },
    write(payload) {
      try {
        const fd = socketFd(socket);
        writeSyncRetry(fd, Buffer.from(payload), SOCKET_READ_TIMEOUT_MS);
      } catch (error) {
        throw toFfiIoError(error, FfiErrorIo);
      }
    },
    close() {
      socket.destroy();
    }
  };
}

function readSyncRetry(fd: number, buffer: Buffer, timeoutMs: number) {
  const deadline = Date.now() + timeoutMs;
  for (;;) {
    try {
      return fs.readSync(fd, buffer, 0, buffer.length, null);
    } catch (error: any) {
      if (!isWouldBlock(error) || Date.now() >= deadline) {
        throw error;
      }
      sleepSync(SOCKET_RETRY_SLEEP_MS);
    }
  }
}

function writeSyncRetry(fd: number, buffer: Buffer, timeoutMs: number) {
  const deadline = Date.now() + timeoutMs;
  let offset = 0;
  while (offset < buffer.length) {
    try {
      offset += fs.writeSync(fd, buffer, offset, buffer.length - offset);
    } catch (error: any) {
      if (!isWouldBlock(error) || Date.now() >= deadline) {
        throw error;
      }
      sleepSync(SOCKET_RETRY_SLEEP_MS);
    }
  }
}

function isWouldBlock(error: NodeJS.ErrnoException) {
  return error.code === 'EAGAIN' || error.code === 'EWOULDBLOCK' || error.code === 'EINTR';
}

const sleepBuffer = new SharedArrayBuffer(4);
const sleepView = new Int32Array(sleepBuffer);

function sleepSync(ms: number) {
  Atomics.wait(sleepView, 0, 0, ms);
}

function toFfiIoError(error: unknown, FfiErrorIo?: FfiErrorIoConstructor) {
  const message = error instanceof Error ? error.message : String(error);
  console.error('[synap-web] sync transport io error', error);
  return FfiErrorIo ? new FfiErrorIo(message) : new Error(message);
}

function socketFd(socket: net.Socket): number {
  const fd = (socket as unknown as { _handle?: { fd?: number } })._handle?.fd;
  if (typeof fd !== 'number' || fd < 0) {
    throw new Error('Node socket file descriptor is unavailable');
  }
  return fd;
}

function callSyncMethod<T>(service: any, keys: string[], ...args: unknown[]): T {
  const method = pickCallable<any, (...inner: unknown[]) => T>(service, keys);
  return method.call(service, ...args);
}

function summarizeSession(session: any) {
  return {
    status: session?.status,
    peer: session?.peer?.kaomoji_fingerprint ?? session?.peer?.id,
    stats: session?.stats
  };
}

async function ensureDiscoveryStarted(port: number) {
  if (discoveryState.isRunning && discoveryState.listenPort === port) {
    return;
  }

  try {
    if (!bonjour) {
      bonjour = new Bonjour();
    }
    publishedService?.stop?.();
    browser?.stop?.();

    const name = localServiceName();
    publishedService = bonjour.publish({
      name,
      type: SERVICE_TYPE,
      protocol: 'tcp',
      port,
      txt: {
        device_name: name,
        protocol: 'tcp',
        version: '1'
      }
    });
    browser = bonjour.find({ type: SERVICE_TYPE, protocol: 'tcp' });
    browser.on('up', (service) => {
      if (service.name === name) return;
      const host = firstRoutableAddress(service.addresses ?? []);
      if (!host || !service.port) return;
      discoveredPeers.set(service.fqdn || service.name, {
        serviceName: service.fqdn || service.name,
        displayName: String(service.txt?.device_name ?? service.name),
        host,
        port: service.port,
        lastSeenAtMs: Date.now()
      });
    });
    browser.on('down', (service) => {
      discoveredPeers.delete(service.fqdn || service.name);
    });

    discoveryState = {
      serviceType: '_synap._tcp.local.',
      advertisedName: name,
      listenPort: port,
      isRunning: true,
      errorMessage: undefined
    };
  } catch (error) {
    discoveryState = {
      ...stoppedDiscoveryState(),
      listenPort: port,
      errorMessage: error instanceof Error ? error.message : String(error)
    };
  }
}

function addressPort(address: string | net.AddressInfo | null) {
  return typeof address === 'object' && address ? address.port : undefined;
}

function currentLocalAddresses() {
  return Object.values(os.networkInterfaces())
    .flatMap((entries) => entries ?? [])
    .filter((entry) => entry.family === 'IPv4' && !entry.internal)
    .map((entry) => entry.address)
    .filter(Boolean);
}

function firstRoutableAddress(addresses: string[]) {
  return addresses.find((address) => net.isIPv4(address) && !address.startsWith('127.')) ?? addresses[0];
}

function localServiceName() {
  return `Synap Web · ${os.hostname() || 'Node'}`;
}

function stoppedListenerState(): ListenerState {
  return {
    protocol: 'TCP',
    backend: 'Node net + fd sync transport',
    isListening: false,
    localAddresses: [],
    status: '未启动',
    errorMessage: undefined
  };
}

function failedListenerState(error: unknown): ListenerState {
  return {
    ...stoppedListenerState(),
    status: '监听失败',
    errorMessage: error instanceof Error ? error.message : String(error)
  };
}

function stoppedDiscoveryState(): DiscoveryState {
  return {
    serviceType: '_synap._tcp.local.',
    isRunning: false,
    errorMessage: undefined
  };
}
