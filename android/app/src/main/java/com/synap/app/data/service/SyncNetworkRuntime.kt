package com.synap.app.data.service

import android.util.Log
import com.synap.app.data.model.SyncListenerState
import com.synap.app.di.IoDispatcher
import java.net.Inet4Address
import java.net.InetSocketAddress
import java.net.NetworkInterface
import java.net.ServerSocket
import java.net.Socket
import javax.inject.Inject
import javax.inject.Singleton
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext

private const val TAG = "TcpSyncRuntime"
private const val SOCKET_CONNECT_TIMEOUT_MS = 10_000
private const val SOCKET_READ_TIMEOUT_MS = 30_000

interface SyncNetworkRuntime {
    val state: StateFlow<SyncListenerState>
    val incomingChannels: SharedFlow<SyncTransportChannel>

    suspend fun ensureStarted(config: SyncListenConfig = SyncListenConfig()): Result<SyncListenerState>

    suspend fun connect(config: SyncConnectConfig): Result<SyncTransportChannel>
}

@Singleton
class TcpSyncNetworkRuntime @Inject constructor(
    @IoDispatcher private val ioDispatcher: CoroutineDispatcher,
) : SyncNetworkRuntime {
    private val mutex = Mutex()
    private val scope = CoroutineScope(SupervisorJob() + ioDispatcher)
    private var serverSocket: ServerSocket? = null
    private var acceptJob: kotlinx.coroutines.Job? = null
    private val _state = MutableStateFlow(SyncListenerState(protocol = "TCP", backend = "Java Socket"))
    private val _incomingChannels = MutableSharedFlow<SyncTransportChannel>(extraBufferCapacity = 8)

    override val state: StateFlow<SyncListenerState> = _state.asStateFlow()
    override val incomingChannels: SharedFlow<SyncTransportChannel> = _incomingChannels.asSharedFlow()

    override suspend fun ensureStarted(config: SyncListenConfig): Result<SyncListenerState> = mutex.withLock {
        serverSocket?.let { return Result.success(_state.value) }

        withContext(ioDispatcher) {
            runCatching {
                val socket = ServerSocket()
                socket.reuseAddress = false
                socket.bind(InetSocketAddress(config.port ?: 0))
                serverSocket = socket
                startAcceptLoop(socket)
                val localAddresses = currentLocalAddresses()

                SyncListenerState(
                    protocol = "TCP",
                    backend = "Java Socket",
                    isListening = true,
                    listenPort = socket.localPort,
                    localAddresses = localAddresses,
                    status = "已监听",
                    errorMessage = null,
                )
            }.fold(
                onSuccess = { state ->
                    _state.value = state
                    Result.success(state)
                },
                onFailure = { throwable ->
                    Log.e(TAG, "Failed to start TCP listener", throwable)
                    runCatching { acceptJob?.cancel() }
                    acceptJob = null
                    runCatching { serverSocket?.close() }
                    serverSocket = null
                    val state = SyncListenerState(
                        protocol = "TCP",
                        backend = "Java Socket",
                        isListening = false,
                        listenPort = null,
                        localAddresses = emptyList(),
                        status = "监听失败",
                        errorMessage = buildString {
                            append(throwable.javaClass.simpleName)
                            throwable.message?.takeIf(String::isNotBlank)?.let {
                                append(": ")
                                append(it)
                            }
                        },
                    )
                    _state.value = state
                    Result.failure(throwable)
                },
            )
        }
    }

    override suspend fun connect(config: SyncConnectConfig): Result<SyncTransportChannel> =
        withContext(ioDispatcher) {
            runCatching {
                val socket = Socket()
                socket.tcpNoDelay = true
                socket.soTimeout = SOCKET_READ_TIMEOUT_MS
                socket.connect(InetSocketAddress(config.host, config.port), SOCKET_CONNECT_TIMEOUT_MS)
                StreamSyncTransportChannel(
                    input = socket.getInputStream(),
                    output = socket.getOutputStream(),
                    socket = socket,
                    ioDispatcher = ioDispatcher,
                )
            }.onFailure { throwable ->
                Log.e(TAG, "Failed to establish TCP connection", throwable)
            }
        }

    private fun startAcceptLoop(socket: ServerSocket) {
        acceptJob?.cancel()
        acceptJob = scope.launch(Dispatchers.IO) {
            while (isActive && !socket.isClosed) {
                val accepted = runCatching { socket.accept() }
                accepted.fold(
                    onSuccess = { client ->
                        client.tcpNoDelay = true
                        client.soTimeout = SOCKET_READ_TIMEOUT_MS
                        val channel = StreamSyncTransportChannel(
                            input = client.getInputStream(),
                            output = client.getOutputStream(),
                            socket = client,
                            ioDispatcher = ioDispatcher,
                        )
                        if (!_incomingChannels.tryEmit(channel)) {
                            launch { _incomingChannels.emit(channel) }
                        }
                    },
                    onFailure = { throwable ->
                        if (!socket.isClosed) {
                            Log.e(TAG, "TCP accept loop failed", throwable)
                        }
                    },
                )
            }
        }
    }

    private fun currentLocalAddresses(): List<String> {
        val ipv4Addresses = runCatching {
            NetworkInterface.getNetworkInterfaces()
                ?.toList()
                .orEmpty()
                .asSequence()
                .filter { networkInterface ->
                    runCatching {
                        networkInterface.isUp &&
                            !networkInterface.isLoopback &&
                            !networkInterface.isVirtual
                    }.getOrDefault(false)
                }
                .flatMap { networkInterface ->
                    networkInterface.inetAddresses.toList().asSequence()
                }
                .filterIsInstance<Inet4Address>()
                .filter { address ->
                    !address.isLoopbackAddress &&
                        !address.isLinkLocalAddress
                }
                .map { it.hostAddress.orEmpty() }
                .filter(String::isNotBlank)
                .distinct()
                .toList()
        }.getOrDefault(emptyList())

        val siteLocal = ipv4Addresses.filter { it.startsWith("10.") || it.startsWith("192.168.") || it.matches(Regex("^172\\.(1[6-9]|2\\d|3[0-1])\\..*")) }
        return if (siteLocal.isNotEmpty()) siteLocal else ipv4Addresses
    }
}
