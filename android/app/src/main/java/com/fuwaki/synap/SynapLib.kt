package com.fuwaki.synap

/**
 * Helper object to load the native library
 */
object SynapLib {
    init {
        System.loadLibrary("uniffi_synap_coreffi")
    }
}
