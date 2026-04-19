package com.synap.app.ui.components

import android.text.format.Formatter
import android.util.Base64
import android.widget.Toast
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Tab
import androidx.compose.material3.TabRow
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp

private const val MAX_SHARE_QR_URL_LENGTH = 1800
private const val MAX_SHARE_URL_LENGTH = 4096

private enum class ShareExportMode {
    QrCode,
    Url,
}

private sealed interface ShareExportSheetState {
    data object Loading : ShareExportSheetState
    data class Ready(
        val importUrl: String,
        val packageBytes: Int,
        val qrSupported: Boolean,
    ) : ShareExportSheetState

    data class TooLarge(
        val importUrlLength: Int,
        val packageBytes: Int,
    ) : ShareExportSheetState

    data class Error(val message: String) : ShareExportSheetState
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ShareExportSheet(
    noteIds: List<String>,
    onDismiss: () -> Unit,
    exportShare: suspend (List<String>) -> ByteArray,
) {
    val context = LocalContext.current
    val clipboardManager = LocalClipboardManager.current
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)

    var mode by rememberSaveable(noteIds) { mutableStateOf(ShareExportMode.QrCode) }
    var uiState by remember(noteIds) {
        mutableStateOf<ShareExportSheetState>(ShareExportSheetState.Loading)
    }

    LaunchedEffect(noteIds) {
        uiState = ShareExportSheetState.Loading
        uiState = runCatching {
            val bytes = exportShare(noteIds)
            val importUrl = buildShareImportUrl(bytes)
            if (importUrl.length > MAX_SHARE_URL_LENGTH) {
                ShareExportSheetState.TooLarge(
                    importUrlLength = importUrl.length,
                    packageBytes = bytes.size,
                )
            } else {
                ShareExportSheetState.Ready(
                    importUrl = importUrl,
                    packageBytes = bytes.size,
                    qrSupported = importUrl.length <= MAX_SHARE_QR_URL_LENGTH,
                )
            }
        }.getOrElse { throwable ->
            ShareExportSheetState.Error(
                throwable.message ?: "分享失败，请检查您的笔记数据",
            )
        }
    }

    LaunchedEffect(uiState) {
        val ready = uiState as? ShareExportSheetState.Ready ?: return@LaunchedEffect
        if (!ready.qrSupported) {
            mode = ShareExportMode.Url
        }
    }

    ModalBottomSheet(
        onDismissRequest = onDismiss,
        sheetState = sheetState,
        containerColor = MaterialTheme.colorScheme.surface,
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 24.dp, vertical = 16.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text(
                text = "分享笔记",
                style = MaterialTheme.typography.titleLarge,
                fontWeight = FontWeight.Bold,
                color = MaterialTheme.colorScheme.onSurface,
            )
            Spacer(modifier = Modifier.height(12.dp))
            Text(
                text = "请选择您分享笔记的方式",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(modifier = Modifier.height(24.dp))

            when (val state = uiState) {
                ShareExportSheetState.Loading -> {
                    CircularProgressIndicator()
                    Spacer(modifier = Modifier.height(16.dp))
                    Text("正在生成分享，请稍候……")
                }

                is ShareExportSheetState.Error -> {
                    Text(
                        text = state.message,
                        color = MaterialTheme.colorScheme.error,
                    )
                }

                is ShareExportSheetState.TooLarge -> {
                    Text(
                        text = "笔记数据过大，暂不支持二维码或链接分享。如需导出全部笔记，请使用备份功能。",
                        style = MaterialTheme.typography.titleMedium,
                        color = MaterialTheme.colorScheme.error,
                    )
                    Spacer(modifier = Modifier.height(12.dp))
                    Text(
                        text = "当前所选笔记数据约 ${Formatter.formatShortFileSize(context, state.packageBytes.toLong())}，生成后的链接长度为 ${state.importUrlLength} 字符，暂时无法导出，建议您可以分成多次分享。",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }

                is ShareExportSheetState.Ready -> {
                    if (state.qrSupported) {
                        TabRow(
                            selectedTabIndex = mode.ordinal,
                            modifier = Modifier.fillMaxWidth()
                        ) {
                            Tab(
                                selected = mode == ShareExportMode.QrCode,
                                onClick = { mode = ShareExportMode.QrCode },
                                text = { Text("二维码") }
                            )
                            Tab(
                                selected = mode == ShareExportMode.Url,
                                onClick = { mode = ShareExportMode.Url },
                                text = { Text("链接") }
                            )
                        }
                        Spacer(modifier = Modifier.height(16.dp))
                    }

                    // ========== 修改：更新显示文案 ==========
                    Text(
                        text = "所选笔记的数据大小：${Formatter.formatShortFileSize(context, state.packageBytes.toLong())}",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )

                    if (!state.qrSupported) {
                        Spacer(modifier = Modifier.height(8.dp))
                        Text(
                            text = "当前所选笔记数据超过二维码存储容量限制，请使用链接导出",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.error,
                        )
                    }

                    Spacer(modifier = Modifier.height(20.dp))

                    when (mode) {
                        ShareExportMode.QrCode -> {
                            val qrPrimaryColor = MaterialTheme.colorScheme.primary.toArgb()
                            val qrBgColor = MaterialTheme.colorScheme.surface.toArgb()
                            val qrBitmap = remember(state.importUrl, qrPrimaryColor, qrBgColor) {
                                generateShareQrCodeBitmap(
                                    text = state.importUrl,
                                    size = 800,
                                    primaryColor = qrPrimaryColor,
                                    backgroundColor = qrBgColor,
                                )
                            }

                            if (qrBitmap != null) {
                                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                                    Surface(
                                        shape = RoundedCornerShape(16.dp),
                                        color = MaterialTheme.colorScheme.surface,
                                        tonalElevation = 8.dp,
                                        shadowElevation = 4.dp,
                                    ) {
                                        Image(
                                            bitmap = qrBitmap.asImageBitmap(),
                                            contentDescription = "笔记分享二维码",
                                            modifier = Modifier
                                                .size(280.dp)
                                                .padding(16.dp),
                                        )
                                    }

                                    // ========== 新增：二维码下方的提示文字 ==========
                                    Spacer(modifier = Modifier.height(12.dp))
                                    Text(
                                        text = "请在接收分享的设备上打开系统扫一扫app，扫描此二维码即可接收。",
                                        style = MaterialTheme.typography.bodySmall,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                        textAlign = TextAlign.Center,
                                        modifier = Modifier.padding(horizontal = 16.dp)
                                    )
                                }
                            } else {
                                Box(
                                    modifier = Modifier.size(280.dp),
                                    contentAlignment = Alignment.Center,
                                ) {
                                    Text(
                                        text = "二维码生成失败，请使用链接分享",
                                        color = MaterialTheme.colorScheme.error,
                                    )
                                }
                            }
                        }

                        ShareExportMode.Url -> {
                            SelectionContainer {
                                Surface(
                                    shape = RoundedCornerShape(16.dp),
                                    color = MaterialTheme.colorScheme.surfaceVariant,
                                    modifier = Modifier.fillMaxWidth(),
                                ) {
                                    Text(
                                        text = state.importUrl,
                                        modifier = Modifier.padding(16.dp),
                                        style = MaterialTheme.typography.bodyMedium,
                                    )
                                }
                            }

                            Spacer(modifier = Modifier.height(16.dp))

                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.spacedBy(12.dp),
                            ) {
                                Button(
                                    onClick = {
                                        clipboardManager.setText(AnnotatedString(state.importUrl))
                                        Toast.makeText(context, "已复制分享链接", Toast.LENGTH_SHORT)
                                            .show()
                                    },
                                    modifier = Modifier.weight(1f),
                                ) {
                                    Text("复制链接")
                                }
                                TextButton(
                                    onClick = onDismiss,
                                    modifier = Modifier.weight(1f),
                                ) {
                                    Text("关闭")
                                }
                            }
                        }
                    }
                }
            }

            Spacer(modifier = Modifier.height(32.dp))
        }
    }
}

private fun buildShareImportUrl(bytes: ByteArray): String {
    val encoded = Base64.encodeToString(
        bytes,
        Base64.URL_SAFE or Base64.NO_WRAP or Base64.NO_PADDING,
    )
    return "synap://import_share/$encoded"
}

private fun generateShareQrCodeBitmap(
    text: String,
    size: Int = 512,
    primaryColor: Int = android.graphics.Color.BLACK,
    backgroundColor: Int = android.graphics.Color.WHITE,
): android.graphics.Bitmap? {
    if (text.isEmpty()) return null

    return try {
        val hints = mapOf(
            com.google.zxing.EncodeHintType.CHARACTER_SET to "UTF-8",
            com.google.zxing.EncodeHintType.MARGIN to 1,
        )
        val bitMatrix = com.google.zxing.MultiFormatWriter().encode(
            text,
            com.google.zxing.BarcodeFormat.QR_CODE,
            size,
            size,
            hints,
        )
        val width = bitMatrix.width
        val height = bitMatrix.height
        val pixels = IntArray(width * height)
        for (y in 0 until height) {
            val offset = y * width
            for (x in 0 until width) {
                pixels[offset + x] = if (bitMatrix[x, y]) primaryColor else backgroundColor
            }
        }
        val bitmap = android.graphics.Bitmap.createBitmap(
            width,
            height,
            android.graphics.Bitmap.Config.ARGB_8888,
        )
        bitmap.setPixels(pixels, 0, width, 0, 0, width, height)
        bitmap
    } catch (_: Exception) {
        null
    }
}