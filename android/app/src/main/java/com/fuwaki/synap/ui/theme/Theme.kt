package com.fuwaki.synap.ui.theme

import android.app.Activity
import android.os.Build
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.SideEffect
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.core.view.WindowCompat

private val LightColorScheme = lightColorScheme(
    primary = DawnPrimary,
    onPrimary = DawnOnPrimary,
    primaryContainer = DawnPrimaryContainer,
    onPrimaryContainer = DawnOnPrimaryContainer,
    secondary = MossSecondary,
    onSecondary = MossOnSecondary,
    secondaryContainer = MossSecondaryContainer,
    onSecondaryContainer = MossOnSecondaryContainer,
    tertiary = SkyTertiary,
    onTertiary = SkyOnTertiary,
    tertiaryContainer = SkyTertiaryContainer,
    onTertiaryContainer = SkyOnTertiaryContainer,
    background = SandBackground,
    surface = SandSurface,
    onBackground = InkOnLight,
    onSurface = InkOnLight,
    outline = SoftOutline,
)

private val DarkColorScheme = darkColorScheme(
    primary = EmberPrimary,
    onPrimary = DawnOnPrimaryContainer,
    primaryContainer = EmberPrimaryContainer,
    onPrimaryContainer = DawnPrimaryContainer,
    secondary = NightSecondary,
    onSecondary = MossOnSecondaryContainer,
    secondaryContainer = NightSecondaryContainer,
    onSecondaryContainer = MossSecondaryContainer,
    tertiary = NightTertiary,
    onTertiary = SkyOnTertiaryContainer,
    tertiaryContainer = NightTertiaryContainer,
    onTertiaryContainer = SkyTertiaryContainer,
    background = NightBackground,
    surface = NightSurface,
    onBackground = FogOnDark,
    onSurface = FogOnDark,
    outline = Color(0xFF9F8D83),
)

@Composable
fun MyApplicationTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    dynamicColor: Boolean = true,
    content: @Composable () -> Unit,
) {
    val colorScheme = when {
        dynamicColor && Build.VERSION.SDK_INT >= Build.VERSION_CODES.S -> {
            val context = LocalContext.current
            if (darkTheme) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
        }
        darkTheme -> DarkColorScheme
        else -> LightColorScheme
    }

    val view = LocalView.current
    if (!view.isInEditMode) {
        SideEffect {
            val window = (view.context as Activity).window
            window.statusBarColor = colorScheme.background.toArgb()
            window.navigationBarColor = colorScheme.surface.toArgb()
            WindowCompat.getInsetsController(window, view).isAppearanceLightStatusBars = !darkTheme
            WindowCompat.getInsetsController(window, view).isAppearanceLightNavigationBars = !darkTheme
        }
    }

    MaterialTheme(
        colorScheme = colorScheme,
        typography = Typography,
        content = content,
    )
}
