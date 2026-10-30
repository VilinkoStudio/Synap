package com.synap.app.ui.util

import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

@Composable
fun Modifier.navigationBarsScrollPadding(extra: Dp = 16.dp): Modifier {
    val inset = WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding()
    return padding(bottom = inset + extra)
}
