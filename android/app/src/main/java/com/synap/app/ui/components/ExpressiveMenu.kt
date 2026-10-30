@file:OptIn(ExperimentalMaterial3ExpressiveApi::class)

package com.synap.app.ui.components

import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Check
import androidx.compose.material3.DropdownMenuGroup
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.DropdownMenuPopup
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.MenuDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector

@Composable
fun ExpressiveMenu(
    expanded: Boolean,
    onDismissRequest: () -> Unit,
    modifier: Modifier = Modifier,
    content: @Composable ColumnScope.() -> Unit,
) {
    DropdownMenuPopup(
        expanded = expanded,
        onDismissRequest = onDismissRequest,
        modifier = modifier,
    ) {
        DropdownMenuGroup(
            shapes = MenuDefaults.groupShape(0, 1),
            content = content,
        )
    }
}

@Composable
fun ExpressiveMenuItem(
    text: String,
    onClick: () -> Unit,
    index: Int,
    count: Int,
    leadingIcon: ImageVector? = null,
) {
    DropdownMenuItem(
        text = { Text(text) },
        onClick = onClick,
        shape = MenuDefaults.itemShape(index, count).shape,
        leadingIcon = leadingIcon?.let { { Icon(it, contentDescription = null) } },
    )
}

@Composable
fun ExpressiveSelectableMenuItem(
    text: String,
    selected: Boolean,
    onClick: () -> Unit,
    index: Int,
    count: Int,
) {
    DropdownMenuItem(
        checked = selected,
        onCheckedChange = { onClick() },
        text = { Text(text) },
        shapes = MenuDefaults.itemShape(index, count),
        checkedLeadingIcon = { Icon(Icons.Filled.Check, contentDescription = null) },
    )
}
