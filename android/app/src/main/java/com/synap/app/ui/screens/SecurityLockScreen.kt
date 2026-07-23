package com.synap.app.ui.screens

import android.app.Activity
import android.app.KeyguardManager
import android.content.Context
import android.content.Intent
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Lock
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.activity.compose.BackHandler
import com.synap.app.R

private const val STATUS_IDLE = 0
private const val STATUS_VERIFIED = 1
private const val STATUS_FAILED = 2

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SecurityLockScreen(
    onUnlock: () -> Unit,
) {
    val context = LocalContext.current
    var status by remember { mutableIntStateOf(STATUS_IDLE) }
    var verifying by remember { mutableStateOf(false) }

    BackHandler(enabled = true) {}

    val launcher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.StartActivityForResult()
    ) { result ->
        verifying = false
        if (result.resultCode == Activity.RESULT_OK) {
            status = STATUS_VERIFIED
            onUnlock()
        } else {
            status = STATUS_FAILED
        }
    }

    fun authenticate() {
        verifying = true
        status = STATUS_IDLE
        val km = context.getSystemService(Context.KEYGUARD_SERVICE) as KeyguardManager
        val intent = km.createConfirmDeviceCredentialIntent(
            context.getString(R.string.security_lock_title),
            context.getString(R.string.security_lock_verify)
        )
        if (intent != null) {
            launcher.launch(intent)
        } else {
            verifying = false
            status = STATUS_VERIFIED
            onUnlock()
        }
    }

    val statusText = when (status) {
        STATUS_VERIFIED -> stringResource(R.string.security_lock_success)
        STATUS_FAILED -> stringResource(R.string.security_lock_failed)
        else -> ""
    }

    LaunchedEffect(Unit) {
        authenticate()
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.security_lock_title)) },
            )
        },
    ) { innerPadding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding),
            verticalArrangement = Arrangement.Center,
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Icon(
                imageVector = Icons.Filled.Lock,
                contentDescription = null,
                modifier = Modifier.size(72.dp),
                tint = MaterialTheme.colorScheme.primary,
            )

            Spacer(modifier = Modifier.height(32.dp))

            if (statusText.isNotEmpty()) {
                Text(
                    text = statusText,
                    style = MaterialTheme.typography.bodyLarge,
                    color = if (status == STATUS_VERIFIED) {
                        MaterialTheme.colorScheme.primary
                    } else {
                        MaterialTheme.colorScheme.error
                    },
                    textAlign = TextAlign.Center,
                    modifier = Modifier.padding(horizontal = 32.dp),
                )

                Spacer(modifier = Modifier.height(24.dp))
            }

            Button(
                onClick = { authenticate() },
                enabled = !verifying,
            ) {
                Text(
                    text = if (verifying) {
                        stringResource(R.string.security_lock_verifying)
                    } else {
                        stringResource(R.string.security_lock_verify)
                    }
                )
            }
        }
    }
}
