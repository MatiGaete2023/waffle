param(
    [string]$SourceDir = $PSScriptRoot,
    [string]$Destination = (Join-Path $env:LOCALAPPDATA 'Programs\FlowRust'),
    [string]$DataBaseDir = (Join-Path $env:LOCALAPPDATA 'FlowRust'),
    [string]$StartMenuDir = (Join-Path ([Environment]::GetFolderPath('Programs')) ''),
    [string]$DesktopDir = ([Environment]::GetFolderPath('Desktop')),
    [switch]$CreateDesktopShortcut,
    [string]$UninstallRegistryKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\FlowRust',
    [switch]$SkipRegistry,
    [string]$Version = '1.0.0',
    [switch]$RequireSha256,
    [switch]$SinHuella,
    [switch]$Silent
)

$ErrorActionPreference = 'Stop'

function Test-MzHeader {
    param([Parameter(Mandatory = $true)][string]$Path)
    $bytes = New-Object byte[] 2
    $fs = [IO.File]::OpenRead($Path)
    try {
        $n = $fs.Read($bytes, 0, 2)
    } finally {
        $fs.Dispose()
    }
    return ($n -eq 2 -and $bytes[0] -eq 0x4D -and $bytes[1] -eq 0x5A)
}

function Get-Sha256Hex {
    param([Parameter(Mandatory = $true)][string]$Path)
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        $fs = [IO.File]::OpenRead($Path)
        try {
            $hashBytes = $sha.ComputeHash($fs)
        } finally {
            $fs.Dispose()
        }
    } finally {
        $sha.Dispose()
    }
    return ([BitConverter]::ToString($hashBytes) -replace '-', '').ToLowerInvariant()
}

function Read-ExpectedSha256 {
    param([Parameter(Mandatory = $true)][string]$ShaPath)
    $lines = Get-Content -LiteralPath $ShaPath -Encoding UTF8
    $expected = $null
    foreach ($line in $lines) {
        if ($line -match '^\s*([0-9a-fA-F]{64})(?:\s+\*?flow_rust\.exe(?:\s+\(debug\))?)?\s*$') {
            $hash = $Matches[1].ToLowerInvariant()
            if ($expected -and $expected -ne $hash) { throw 'SHA256.txt contiene huellas contradictorias.' }
            $expected = $hash
        }
    }
    if ($expected) { return $expected }
    throw "SHA256.txt no contiene una huella SHA-256 valida: $ShaPath"
}

function New-ShortcutFile {
    param(
        [Parameter(Mandatory = $true)][string]$ShortcutPath,
        [Parameter(Mandatory = $true)][string]$TargetPath,
        [Parameter(Mandatory = $true)][string]$WorkingDirectory
    )
    $parent = Split-Path -Parent $ShortcutPath
    if ($parent) {
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
    }
    if ((Test-Path -LiteralPath $ShortcutPath) -and ((Get-Item -LiteralPath $ShortcutPath -Force).Attributes -band [IO.FileAttributes]::ReparsePoint)) {
        Write-Warning "Se conserva un enlace ajeno: $ShortcutPath"
        return
    }
    $wsh = New-Object -ComObject WScript.Shell
    try {
        $lnk = $wsh.CreateShortcut($ShortcutPath)
        if ((Test-Path -LiteralPath $ShortcutPath) -and $lnk.TargetPath -ne $TargetPath) {
            Write-Warning "Se conserva un acceso directo ajeno: $ShortcutPath"
            return
        }
        $lnk.TargetPath = $TargetPath
        $lnk.WorkingDirectory = $WorkingDirectory
        $lnk.IconLocation = "$TargetPath,0"
        $lnk.Description = 'Flow Rust - Asistente CSMP y Descargador'
        $lnk.Save()
    } finally {
        [void][Runtime.InteropServices.Marshal]::ReleaseComObject($wsh)
    }
}

$sourceFull = [IO.Path]::GetFullPath($SourceDir)
$destFull = [IO.Path]::GetFullPath($Destination).TrimEnd('\', '/')
$dataFull = [IO.Path]::GetFullPath($DataBaseDir)
$exeSource = Join-Path $sourceFull 'flow_rust.exe'
$shaSource = Join-Path $sourceFull 'SHA256.txt'
$uninstallScriptSource = Join-Path $PSScriptRoot 'Desinstalar_Flow.ps1'

if (-not (Test-Path -LiteralPath $exeSource -PathType Leaf)) {
    throw "No se encontro flow_rust.exe junto al instalador: $exeSource"
}
if (-not (Test-MzHeader -Path $exeSource)) {
    throw "El archivo flow_rust.exe no es un ejecutable Windows valido."
}

if ($RequireSha256 -and $SinHuella) {
    throw '-RequireSha256 es incompatible con -SinHuella.'
}
if (-not (Test-Path -LiteralPath $uninstallScriptSource -PathType Leaf)) {
    throw 'Falta Desinstalar_Flow.ps1 junto al instalador.'
}

$actualHash = Get-Sha256Hex -Path $exeSource
$hasShaFile = Test-Path -LiteralPath $shaSource -PathType Leaf
if (($RequireSha256 -or -not $SinHuella) -and -not $hasShaFile) {
    throw "Falta SHA256.txt junto a flow_rust.exe en: $sourceFull"
}
if ($hasShaFile) {
    $expectedHash = Read-ExpectedSha256 -ShaPath $shaSource
    if ($expectedHash -ne $actualHash) {
        throw "La huella SHA-256 de flow_rust.exe no coincide con SHA256.txt (esperada: $expectedHash, actual: $actualHash)."
    }
}

# Verificar que Flow no este ejecutandose desde la carpeta destino antes de reemplazar.
$destPrefix = $destFull.TrimEnd('\', '/') + '\'
$running = Get-Process -Name 'flow_rust' -ErrorAction SilentlyContinue | Where-Object {
    try {
        $_.Path -and $_.Path.StartsWith($destPrefix, [StringComparison]::OrdinalIgnoreCase)
    } catch {
        $false
    }
}
if ($running) {
    throw "Cierra Flow Rust antes de instalar o actualizar en: $destFull"
}

# No instalar a traves de enlaces/junctions ni sobre carpetas de datos.
$protected = @($env:USERPROFILE, $env:LOCALAPPDATA, $env:APPDATA,
    (Join-Path $env:LOCALAPPDATA 'FlowRust'), (Join-Path $env:APPDATA 'FlowRust'),
    ([Environment]::GetFolderPath('Desktop')), ([Environment]::GetFolderPath('MyDocuments')),
    (Join-Path $env:USERPROFILE 'Downloads'), $dataFull)
$knownFolders = Get-ItemProperty -LiteralPath 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\User Shell Folders' -ErrorAction SilentlyContinue
$downloads = $knownFolders.'{374DE290-123F-4565-9164-39C4925E467B}'
if ($downloads) { $protected += [Environment]::ExpandEnvironmentVariables($downloads) }
$protected += @((Join-Path $env:USERPROFILE 'Desktop'), (Join-Path $env:USERPROFILE 'Documents'))
foreach ($dir in $protected) {
    if ($dir -and $destFull -eq [IO.Path]::GetFullPath($dir).TrimEnd('\', '/')) {
        throw "Ruta de instalacion rechazada por seguridad: $destFull"
    }
}
foreach ($dataDir in @((Join-Path $env:LOCALAPPDATA 'FlowRust'), (Join-Path $env:APPDATA 'FlowRust'))) {
    $dataPrefix = [IO.Path]::GetFullPath($dataDir).TrimEnd('\', '/') + '\'
    if ($destFull.StartsWith($dataPrefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Ruta dentro de los datos de Flow rechazada: $destFull"
    }
}
for ($dir = $destFull; $dir; $dir = Split-Path -Parent $dir) {
    if ((Test-Path -LiteralPath $dir) -and ((Get-Item -LiteralPath $dir -Force).Attributes -band [IO.FileAttributes]::ReparsePoint)) {
        throw "Ruta de instalacion con enlace o junction: $dir"
    }
    if ($dir -eq (Split-Path -Parent $dir)) { break }
}
if ($destFull.Length -lt 10) { throw 'Ruta de instalacion demasiado corta.' }

if (-not $SkipRegistry -and $UninstallRegistryKey) {
    if (Test-Path -LiteralPath $UninstallRegistryKey) {
        $existing = Get-ItemProperty -LiteralPath $UninstallRegistryKey
        if ($existing.DisplayName -ne 'Flow Rust' -or $existing.InstallLocation -ne $destFull) {
            throw 'La clave de desinstalacion pertenece a otra instalacion; no se modifico.'
        }
    }
}

# Instalar por usuario sin requerir administrador y respetando %LOCALAPPDATA%\FlowRust\actualizaciones.
New-Item -ItemType Directory -Force -Path $destFull | Out-Null
$updatesDir = Join-Path $dataFull 'actualizaciones'
New-Item -ItemType Directory -Force -Path $updatesDir | Out-Null

$exeDest = Join-Path $destFull 'flow_rust.exe'
$uninstallDest = Join-Path $destFull 'Desinstalar_Flow.ps1'
$manifestPath = Join-Path $destFull '.flow_rust_manifest.json'
$allowed = @('flow_rust.exe', 'SHA256.txt', 'Desinstalar_Flow.ps1', 'LEEME.txt')
$owned = @()
if (Test-Path -LiteralPath $manifestPath) {
    $item = Get-Item -LiteralPath $manifestPath -Force
    if ($item.PSIsContainer -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw 'Manifiesto inseguro.' }
    $old = Get-Content -LiteralPath $manifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
    if ($old.schema -ne 'FlowRustOwnership/1' -or $old.destination -ne $destFull -or
        -not ($old.files -is [array]) -or $old.files -notcontains 'flow_rust.exe' -or $old.files -notcontains 'Desinstalar_Flow.ps1') {
        throw 'Manifiesto de propiedad invalido.'
    }
    foreach ($name in $old.files) {
        if ($name -isnot [string] -or $allowed -cnotcontains $name -or $owned -contains $name) { throw 'Manifiesto de propiedad invalido.' }
        $owned += $name
    }
}
$sources = [ordered]@{'flow_rust.exe' = $exeSource; 'Desinstalar_Flow.ps1' = $uninstallScriptSource}
if ($hasShaFile) { $sources['SHA256.txt'] = $shaSource }
$readme = Join-Path $sourceFull 'LEEME.txt'
if (Test-Path -LiteralPath $readme -PathType Leaf) { $sources['LEEME.txt'] = $readme }
foreach ($name in $allowed) {
    $path = Join-Path $destFull $name
    if (Test-Path -LiteralPath $path) {
        $item = Get-Item -LiteralPath $path -Force
        if ($item.PSIsContainer -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw "Archivo de destino inseguro: $path" }
        if ($sources.Contains($name) -and $owned -notcontains $name) { throw "Archivo preexistente sin propiedad Flow: $path" }
    }
}
# Preparar y verificar todos los archivos antes de reemplazar el primero.
$staged = [ordered]@{}
try {
    foreach ($name in $sources.Keys) {
        $target = Join-Path $destFull $name
        $tmp = "$target.$([guid]::NewGuid().ToString('N')).tmp"
        [IO.File]::Copy($sources[$name], $tmp, $false)
        $staged[$name] = $tmp
        if ($name -eq 'flow_rust.exe' -and (Get-Sha256Hex -Path $tmp) -ne $actualHash) {
            throw 'La huella SHA-256 de la copia no coincide.'
        }
        if ($name -eq 'SHA256.txt' -and (Read-ExpectedSha256 -ShaPath $tmp) -ne $actualHash) {
            throw 'La huella SHA-256 copiada no corresponde al ejecutable.'
        }
    }
    foreach ($name in $staged.Keys) {
        $target = Join-Path $destFull $name
        $tmp = $staged[$name]
        if ([IO.File]::Exists($target)) {
            if ($owned -notcontains $name) { throw "Archivo preexistente sin propiedad Flow: $target" }
            [IO.File]::Replace($tmp, $target, [NullString]::Value)
        } else { [IO.File]::Move($tmp, $target) }
    }
} finally {
    foreach ($tmp in $staged.Values) {
        if ([IO.File]::Exists($tmp)) { [IO.File]::Delete($tmp) }
    }
}
# Una instalacion sin huella no debe conservar la huella de la version anterior.
if (-not $hasShaFile -and $owned -contains 'SHA256.txt') {
    [IO.File]::Delete((Join-Path $destFull 'SHA256.txt'))
    $owned = @($owned | Where-Object { $_ -ne 'SHA256.txt' })
}
$owned = @(@($owned) + @($sources.Keys) | Sort-Object -Unique | Where-Object { Test-Path -LiteralPath (Join-Path $destFull $_) -PathType Leaf })
$manifestTmp = "$manifestPath.$([guid]::NewGuid().ToString('N')).tmp"
try {
    $manifest = @{ schema = 'FlowRustOwnership/1'; destination = $destFull; files = $owned } | ConvertTo-Json -Depth 4
    $stream = [IO.File]::Open($manifestTmp, [IO.FileMode]::CreateNew)
    try {
        $bytes = [Text.Encoding]::UTF8.GetBytes($manifest)
        $stream.Write($bytes, 0, $bytes.Length)
    } finally { $stream.Dispose() }
    if ([IO.File]::Exists($manifestPath)) { [IO.File]::Replace($manifestTmp, $manifestPath, [NullString]::Value) }
    else { [IO.File]::Move($manifestTmp, $manifestPath) }
} finally {
    if ([IO.File]::Exists($manifestTmp)) { [IO.File]::Delete($manifestTmp) }
}

$startMenuShortcut = $null
if ($StartMenuDir) {
    $startMenuShortcut = Join-Path ([IO.Path]::GetFullPath($StartMenuDir)) 'Flow.lnk'
    New-ShortcutFile -ShortcutPath $startMenuShortcut -TargetPath $exeDest -WorkingDirectory $destFull
}

$desktopShortcut = $null
if ($CreateDesktopShortcut -and $DesktopDir) {
    $desktopShortcut = Join-Path ([IO.Path]::GetFullPath($DesktopDir)) 'Flow.lnk'
    New-ShortcutFile -ShortcutPath $desktopShortcut -TargetPath $exeDest -WorkingDirectory $destFull
}

if (-not $SkipRegistry -and $UninstallRegistryKey) {
    New-Item -Path $UninstallRegistryKey -Force | Out-Null
    # Comando PowerShell con literales escapados, sin alternativa de borrado recursivo.
    $quotedDest = $destFull.Replace("'", "''")
    $quotedScript = $uninstallDest.Replace("'", "''")
    $uninstCmd = "powershell.exe -NoProfile -ExecutionPolicy Bypass -Command `"& '$quotedScript' -Destination '$quotedDest'`""
    $quietCmd = "powershell.exe -NoProfile -ExecutionPolicy Bypass -Command `"& '$quotedScript' -Destination '$quotedDest' -Silent`""
    New-ItemProperty -Path $UninstallRegistryKey -Name 'DisplayName' -Value 'Flow Rust' -PropertyType String -Force | Out-Null
    New-ItemProperty -Path $UninstallRegistryKey -Name 'DisplayVersion' -Value $Version -PropertyType String -Force | Out-Null
    New-ItemProperty -Path $UninstallRegistryKey -Name 'Publisher' -Value 'NuRus' -PropertyType String -Force | Out-Null
    New-ItemProperty -Path $UninstallRegistryKey -Name 'InstallLocation' -Value $destFull -PropertyType String -Force | Out-Null
    New-ItemProperty -Path $UninstallRegistryKey -Name 'DisplayIcon' -Value "$exeDest,0" -PropertyType String -Force | Out-Null
    New-ItemProperty -Path $UninstallRegistryKey -Name 'UninstallString' -Value $uninstCmd -PropertyType String -Force | Out-Null
    New-ItemProperty -Path $UninstallRegistryKey -Name 'QuietUninstallString' -Value $quietCmd -PropertyType String -Force | Out-Null
    New-ItemProperty -Path $UninstallRegistryKey -Name 'NoModify' -Value 1 -PropertyType DWord -Force | Out-Null
    New-ItemProperty -Path $UninstallRegistryKey -Name 'NoRepair' -Value 1 -PropertyType DWord -Force | Out-Null
}

if (-not $Silent) {
    Write-Host "Flow Rust instalado para el usuario actual en: $destFull"
    if ($hasShaFile) {
        Write-Host "SHA-256 verificado: $actualHash"
    } else {
        Write-Host "SHA-256 no verificado (instalación sin SHA256.txt)"
    }
    Write-Host "Carpeta de actualizaciones (E-I2) preservada en: $updatesDir"
    if ($startMenuShortcut) {
        Write-Host "Acceso directo en menu Inicio: $startMenuShortcut"
    }
}
