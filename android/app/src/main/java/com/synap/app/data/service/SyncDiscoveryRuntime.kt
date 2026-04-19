package com.synap.app.data.service

import android.content.Context
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import android.os.Build
import android.util.Log
import com.synap.app.data.model.DiscoveredSyncPeer
import dagger.hilt.android.qualifiers.ApplicationContext
import java.nio.charset.StandardCharsets
import javax.inject.Inject
import javax.inject.Singleton
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

private const val TAG = "SyncDiscoveryRuntime"
private const val SERVICE_TYPE = "_synap._tcp"

interface SyncDiscoveryRuntime {
    val discoveredPeers: StateFlow<List<DiscoveredSyncPeer>>

    suspend fun ensureStarted(listenPort: Int): Result<Unit>
}

@Singleton
class AndroidSyncDiscoveryRuntime @Inject constructor(
    @ApplicationContext private val context: Context,
) : SyncDiscoveryRuntime {
    private val mutex = Mutex()
    private val nsdManager = context.getSystemService(Context.NSD_SERVICE) as NsdManager
    private val _discoveredPeers = MutableStateFlow<List<DiscoveredSyncPeer>>(emptyList())

    override val discoveredPeers: StateFlow<List<DiscoveredSyncPeer>> = _discoveredPeers.asStateFlow()

    private val resolvedPeers = linkedMapOf<String, DiscoveredSyncPeer>()
    private var registrationListener: NsdManager.RegistrationListener? = null
    private var discoveryListener: NsdManager.DiscoveryListener? = null
    private var localServiceName: String? = null
    private var advertisedPort: Int? = null

    override suspend fun ensureStarted(listenPort: Int): Result<Unit> = mutex.withLock {
        runCatching {
            if (advertisedPort != listenPort) {
                unregisterLocked()
                registerLocked(listenPort)
            }
            if (discoveryListener == null) {
                startDiscoveryLocked()
            }
        }
    }

    private fun registerLocked(listenPort: Int) {
        val requestedName = buildLocalServiceName()
        val serviceInfo = NsdServiceInfo().apply {
            serviceName = requestedName
            serviceType = SERVICE_TYPE
            port = listenPort
            setAttribute("device_name", requestedName)
            setAttribute("protocol", "tcp")
            setAttribute("version", "1")
        }

        val listener = object : NsdManager.RegistrationListener {
            override fun onServiceRegistered(info: NsdServiceInfo) {
                localServiceName = info.serviceName
                advertisedPort = listenPort
                Log.i(TAG, "mDNS service registered: ${info.serviceName}:${info.port}")
            }

            override fun onRegistrationFailed(serviceInfo: NsdServiceInfo, errorCode: Int) {
                Log.e(TAG, "mDNS registration failed: $errorCode")
                if (registrationListener === this) {
                    registrationListener = null
                    localServiceName = null
                    advertisedPort = null
                }
            }

            override fun onServiceUnregistered(serviceInfo: NsdServiceInfo) {
                if (registrationListener === this) {
                    registrationListener = null
                    localServiceName = null
                    advertisedPort = null
                }
            }

            override fun onUnregistrationFailed(serviceInfo: NsdServiceInfo, errorCode: Int) {
                Log.w(TAG, "mDNS unregistration failed: $errorCode")
                if (registrationListener === this) {
                    registrationListener = null
                    localServiceName = null
                    advertisedPort = null
                }
            }
        }

        registrationListener = listener
        nsdManager.registerService(serviceInfo, NsdManager.PROTOCOL_DNS_SD, listener)
    }

    private fun startDiscoveryLocked() {
        val listener = object : NsdManager.DiscoveryListener {
            override fun onStartDiscoveryFailed(serviceType: String, errorCode: Int) {
                Log.e(TAG, "mDNS discovery start failed: $errorCode")
                runCatching { nsdManager.stopServiceDiscovery(this) }
                if (discoveryListener === this) {
                    discoveryListener = null
                }
            }

            override fun onStopDiscoveryFailed(serviceType: String, errorCode: Int) {
                Log.w(TAG, "mDNS discovery stop failed: $errorCode")
                runCatching { nsdManager.stopServiceDiscovery(this) }
                if (discoveryListener === this) {
                    discoveryListener = null
                }
            }

            override fun onDiscoveryStarted(serviceType: String) {
                Log.i(TAG, "mDNS discovery started for $serviceType")
            }

            override fun onDiscoveryStopped(serviceType: String) {
                if (discoveryListener === this) {
                    discoveryListener = null
                }
                Log.i(TAG, "mDNS discovery stopped for $serviceType")
            }

            override fun onServiceFound(serviceInfo: NsdServiceInfo) {
                if (serviceInfo.serviceType.normalizeServiceType() != SERVICE_TYPE) {
                    return
                }
                if (serviceInfo.serviceName == localServiceName) {
                    return
                }
                resolve(serviceInfo)
            }

            override fun onServiceLost(serviceInfo: NsdServiceInfo) {
                synchronized(resolvedPeers) {
                    resolvedPeers.remove(serviceInfo.serviceName)
                    publishPeersLocked()
                }
            }
        }

        discoveryListener = listener
        nsdManager.discoverServices(SERVICE_TYPE, NsdManager.PROTOCOL_DNS_SD, listener)
    }

    private fun resolve(serviceInfo: NsdServiceInfo) {
        val listener = object : NsdManager.ResolveListener {
            override fun onResolveFailed(serviceInfo: NsdServiceInfo, errorCode: Int) {
                Log.w(TAG, "mDNS resolve failed for ${serviceInfo.serviceName}: $errorCode")
            }

            override fun onServiceResolved(info: NsdServiceInfo) {
                if (info.serviceName == localServiceName) {
                    return
                }
                val host = info.host?.hostAddress ?: return
                synchronized(resolvedPeers) {
                    resolvedPeers[info.serviceName] = DiscoveredSyncPeer(
                        serviceName = info.serviceName,
                        displayName = info.attributeValue("device_name") ?: info.serviceName,
                        host = host,
                        port = info.port,
                        lastSeenAtMs = System.currentTimeMillis(),
                    )
                    publishPeersLocked()
                }
            }
        }

        runCatching { nsdManager.resolveService(serviceInfo, listener) }
            .onFailure { throwable ->
                Log.w(TAG, "mDNS resolve call failed for ${serviceInfo.serviceName}", throwable)
            }
    }

    private fun unregisterLocked() {
        registrationListener?.let { listener ->
            runCatching { nsdManager.unregisterService(listener) }
        }
        registrationListener = null
        localServiceName = null
        advertisedPort = null
    }

    private fun publishPeersLocked() {
        _discoveredPeers.value = resolvedPeers.values
            .sortedBy { it.displayName.lowercase() }
            .toList()
    }

    private fun buildLocalServiceName(): String {
        val appName = context.applicationInfo.loadLabel(context.packageManager).toString()
        val model = listOfNotNull(
            Build.MANUFACTURER?.trim()?.takeIf(String::isNotBlank),
            Build.MODEL?.trim()?.takeIf(String::isNotBlank),
        ).joinToString(" ")
        return listOf(appName, model)
            .filter(String::isNotBlank)
            .joinToString(" · ")
            .ifBlank { "Synap Device" }
    }

    private fun NsdServiceInfo.attributeValue(key: String): String? {
        val payload = attributes[key] ?: return null
        return runCatching { String(payload, StandardCharsets.UTF_8) }.getOrNull()
            ?.takeIf(String::isNotBlank)
    }

    private fun String.normalizeServiceType(): String = trimEnd('.')
}
