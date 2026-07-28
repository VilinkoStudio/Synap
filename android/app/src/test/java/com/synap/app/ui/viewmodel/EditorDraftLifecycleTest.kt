package com.synap.app.ui.viewmodel

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class EditorDraftLifecycleTest {
    @Test
    fun changedNonBlankContentIsPersistedOnClose() {
        assertTrue(shouldPersistDraftOnClose("latest content", hasUserChanges = true))
    }

    @Test
    fun untouchedOrBlankDraftIsDiscardedOnClose() {
        assertFalse(shouldPersistDraftOnClose("existing note", hasUserChanges = false))
        assertFalse(shouldPersistDraftOnClose("   ", hasUserChanges = true))
    }
}
