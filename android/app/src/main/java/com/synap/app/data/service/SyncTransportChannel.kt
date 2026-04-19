package com.synap.app.data.service

import com.synap.app.di.IoDispatcher
import java.io.InputStream
import java.io.OutputStream
import java.net.Socket
import java.util.concurrent.atomic.AtomicBoolean
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.withContext

data class SyncListenConfig(
    val port: Int? = null,
)

data class SyncConnectConfig(
    val host: String,
    val port: Int,
)

interface SyncTransportChannel {
    suspend fun read(maxBytes: Int = DEFAULT_READ_SIZE): ByteArray

    suspend fun write(bytes: ByteArray)

    suspend fun close()

    companion object {
        const val DEFAULT_READ_SIZE: Int = 16 * 1024
    }
}

class StreamSyncTransportChannel(
    private val input: InputStream,
    private val output: OutputStream,
    private val socket: Socket? = null,
    @IoDispatcher private val ioDispatcher: CoroutineDispatcher,
) : SyncTransportChannel {
    private val closed = AtomicBoolean(false)

    override suspend fun read(maxBytes: Int): ByteArray = withContext(ioDispatcher) {
        require(maxBytes > 0) { "maxBytes must be positive" }

        val buffer = ByteArray(maxBytes)
        val read = input.read(buffer)
        if (read <= 0) {
            ByteArray(0)
        } else {
            buffer.copyOf(read)
        }
    }

    override suspend fun write(bytes: ByteArray) {
        if (bytes.isEmpty()) {
            return
        }

        withContext(ioDispatcher) {
            output.write(bytes)
            output.flush()
        }
    }

    override suspend fun close() {
        if (!closed.compareAndSet(false, true)) {
            return
        }

        withContext(ioDispatcher) {
            runCatching { output.close() }
            runCatching { input.close() }
            runCatching { socket?.close() }
        }
    }
}
