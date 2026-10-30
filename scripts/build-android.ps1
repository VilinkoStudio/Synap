param(
    [switch]$Release,
    [string]$VersionName,
    [int]$VersionCode = 0
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$androidDir = Join-Path $repoRoot "android"
$keystoreProperties = Join-Path $androidDir "keystore.properties"

if (-not (Test-Path $androidDir)) {
    throw "Android directory not found: $androidDir"
}

if ($Release -and -not (Test-Path $keystoreProperties)) {
    Write-Warning "未找到 android\keystore.properties，release APK 将以未签名形式输出。"
    Write-Warning "如需签名，请创建 keystore.properties（storeFile/storePassword/keyAlias/keyPassword）或设置 SIGNING_* 环境变量。"
}

Push-Location $androidDir
try {
    $task = if ($Release) { "assembleRelease" } else { "assembleDebug" }
    $gradleArgs = @()
    if ($VersionName) { $gradleArgs += "-PversionName=$VersionName" }
    if ($VersionCode -gt 0) { $gradleArgs += "-PversionCode=$VersionCode" }

    Write-Host "Running :app:$task ..."
    if ($VersionName -or $VersionCode -gt 0) {
        Write-Host "VersionName: $(if ($VersionName) { $VersionName } else { '<git>' }), VersionCode: $(if ($VersionCode -gt 0) { $VersionCode } else { '1' })"
    }

    & .\gradlew.bat $task @gradleArgs
    if ($LASTEXITCODE -ne 0) {
        throw "Gradle build failed: $task"
    }

    $apkDir = if ($Release) {
        Join-Path $androidDir "app\build\outputs\apk\release"
    } else {
        Join-Path $androidDir "app\build\outputs\apk\debug"
    }

    $apks = Get-ChildItem -Path $apkDir -Filter "*.apk" -ErrorAction SilentlyContinue
    if (-not $apks) {
        throw "No APK produced in $apkDir"
    }

    $distDir = Join-Path $repoRoot "dist"
    New-Item -ItemType Directory -Force -Path $distDir | Out-Null

    foreach ($apk in $apks) {
        if ($apk.Name -like "*unsigned*") {
            Write-Warning "未签名 APK：$($apk.FullName)"
        }
        $target = Join-Path $distDir $apk.Name
        Copy-Item -LiteralPath $apk.FullName -Destination $target -Force
        Write-Host "APK: $target"
    }
}
finally {
    Pop-Location
}
