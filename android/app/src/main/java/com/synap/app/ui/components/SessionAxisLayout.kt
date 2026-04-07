package com.synap.app.ui.components

import kotlin.math.max

data class SessionAxisLayout(
    val segmentWeights: List<Float>,
) {
    val sessionCount: Int = segmentWeights.size
    val lastSessionIndex: Int = segmentWeights.lastIndex
    val cumulativeWeights: List<Float> = buildList {
        var running = 0f
        add(running)
        segmentWeights.forEach { weight ->
            running += weight
            add(running)
        }
    }
    val totalWeight: Float = cumulativeWeights.lastOrNull() ?: 0f

    fun boundaryFraction(boundaryIndex: Int): Float {
        if (sessionCount == 0 || totalWeight <= 0f) {
            return 0f
        }

        val safeIndex = boundaryIndex.coerceIn(0, sessionCount)
        return cumulativeWeights[safeIndex] / totalWeight
    }

    fun boundaryWeight(boundaryIndex: Int): Float {
        if (sessionCount == 0 || totalWeight <= 0f) {
            return 0f
        }

        val safeIndex = boundaryIndex.coerceIn(0, sessionCount)
        return cumulativeWeights[safeIndex]
    }

    fun weightForSessionFraction(
        sessionIndex: Int,
        fraction: Float,
    ): Float {
        if (sessionCount == 0 || totalWeight <= 0f) {
            return 0f
        }

        val safeIndex = sessionIndex.coerceIn(0, lastSessionIndex)
        val segmentWeight = segmentWeights[safeIndex]
        return (cumulativeWeights[safeIndex] + segmentWeight * fraction.coerceIn(0f, 1f))
            .coerceIn(0f, totalWeight)
    }

    fun sessionProgressToWeight(progress: Float): Float {
        if (sessionCount == 0 || totalWeight <= 0f) {
            return 0f
        }

        val clamped = progress.coerceIn(0f, lastSessionIndex.toFloat())
        val whole = clamped.toInt().coerceIn(0, lastSessionIndex)
        val fraction = if (whole == lastSessionIndex) 1f else (clamped - whole).coerceIn(0f, 1f)
        return if (whole == lastSessionIndex) {
            totalWeight
        } else {
            weightForSessionFraction(whole, fraction)
        }
    }

    fun weightToSessionProgress(weight: Float): Float {
        if (sessionCount <= 1 || totalWeight <= 0f) {
            return 0f
        }

        val clamped = weight.coerceIn(0f, totalWeight)
        for (index in 0 until sessionCount) {
            val start = cumulativeWeights[index]
            val end = cumulativeWeights[index + 1]
            if (clamped <= end || index == lastSessionIndex) {
                val segmentWeight = segmentWeights[index].takeIf { it > 0f } ?: 1f
                val fraction = ((clamped - start) / segmentWeight).coerceIn(0f, 1f)
                return if (index == lastSessionIndex) {
                    lastSessionIndex.toFloat()
                } else {
                    index + fraction
                }
            }
        }

        return lastSessionIndex.toFloat()
    }

    fun weightToActiveSessionIndex(weight: Float): Int {
        if (sessionCount == 0 || totalWeight <= 0f) {
            return 0
        }

        val clamped = weight.coerceIn(0f, totalWeight)
        for (index in 0 until sessionCount) {
            if (clamped <= cumulativeWeights[index + 1] || index == lastSessionIndex) {
                return index
            }
        }

        return lastSessionIndex
    }

    companion object {
        fun fromMarkers(markers: List<TimeClusterMarker>): SessionAxisLayout =
            SessionAxisLayout(
                segmentWeights = markers.map { marker ->
                    max(marker.noteCount, 1).toFloat()
                },
            )
    }
}
