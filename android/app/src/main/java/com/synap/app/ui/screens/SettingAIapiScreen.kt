package com.synap.app.ui.screens

import androidx.activity.compose.PredictiveBackHandler
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.KeyboardArrowRight
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.synap.app.R
import com.synap.app.data.service.AIModelPresetStore
import com.synap.app.data.service.AIModelRecord
import com.synap.app.data.service.AIModelStore
import com.synap.app.data.service.PresetModelProvider
import kotlinx.coroutines.CancellationException

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingAIapiScreen(
    onNavigateBack: () -> Unit,
) {
    val context = LocalContext.current
    val store = remember { runCatching { AIModelStore(context) }.getOrNull() }
    val presetProviders = remember { runCatching { AIModelPresetStore.load(context) }.getOrDefault(emptyList()) }
    var importedModels by remember { mutableStateOf(store?.list().orEmpty()) }

    // ========== 增加AI提供商弹窗状态 ==========
    var showAddProviderDialog by remember { mutableStateOf(false) }
    var providerId by remember { mutableStateOf("") }
    var serviceName by remember { mutableStateOf("") }
    var baseUrl by remember { mutableStateOf("") }
    var apiKey by remember { mutableStateOf("") }
    var modelId by remember { mutableStateOf("") }
    var modelName by remember { mutableStateOf("") }
    var modelType by remember { mutableStateOf("LLM") }

    // ========== 预设模型弹窗状态 ==========
    var showPresetDialog by remember { mutableStateOf(false) }
    var currentPreset by remember { mutableStateOf<PresetModelProvider?>(null) }
    var presetApiKey by remember { mutableStateOf("") }
    var presetSelectedModelIndex by remember { mutableStateOf(-1) }
    var presetCustomModelId by remember { mutableStateOf("") }
    var presetCustomModelName by remember { mutableStateOf("") }

    // ========== 编辑模型弹窗状态 ==========
    var showEditDialog by remember { mutableStateOf(false) }
    var editingModel by remember { mutableStateOf<AIModelRecord?>(null) }
    var editProviderId by remember { mutableStateOf("") }
    var editServiceName by remember { mutableStateOf("") }
    var editBaseUrl by remember { mutableStateOf("") }
    var editApiKey by remember { mutableStateOf("") }
    var editModelId by remember { mutableStateOf("") }
    var editModelName by remember { mutableStateOf("") }
    var editModelType by remember { mutableStateOf("LLM") }

    var backProgress by remember { mutableFloatStateOf(0f) }

    PredictiveBackHandler { progressFlow ->
        try {
            progressFlow.collect { backEvent ->
                backProgress = backEvent.progress
            }
            onNavigateBack()
        } catch (e: CancellationException) {
            backProgress = 0f
        }
    }

    fun resetAddProviderState() {
        providerId = ""
        serviceName = ""
        baseUrl = ""
        apiKey = ""
        modelId = ""
        modelName = ""
        modelType = "LLM"
    }

    fun resetPresetState() {
        presetApiKey = ""
        presetSelectedModelIndex = -1
        presetCustomModelId = ""
        presetCustomModelName = ""
        currentPreset = null
    }

    fun resetEditState() {
        editingModel = null
        editProviderId = ""
        editServiceName = ""
        editBaseUrl = ""
        editApiKey = ""
        editModelId = ""
        editModelName = ""
        editModelType = "LLM"
    }

    fun openEditDialog(model: AIModelRecord) {
        editingModel = model
        editProviderId = model.providerId
        editServiceName = model.serviceName
        editBaseUrl = model.baseUrl
        editApiKey = model.apiKey
        editModelId = model.modelId
        editModelName = model.modelName
        editModelType = model.modelType
        showEditDialog = true
    }

    // ========== 增加AI提供商弹窗 ==========
    if (showAddProviderDialog) {
        AlertDialog(
            onDismissRequest = { showAddProviderDialog = false },
            title = { Text(stringResource(R.string.ai_add_provider)) },
            text = {
                Column(
                    modifier = Modifier.verticalScroll(rememberScrollState()),
                    verticalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                    OutlinedTextField(
                        value = providerId,
                        onValueChange = { providerId = it },
                        label = { Text(stringResource(R.string.ai_provider_id)) },
                        supportingText = { Text(stringResource(R.string.ai_provider_id_hint)) },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true,
                    )
                    OutlinedTextField(
                        value = serviceName,
                        onValueChange = { serviceName = it },
                        label = { Text(stringResource(R.string.ai_remark_name)) },
                        supportingText = { Text(stringResource(R.string.ai_service_name_hint)) },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true,
                    )
                    OutlinedTextField(
                        value = baseUrl,
                        onValueChange = { baseUrl = it },
                        label = { Text(stringResource(R.string.ai_base_url)) },
                        supportingText = { Text(stringResource(R.string.ai_base_url_hint)) },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true,
                    )
                    OutlinedTextField(
                        value = apiKey,
                        onValueChange = { apiKey = it },
                        label = { Text(stringResource(R.string.ai_api_key)) },
                        supportingText = { Text(stringResource(R.string.ai_api_key_hint)) },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true,
                    )
                    OutlinedTextField(
                        value = modelId,
                        onValueChange = { modelId = it },
                        label = { Text(stringResource(R.string.ai_model_id)) },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true,
                    )
                    OutlinedTextField(
                        value = modelName,
                        onValueChange = { modelName = it },
                        label = { Text(stringResource(R.string.ai_model_remark)) },
                        supportingText = { Text(stringResource(R.string.ai_model_name_hint)) },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true,
                    )
                    Text(
                        text = stringResource(R.string.ai_model_type),
                        style = MaterialTheme.typography.bodyLarge,
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        OutlinedButton(
                            onClick = { modelType = "LLM" },
                            modifier = Modifier.fillMaxWidth(),
                        ) {
                            if (modelType == "LLM") {
                                Icon(Icons.Filled.Check, contentDescription = null, modifier = Modifier.size(18.dp))
                                Spacer(modifier = Modifier.width(8.dp))
                            }
                            Text(stringResource(R.string.ai_model_type_llm))
                        }
                        OutlinedButton(
                            onClick = { modelType = "Embedding" },
                            modifier = Modifier.fillMaxWidth(),
                        ) {
                            if (modelType == "Embedding") {
                                Icon(Icons.Filled.Check, contentDescription = null, modifier = Modifier.size(18.dp))
                                Spacer(modifier = Modifier.width(8.dp))
                            }
                            Text(stringResource(R.string.ai_model_type_embedding))
                        }
                    }
                }
            },
            confirmButton = {
                Button(
                    onClick = {
                        store?.add(
                            AIModelRecord(
                                providerId = providerId,
                                serviceName = serviceName.ifBlank { providerId },
                                baseUrl = baseUrl,
                                apiKey = apiKey,
                                modelId = modelId,
                                modelName = modelName.ifBlank { modelId },
                                modelType = modelType,
                            )
                        )
                        importedModels = store?.list().orEmpty()
                        showAddProviderDialog = false
                        resetAddProviderState()
                    },
                    enabled = providerId.isNotBlank() && baseUrl.isNotBlank() && modelId.isNotBlank(),
                ) {
                    Text(stringResource(R.string.save))
                }
            },
            dismissButton = {
                TextButton(onClick = {
                    showAddProviderDialog = false
                    resetAddProviderState()
                }) {
                    Text(stringResource(R.string.cancel))
                }
            },
        )
    }

    // ========== 预设模型弹窗 ==========
    if (showPresetDialog && currentPreset != null) {
        val preset = currentPreset!!
        val hasModels = preset.models.isNotEmpty()
        val isCustom = presetSelectedModelIndex == preset.models.size
        val canSave = if (hasModels) {
            if (isCustom) presetCustomModelId.isNotBlank() else presetSelectedModelIndex >= 0
        } else {
            presetCustomModelId.isNotBlank()
        }

        AlertDialog(
            onDismissRequest = { showPresetDialog = false },
            title = { Text(preset.name) },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    OutlinedTextField(
                        value = presetApiKey,
                        onValueChange = { presetApiKey = it },
                        label = { Text(stringResource(R.string.ai_api_key)) },
                        supportingText = { Text(stringResource(R.string.ai_api_key_hint)) },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true,
                    )
                    if (hasModels) {
                        Text(
                            text = stringResource(R.string.ai_select_model),
                            style = MaterialTheme.typography.bodyLarge,
                            color = MaterialTheme.colorScheme.onSurface,
                        )
                        val isDark = MaterialTheme.colorScheme.surface.luminance() < 0.5f
                        Column(
                            modifier = Modifier
                                .fillMaxWidth()
                                .clip(RoundedCornerShape(12.dp))
                                .background(if (isDark) Color.Black else Color.White),
                        ) {
                            preset.models.forEachIndexed { index, model ->
                                Row(
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .clickable { presetSelectedModelIndex = index }
                                        .padding(12.dp),
                                    verticalAlignment = Alignment.CenterVertically,
                                ) {
                                    Text(
                                        text = model.id,
                                        style = MaterialTheme.typography.bodyMedium,
                                        color = MaterialTheme.colorScheme.onSurface,
                                        modifier = Modifier.weight(1f),
                                    )
                                    if (presetSelectedModelIndex == index) {
                                        Icon(
                                            Icons.Filled.Check,
                                            contentDescription = null,
                                            tint = MaterialTheme.colorScheme.primary,
                                            modifier = Modifier.size(20.dp),
                                        )
                                    }
                                }
                                if (index < preset.models.lastIndex) {
                                    HorizontalDivider(
                                        color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f),
                                        modifier = Modifier.padding(horizontal = 12.dp),
                                    )
                                }
                            }
                            HorizontalDivider(
                                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f),
                                modifier = Modifier.padding(horizontal = 12.dp),
                            )
                            Row(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .clickable { presetSelectedModelIndex = preset.models.size }
                                    .padding(12.dp),
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                Text(
                                    text = stringResource(R.string.ai_custom_model),
                                    style = MaterialTheme.typography.bodyMedium,
                                    color = MaterialTheme.colorScheme.primary,
                                    modifier = Modifier.weight(1f),
                                )
                                if (isCustom) {
                                    Icon(
                                        Icons.Filled.Check,
                                        contentDescription = null,
                                        tint = MaterialTheme.colorScheme.primary,
                                        modifier = Modifier.size(20.dp),
                                    )
                                }
                            }
                        }
                    }
                    if (!hasModels || isCustom) {
                        OutlinedTextField(
                            value = presetCustomModelId,
                            onValueChange = { presetCustomModelId = it },
                            label = { Text(stringResource(R.string.ai_model_id)) },
                            modifier = Modifier.fillMaxWidth(),
                            singleLine = true,
                        )
                        OutlinedTextField(
                            value = presetCustomModelName,
                            onValueChange = { presetCustomModelName = it },
                            label = { Text(stringResource(R.string.ai_model_remark)) },
                            supportingText = { Text(stringResource(R.string.ai_model_name_hint)) },
                            modifier = Modifier.fillMaxWidth(),
                            singleLine = true,
                        )
                    }
                }
            },
            confirmButton = {
                Button(
                    onClick = {
                        val finalModelId = if (hasModels && !isCustom) {
                            preset.models[presetSelectedModelIndex].id
                        } else {
                            presetCustomModelId
                        }
                        val finalModelName = if (hasModels && !isCustom) {
                            preset.models[presetSelectedModelIndex].name
                        } else {
                            presetCustomModelName.ifBlank { presetCustomModelId }
                        }
                        store?.add(
                            AIModelRecord(
                                providerId = preset.id,
                                serviceName = preset.name,
                                baseUrl = preset.baseURL,
                                apiKey = presetApiKey,
                                modelId = finalModelId,
                                modelName = finalModelName,
                                modelType = "LLM",
                                isPreset = true,
                                presetIconRes = AIModelPresetStore.getIconRes(preset.iconResName),
                            )
                        )
                        importedModels = store?.list().orEmpty()
                        showPresetDialog = false
                        resetPresetState()
                    },
                    enabled = canSave,
                ) {
                    Text(stringResource(R.string.save))
                }
            },
            dismissButton = {
                TextButton(onClick = {
                    showPresetDialog = false
                    resetPresetState()
                }) {
                    Text(stringResource(R.string.cancel))
                }
            },
        )
    }

    // ========== 编辑模型弹窗 ==========
    if (showEditDialog && editingModel != null) {
        val model = editingModel!!
        AlertDialog(
            onDismissRequest = { showEditDialog = false },
            title = { Text(stringResource(R.string.ai_edit_model)) },
            text = {
                Column(
                    modifier = Modifier.verticalScroll(rememberScrollState()),
                    verticalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                    OutlinedTextField(
                        value = editProviderId,
                        onValueChange = { editProviderId = it },
                        label = { Text(stringResource(R.string.ai_provider_id)) },
                        supportingText = { Text(stringResource(R.string.ai_provider_id_hint)) },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true,
                        enabled = !model.isPreset,
                    )
                    OutlinedTextField(
                        value = editServiceName,
                        onValueChange = { editServiceName = it },
                        label = { Text(stringResource(R.string.ai_remark_name)) },
                        supportingText = { Text(stringResource(R.string.ai_service_name_hint)) },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true,
                        enabled = !model.isPreset,
                    )
                    OutlinedTextField(
                        value = editBaseUrl,
                        onValueChange = { editBaseUrl = it },
                        label = { Text(stringResource(R.string.ai_base_url)) },
                        supportingText = { Text(stringResource(R.string.ai_base_url_hint)) },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true,
                        enabled = !model.isPreset,
                    )
                    OutlinedTextField(
                        value = editApiKey,
                        onValueChange = { editApiKey = it },
                        label = { Text(stringResource(R.string.ai_api_key)) },
                        supportingText = { Text(stringResource(R.string.ai_api_key_hint)) },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true,
                    )
                    OutlinedTextField(
                        value = editModelId,
                        onValueChange = { editModelId = it },
                        label = { Text(stringResource(R.string.ai_model_id)) },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true,
                    )
                    OutlinedTextField(
                        value = editModelName,
                        onValueChange = { editModelName = it },
                        label = { Text(stringResource(R.string.ai_model_remark)) },
                        supportingText = { Text(stringResource(R.string.ai_model_name_hint)) },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true,
                    )
                    if (!model.isPreset) {
                        Text(
                            text = stringResource(R.string.ai_model_type),
                            style = MaterialTheme.typography.bodyLarge,
                            color = MaterialTheme.colorScheme.onSurface,
                        )
                        Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                            OutlinedButton(
                                onClick = { editModelType = "LLM" },
                                modifier = Modifier.fillMaxWidth(),
                            ) {
                                if (editModelType == "LLM") {
                                    Icon(Icons.Filled.Check, contentDescription = null, modifier = Modifier.size(18.dp))
                                    Spacer(modifier = Modifier.width(8.dp))
                                }
                                Text(stringResource(R.string.ai_model_type_llm))
                            }
                            OutlinedButton(
                                onClick = { editModelType = "Embedding" },
                                modifier = Modifier.fillMaxWidth(),
                            ) {
                                if (editModelType == "Embedding") {
                                    Icon(Icons.Filled.Check, contentDescription = null, modifier = Modifier.size(18.dp))
                                    Spacer(modifier = Modifier.width(8.dp))
                                }
                                Text(stringResource(R.string.ai_model_type_embedding))
                            }
                        }
                    }
                }
            },
            confirmButton = {
                Button(
                    onClick = {
                        store?.delete(model.id)
                        store?.add(
                            AIModelRecord(
                                id = model.id,
                                providerId = editProviderId,
                                serviceName = editServiceName,
                                baseUrl = editBaseUrl,
                                apiKey = editApiKey,
                                modelId = editModelId,
                                modelName = editModelName,
                                modelType = editModelType,
                                isPreset = model.isPreset,
                                presetIconRes = model.presetIconRes,
                            )
                        )
                        importedModels = store?.list().orEmpty()
                        showEditDialog = false
                        resetEditState()
                    },
                    enabled = editProviderId.isNotBlank() && editBaseUrl.isNotBlank() && editModelId.isNotBlank(),
                ) {
                    Text(stringResource(R.string.save))
                }
            },
            dismissButton = {
                TextButton(onClick = {
                    showEditDialog = false
                    resetEditState()
                }) {
                    Text(stringResource(R.string.cancel))
                }
            },
        )
    }

    // ========== 主页面 ==========
    Scaffold(
        modifier = Modifier
            .fillMaxSize()
            .graphicsLayer {
                val scale = 1f - (0.1f * backProgress)
                scaleX = scale
                scaleY = scale
                translationX = backProgress * 16.dp.toPx() // 向右边缘移动
                transformOrigin = TransformOrigin(1f, 0.5f) // 缩放原点在右侧中心
                shape = RoundedCornerShape(32.dp * backProgress)
                clip = true
            },
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.ai_service_provider)) },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.Filled.ArrowBack, contentDescription = stringResource(R.string.back))
                    }
                },
                actions = {
                    IconButton(onClick = { showAddProviderDialog = true }) {
                        Icon(Icons.Filled.Add, contentDescription = stringResource(R.string.ai_add_provider))
                    }
                },
            )
        },
    ) { innerPadding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 16.dp),
        ) {
            Spacer(modifier = Modifier.height(16.dp))

            // ==================== 已导入模型 ====================
            Text(
                text = stringResource(R.string.ai_imported_models),
                style = MaterialTheme.typography.titleSmall,
                color = MaterialTheme.colorScheme.primary,
                modifier = Modifier.padding(bottom = 12.dp, start = 8.dp),
            )
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .clip(RoundedCornerShape(16.dp))
                    .background(MaterialTheme.colorScheme.surfaceVariant),
            ) {
                if (importedModels.isEmpty()) {
                    Text(
                        text = stringResource(R.string.ai_no_imported_models),
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(16.dp),
                    )
                } else {
                    importedModels.forEachIndexed { index, model ->
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .clickable { openEditDialog(model) }
                                .padding(16.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            if (model.presetIconRes != null && model.presetIconRes != 0) {
                                Image(
                                    painter = painterResource(id = model.presetIconRes),
                                    contentDescription = null,
                                    modifier = Modifier
                                        .size(40.dp)
                                        .clip(RoundedCornerShape(8.dp))
                                )
                            } else {
                                Box(
                                    modifier = Modifier
                                        .size(40.dp)
                                        .clip(RoundedCornerShape(8.dp))
                                        .background(MaterialTheme.colorScheme.primary),
                                    contentAlignment = Alignment.Center,
                                ) {
                                    Text(
                                        text = "AI",
                                        color = MaterialTheme.colorScheme.onPrimary,
                                        fontSize = 14.sp,
                                        fontWeight = FontWeight.Bold,
                                        textAlign = TextAlign.Center,
                                    )
                                }
                            }
                            Spacer(modifier = Modifier.width(16.dp))
                            Column(modifier = Modifier.weight(1f)) {
                                Text(
                                    text = model.serviceName,
                                    style = MaterialTheme.typography.bodyLarge,
                                    color = MaterialTheme.colorScheme.onSurface,
                                )
                                Text(
                                    text = model.modelName,
                                    style = MaterialTheme.typography.bodyMedium,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                        }
                        if (index < importedModels.lastIndex) {
                            HorizontalDivider(
                                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f),
                                modifier = Modifier.padding(horizontal = 16.dp),
                            )
                        }
                    }
                }
            }

            Spacer(modifier = Modifier.height(24.dp))

            // ==================== 预设模型 ====================
            Text(
                text = stringResource(R.string.ai_preset_models),
                style = MaterialTheme.typography.titleSmall,
                color = MaterialTheme.colorScheme.primary,
                modifier = Modifier.padding(bottom = 12.dp, start = 8.dp),
            )
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .clip(RoundedCornerShape(16.dp))
                    .background(MaterialTheme.colorScheme.surfaceVariant),
            ) {
                presetProviders.forEachIndexed { index, provider ->
                    val iconRes = AIModelPresetStore.getIconRes(provider.iconResName)
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable {
                                currentPreset = provider
                                presetSelectedModelIndex = -1
                                showPresetDialog = true
                            }
                            .padding(16.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        if (iconRes != 0) {
                            Image(
                                painter = painterResource(id = iconRes),
                                contentDescription = null,
                                modifier = Modifier
                                    .size(40.dp)
                                    .clip(RoundedCornerShape(8.dp))
                            )
                        }
                        Text(
                            text = provider.name,
                            style = MaterialTheme.typography.bodyLarge,
                            color = MaterialTheme.colorScheme.onSurface,
                            modifier = Modifier
                                .weight(1f)
                                .padding(start = 16.dp),
                        )
                        Icon(
                            Icons.Filled.KeyboardArrowRight,
                            contentDescription = null,
                            tint = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    if (index < presetProviders.lastIndex) {
                        HorizontalDivider(
                            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f),
                            modifier = Modifier.padding(horizontal = 16.dp),
                        )
                    }
                }
            }
        }
    }
}
