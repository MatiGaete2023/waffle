param(
    [string]$Destination = $PSScriptRoot,
    [string]$StartMenuDir = (Join-Path ([Environment]::GetFolderPath('Programs')) ''),
    [string]$DesktopDir = ([Environment]::GetFolderPath('Desktop')),
    [string]$UninstallRegistryKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\FlowRust',
    [switch]$SkipRegistry,
    [switch]$Silent
)

$ErrorActionPreference = 'Stop'

if (-not $Destination) {
    throw 'Se requiere la carpeta de instalacion (-Destination).'
}

$destFull = [IO.Path]::GetFullPath($Destination).TrimEnd('\', '/')
$localApp = [IO.Path]::GetFullPath($env:LOCALAPPDATA).TrimEnd('\', '/')
$roamingApp = [IO.Path]::GetFullPath($env:APPDATA).TrimEnd('\', '/')
$flowDataDir = Join-Path $localApp 'FlowRust'
$flowRoamingDir = Join-Path $roamingApp 'FlowRust'
$protected = @($localApp, $roamingApp, $flowDataDir, $flowRoamingDir, $env:USERPROFILE,
    ([Environment]::GetFolderPath('Desktop')), ([Environment]::GetFolderPath('MyDocuments')),
    (Join-Path $env:USERPROFILE 'Downloads'), $DesktopDir, $StartMenuDir)
$knownFolders = Get-ItemProperty -LiteralPath 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\User Shell Folders' -ErrorAction SilentlyContinue
$downloads = $knownFolders.'{374DE290-123F-4565-9164-39C4925E467B}'
if ($downloads) { $protected += [Environment]::ExpandEnvironmentVariables($downloads) }
$protected += @((Join-Path $env:USERPROFILE 'Desktop'), (Join-Path $env:USERPROFILE 'Documents'))
foreach ($dir in $protected) {
    if ($dir -and $destFull -eq [IO.Path]::GetFullPath($dir).TrimEnd('\', '/')) {
        throw "Ruta de desinstalacion rechazada por seguridad: $destFull"
    }
}
if ($destFull.Length -lt 10) { throw "Ruta de desinstalacion demasiado corta: $destFull" }
foreach ($dataDir in @((Join-Path $env:LOCALAPPDATA 'FlowRust'), (Join-Path $env:APPDATA 'FlowRust'))) {
    $dataPrefix = [IO.Path]::GetFullPath($dataDir).TrimEnd('\', '/') + '\'
    if ($destFull.StartsWith($dataPrefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Ruta dentro de los datos de Flow rechazada: $destFull"
    }
}
for ($dir = $destFull; $dir; $dir = Split-Path -Parent $dir) {
    if ((Test-Path -LiteralPath $dir) -and ((Get-Item -LiteralPath $dir -Force).Attributes -band [IO.FileAttributes]::ReparsePoint)) {
        throw "Ruta de desinstalacion con enlace o junction: $dir"
    }
    if ($dir -eq (Split-Path -Parent $dir)) { break }
}
$manifestPath = Join-Path $destFull '.flow_rust_manifest.json'
if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
    throw "La carpeta indicada no corresponde a una instalacion de Flow Rust: falta manifiesto de propiedad."
}
if ((Get-Item -LiteralPath $manifestPath -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Manifiesto inseguro.' }
$manifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
$allowed = @('flow_rust.exe', 'SHA256.txt', 'Desinstalar_Flow.ps1', 'LEEME.txt')
if ($manifest.schema -ne 'FlowRustOwnership/1' -or $manifest.destination -ne $destFull -or
    -not ($manifest.files -is [array]) -or $manifest.files -notcontains 'flow_rust.exe' -or $manifest.files -notcontains 'Desinstalar_Flow.ps1') {
    throw 'Manifiesto de propiedad invalido.'
}
$owned = @()
foreach ($name in $manifest.files) {
    if ($name -isnot [string] -or $allowed -cnotcontains $name -or $owned -contains $name) { throw 'Manifiesto de propiedad invalido.' }
    $owned += $name
}
$owned += @('flow_rust.exe.anterior', 'flow_rust.exe.intercambio')
# Validar todos los objetos antes de borrar el primero.
foreach ($name in $owned) {
    $path = Join-Path $destFull $name
    if (Test-Path -LiteralPath $path) {
        $item = Get-Item -LiteralPath $path -Force
        if ($item.PSIsContainer -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw "Archivo de instalacion inseguro: $path" }
    }
}
if (Test-Path -LiteralPath $destFull -PathType Container) {
    $destPrefix = $destFull.TrimEnd('\', '/') + '\'
    $running = Get-Process -Name 'flow_rust' -ErrorAction SilentlyContinue | Where-Object {
        try {
            $_.Path -and $_.Path.StartsWith($destPrefix, [StringComparison]::OrdinalIgnoreCase)
        } catch {
            $false
        }
    }
    if ($running) {
        throw "Cierra Flow Rust antes de desinstalarlo desde: $destFull"
    }

    # Si PowerShell esta corriendo con cwd dentro de $destFull, salir antes de borrar.
    $cwd = [IO.Path]::GetFullPath((Get-Location).Path)
    if ($cwd.Equals($destFull, [StringComparison]::OrdinalIgnoreCase) -or
        $cwd.StartsWith($destPrefix, [StringComparison]::OrdinalIgnoreCase)) {
        Set-Location -LiteralPath (Split-Path -Parent $destFull)
    }

    foreach ($name in $owned) {
        $path = Join-Path $destFull $name
        if (Test-Path -LiteralPath $path -PathType Leaf) { Remove-Item -LiteralPath $path -Force }
    }
    Remove-Item -LiteralPath $manifestPath -Force
    if (@(Get-ChildItem -LiteralPath $destFull -Force).Count -eq 0) {
        [IO.Directory]::Delete($destFull, $false)
    }
}

# Retirar solo accesos directos que realmente apuntan a esta instalacion.
foreach ($dir in @($StartMenuDir, $DesktopDir)) {
    if (-not $dir) { continue }
    $lnk = Join-Path ([IO.Path]::GetFullPath($dir)) 'Flow.lnk'
    if (Test-Path -LiteralPath $lnk -PathType Leaf) {
        if ((Get-Item -LiteralPath $lnk -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) { continue }
        $wsh = New-Object -ComObject WScript.Shell
        try {
            $shortcut = $wsh.CreateShortcut($lnk)
            if ($shortcut.TargetPath -eq (Join-Path $destFull 'flow_rust.exe')) { Remove-Item -LiteralPath $lnk -Force }
        } finally { [void][Runtime.InteropServices.Marshal]::ReleaseComObject($wsh) }
    }
}
if (-not $SkipRegistry -and $UninstallRegistryKey -and (Test-Path -LiteralPath $UninstallRegistryKey)) {
    $props = Get-ItemProperty -LiteralPath $UninstallRegistryKey
    if ($props.InstallLocation -eq $destFull -and $props.DisplayName -eq 'Flow Rust') {
        if (@(Get-ChildItem -LiteralPath $UninstallRegistryKey).Count -eq 0) {
            Remove-Item -LiteralPath $UninstallRegistryKey -Force
        } else { Write-Warning 'La clave contiene subclaves; no se elimino.' }
    }
}

if (-not $Silent) {
    Write-Host "Flow Rust desinstalado de: $destFull"
    Write-Host "Las actualizaciones y datos del usuario (%LOCALAPPDATA%\FlowRust y %APPDATA%\FlowRust) se conservaron."
}
