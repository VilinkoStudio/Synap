package com.synap.app.data.error

import com.fuwaki.synap.bindings.uniffi.synap_coreffi.FfiException
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class SynapErrorTest {
    @Test
    fun mapsNotFoundFromFfiException() {
        val error = SynapError.fromFfiException(FfiException.NotFound("note missing"))

        assertTrue(error is SynapError.NotFound)
        assertEquals("note missing", error.message)
    }

    @Test
    fun mapsInvalidIdFromFfiException() {
        val error = SynapError.fromFfiException(FfiException.InvalidId("bad id"))

        assertTrue(error is SynapError.InvalidId)
        assertEquals("bad id", error.message)
    }

    @Test
    fun mapsDatabaseFromFfiException() {
        val error = SynapError.fromFfiException(FfiException.Database("db offline"))

        assertTrue(error is SynapError.Database)
        assertEquals("db offline", error.message)
    }

    @Test
    fun mapsOtherFromFfiException() {
        val error = SynapError.fromFfiException(FfiException.Other("unexpected"))

        assertTrue(error is SynapError.Unknown)
        assertEquals("unexpected", error.message)
    }
}
