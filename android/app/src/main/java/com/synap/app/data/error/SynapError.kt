package com.synap.app.data.error

import com.fuwaki.synap.bindings.uniffi.synap_coreffi.FfiException

/**
 * Sealed hierarchy of domain errors for the Synap app
 * Maps from FFI errors to user-friendly messages
 */
sealed class SynapError(
    override val message: String,
    override val cause: Throwable? = null
) : Exception(message, cause) {

    /**
     * Database-related errors (open, close, corruption)
     */
    data class Database(
        override val message: String = "Database error occurred",
        override val cause: Throwable? = null
    ) : SynapError(
        message = message,
        cause = cause
    )

    /**
     * Resource not found
     */
    data class NotFound(
        override val message: String = "Resource not found",
        override val cause: Throwable? = null
    ) : SynapError(
        message = message,
        cause = cause
    )

    /**
     * Invalid ID or identifier format
     */
    data class InvalidId(
        override val message: String = "Invalid ID",
        override val cause: Throwable? = null
    ) : SynapError(
        message = message,
        cause = cause
    )

    /**
     * I/O error
     */
    data class Io(
        override val message: String = "I/O error occurred",
        override val cause: Throwable? = null
    ) : SynapError(message, cause)

    /**
     * Generic/unknown error
     */
    data class Unknown(
        override val message: String = "An unknown error occurred",
        override val cause: Throwable? = null
    ) : SynapError(message, cause)

    companion object {
        private fun messageOrDefault(value: String?, fallback: String): String =
            value?.takeIf { it.isNotBlank() } ?: fallback

        /**
         * Convert FfiException to SynapError
         */
        fun fromFfiException(ffiException: FfiException): SynapError = when (ffiException) {
            is FfiException.Database -> Database(
                message = messageOrDefault(ffiException.message, "Database error occurred"),
                cause = ffiException,
            )
            is FfiException.NotFound -> NotFound(
                message = messageOrDefault(ffiException.message, "Resource not found"),
                cause = ffiException,
            )
            is FfiException.InvalidId -> InvalidId(
                message = messageOrDefault(ffiException.message, "Invalid ID"),
                cause = ffiException,
            )
            is FfiException.Io -> Io(
                message = messageOrDefault(ffiException.message, "I/O error occurred"),
                cause = ffiException,
            )
            is FfiException.Other -> Unknown(
                message = messageOrDefault(ffiException.message, "An unknown error occurred"),
                cause = ffiException,
            )
        }
    }
}
