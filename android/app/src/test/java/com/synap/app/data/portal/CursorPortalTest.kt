package com.synap.app.data.portal

import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class CursorPortalTest {
    @Test
    fun refreshLoadsFirstPage() = runBlocking {
        val requestedCursors = mutableListOf<String?>()
        val portal = CursorPortal(
            limit = 2u,
            fetchPage = { cursor, _ ->
                requestedCursors += cursor
                when (cursor) {
                    null -> listOf("a", "b")
                    else -> emptyList()
                }
            },
            cursorOf = { it },
        )

        portal.refresh()

        assertEquals(listOf<String?>(null), requestedCursors)
        assertEquals(listOf("a", "b"), portal.state.value.items)
        assertTrue(portal.state.value.hasMore)
        assertFalse(portal.state.value.isStale)
        assertNull(portal.state.value.error)
    }

    @Test
    fun loadNextAppendsAndExhaustsOnShortPage() = runBlocking {
        val requestedCursors = mutableListOf<String?>()
        val portal = CursorPortal(
            limit = 2u,
            fetchPage = { cursor, _ ->
                requestedCursors += cursor
                when (cursor) {
                    null -> listOf("a", "b")
                    "b" -> listOf("c")
                    else -> emptyList()
                }
            },
            cursorOf = { it },
        )

        portal.refresh()
        portal.loadNext()

        assertEquals(listOf<String?>(null, "b"), requestedCursors)
        assertEquals(listOf("a", "b", "c"), portal.state.value.items)
        assertFalse(portal.state.value.hasMore)
        assertFalse(portal.state.value.isLoading)
    }

    @Test
    fun stalePortalReloadsFromBeginningOnNextLoad() = runBlocking {
        val requestedCursors = mutableListOf<String?>()
        val portal = CursorPortal(
            limit = 2u,
            fetchPage = { cursor, _ ->
                requestedCursors += cursor
                when (cursor) {
                    null -> listOf("a", "b")
                    "b" -> listOf("c", "d")
                    else -> emptyList()
                }
            },
            cursorOf = { it },
        )

        portal.refresh()
        portal.loadNext()
        portal.invalidate()
        portal.loadNext()

        assertEquals(listOf<String?>(null, "b", null), requestedCursors)
        assertEquals(listOf("a", "b"), portal.state.value.items)
        assertTrue(portal.state.value.hasMore)
        assertFalse(portal.state.value.isStale)
    }
}
