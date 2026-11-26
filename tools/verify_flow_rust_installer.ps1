param(
    [Parameter(Mandatory = $true)][string]$BundleDir,
    [string]$InnoInstaller = '',
    [switch]$SkipSelfTest,
    [string]$EvidenceDirectory = ''
)

$ErrorActionPreference = 'Stop'

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

function Assert-FileEquals {
    param([string]$Path, [string]$Expected)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Falta archivo preservado esperado: $Path"
    }
    $actual = (Get-Content -LiteralPath $Path -Raw -Encoding UTF8).TrimEnd("`r", "`n")
    if ($actual -ne $Expected) {
        throw "Contenido alterado en $Path (esperado '$Expected', actual '$actual')"
    }
}

$bundleFull = (Resolve-Path -LiteralPath $BundleDir).Path
$installScript = Join-Path $bundleFull 'Instalar_Flow.ps1'
if (-not (Test-Path -LiteralPath $installScript -PathType Leaf)) {
    $installScript = (Resolve-Path (Join-Path $PSScriptRoot '..\installer\Instalar_Flow.ps1')).Path
}
$exeBundle = Join-Path $bundleFull 'flow_rust.exe'
$shaBundle = Join-Path $bundleFull 'SHA256.txt'
if (-not (Test-Path -LiteralPath $exeBundle -PathType Leaf)) {
    throw "No se encontro flow_rust.exe en el paquete: $bundleFull"
}
if (-not (Test-Path -LiteralPath $shaBundle -PathType Leaf)) {
    throw "No se encontro SHA256.txt en el paquete: $bundleFull"
}

$rootTmp = Join-Path ([IO.Path]::GetTempPath()) ('flow-rust-inst-check-' + [guid]::NewGuid().ToString('N'))
$tamperBundle = Join-Path $rootTmp 'tamper_bundle'
$tamperDest = Join-Path $rootTmp 'tamper_dest'
$dest = Join-Path $rootTmp 'programa\FlowRust'
$dataBase = Join-Path $rootTmp 'localappdata\FlowRust'
$startMenu = Join-Path $rootTmp 'menu'
$regKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\FlowRust_Prueba_" + ([guid]::NewGuid().ToString('N').Substring(0, 8))

try {
    New-Item -ItemType Directory -Force -Path $tamperBundle, $rootTmp | Out-Null

    # 1. Huella SHA-256 alterada debe rechazar la instalacion antes de copiar el programa.
    Copy-Item -LiteralPath $exeBundle -Destination (Join-Path $tamperBundle 'flow_rust.exe') -Force
    "$('0' * 64)  flow_rust.exe" | Set-Content -LiteralPath (Join-Path $tamperBundle 'SHA256.txt') -Encoding UTF8
    $tamperExit = 0
    try {
        & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $installScript `
            -SourceDir $tamperBundle -Destination $tamperDest -DataBaseDir (Join-Path $rootTmp 'tamper_data') `
            -StartMenuDir $startMenu -SkipRegistry -RequireSha256 -Silent 2>$null
        $tamperExit = $LASTEXITCODE
    } catch {
        $tamperExit = 1
    }
    if ($tamperExit -eq 0 -or (Test-Path -LiteralPath (Join-Path $tamperDest 'flow_rust.exe'))) {
        throw 'El instalador acepto un SHA256.txt manipulado.'
    }
    Write-Host 'APROBADO: rechazo de huella SHA-256 manipulada'

    # 1b. Ejecutar el CMD real desde una ruta con espacios, sin tocar perfiles reales.
    $cmdBundle = Join-Path $rootTmp 'paquete con espacios'
    $cmdDest = Join-Path $rootTmp "programa con espacios y 'comilla'"
    $cmdData = Join-Path $rootTmp 'datos cmd'
    $cmdMenu = Join-Path $rootTmp 'menu cmd'
    New-Item -ItemType Directory -Path $cmdBundle | Out-Null
    $installerDir = Split-Path -Parent $installScript
    foreach ($name in @('Instalar_Flow.cmd', 'Instalar_Flow.ps1', 'Desinstalar_Flow.ps1')) {
        Copy-Item -LiteralPath (Join-Path $installerDir $name) -Destination $cmdBundle
    }
    Copy-Item -LiteralPath $exeBundle, $shaBundle -Destination $cmdBundle
    'LEEME sintetico' | Set-Content -LiteralPath (Join-Path $cmdBundle 'LEEME.txt') -Encoding UTF8
    Push-Location -LiteralPath $cmdBundle
    try {
        # La entrada por pipe libera el pause del lanzador.
        $cmdLine = 'Instalar_Flow.cmd -Destination "{0}" -DataBaseDir "{1}" -StartMenuDir "{2}" -SkipRegistry -Silent < NUL' -f $cmdDest, $cmdData, $cmdMenu
        $cmdRun = Start-Process -FilePath 'cmd.exe' -ArgumentList @('/d', '/c', $cmdLine) -WorkingDirectory $cmdBundle -NoNewWindow -Wait -PassThru
        if ($cmdRun.ExitCode -ne 0) { throw "Fallo CMD desde ruta con espacios: $($cmdRun.ExitCode)" }
        if ((Get-Sha256Hex (Join-Path $cmdDest 'flow_rust.exe')) -ne (Get-Sha256Hex $exeBundle)) { throw 'CMD no instalo el ejecutable verificado.' }
        Assert-FileEquals (Join-Path $cmdDest 'LEEME.txt') 'LEEME sintetico'
        # Probar que RequireSha256 no se pierde en el paso por CMD.
        Remove-Item -LiteralPath (Join-Path $cmdBundle 'SHA256.txt')
        $badCmdDest = Join-Path $rootTmp 'cmd sin huella'
        $badCmd = 'Instalar_Flow.cmd -Destination "{0}" -DataBaseDir "{1}" -StartMenuDir "{2}" -SkipRegistry -Silent < NUL' -f $badCmdDest, $cmdData, $cmdMenu
        $badExit = 0
        $badExit = (Start-Process -FilePath 'cmd.exe' -ArgumentList @('/d', '/c', $badCmd) -WorkingDirectory $cmdBundle -NoNewWindow -Wait -PassThru).ExitCode
        if ($badExit -eq 0 -or (Test-Path -LiteralPath (Join-Path $badCmdDest 'flow_rust.exe'))) { throw 'CMD acepto una entrega sin SHA256.txt.' }
        # Rechazar combinacion contradictoria incluso con una huella valida.
        Copy-Item -LiteralPath $shaBundle -Destination $cmdBundle
        $badExit = 0
        $badExit = (Start-Process -FilePath 'cmd.exe' -ArgumentList @('/d', '/c', ($badCmd.Replace(' -Silent', ' -Silent -SinHuella'))) -WorkingDirectory $cmdBundle -NoNewWindow -Wait -PassThru).ExitCode
        if ($badExit -eq 0 -or (Test-Path -LiteralPath (Join-Path $badCmdDest 'flow_rust.exe'))) { throw 'CMD acepto RequireSha256 junto con SinHuella.' }
    } finally { Pop-Location }
    Write-Host 'APROBADO: CMD con espacios, LEEME, SHA256 obligatorio y opciones incompatibles'

    # 1c. Una carpeta de extraccion con el script, pero sin manifiesto, nunca se borra.
    $unownedDir = Join-Path $rootTmp 'extraccion sin manifiesto'
    New-Item -ItemType Directory -Path $unownedDir | Out-Null
    $unownedScript = Join-Path $unownedDir 'Desinstalar_Flow.ps1'
    Copy-Item -LiteralPath (Join-Path $installerDir 'Desinstalar_Flow.ps1') -Destination $unownedScript
    $unownedSentinel = Join-Path $unownedDir 'archivo ajeno.txt'
    'testigo ajeno' | Set-Content -LiteralPath $unownedSentinel -Encoding UTF8
    $unownedExit = 0
    try {
        & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $unownedScript -StartMenuDir $cmdMenu -SkipRegistry -Silent 2>$null
        $unownedExit = $LASTEXITCODE
    } catch { $unownedExit = 1 }
    if ($unownedExit -eq 0 -or -not (Test-Path -LiteralPath $unownedScript)) { throw 'Desinstalador acepto carpeta sin manifiesto.' }
    Assert-FileEquals $unownedSentinel 'testigo ajeno'
    Write-Host 'APROBADO: rechazo de carpeta sin manifiesto'

    # 2. Instalacion limpia por usuario.
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $installScript `
        -SourceDir $bundleFull -Destination $dest -DataBaseDir $dataBase `
        -StartMenuDir $startMenu -UninstallRegistryKey $regKey -RequireSha256 -Silent
    if ($LASTEXITCODE -ne 0) { throw "Fallo la instalacion limpia (exit $LASTEXITCODE)" }

    $installedExe = Join-Path $dest 'flow_rust.exe'
    $updatesDir = Join-Path $dataBase 'actualizaciones'
    $shortcutPath = Join-Path $startMenu 'Flow.lnk'
    foreach ($req in @($installedExe, (Join-Path $dest 'SHA256.txt'), (Join-Path $dest 'Desinstalar_Flow.ps1'), (Join-Path $dest '.flow_rust_manifest.json'), $updatesDir, $shortcutPath)) {
        if (-not (Test-Path -LiteralPath $req)) {
            throw "Falta artefacto tras instalar: $req"
        }
    }

    $origHash = Get-Sha256Hex -Path $exeBundle
    $instHash = Get-Sha256Hex -Path $installedExe
    if ($origHash -ne $instHash) {
        throw 'La huella del ejecutable instalado difiere del origen.'
    }

    $wsh = New-Object -ComObject WScript.Shell
    try {
        $lnk = $wsh.CreateShortcut($shortcutPath)
        if ($lnk.TargetPath -ne $installedExe) {
            throw "TargetPath inesperado en Flow.lnk: $($lnk.TargetPath)"
        }
    } finally {
        [void][Runtime.InteropServices.Marshal]::ReleaseComObject($wsh)
    }

    $regProps = Get-ItemProperty -LiteralPath $regKey
    if ($regProps.DisplayName -ne 'Flow Rust' -or $regProps.InstallLocation -ne $dest) {
        throw 'Registro de desinstalacion HKCU incompleto.'
    }
    Write-Host 'APROBADO: instalacion por usuario, huella, acceso directo y registro HKCU'

    # 3. Ejecutar --self-test desde el ejecutable instalado.
    if (-not $SkipSelfTest) {
        $diagJson = if ($EvidenceDirectory) {
            New-Item -ItemType Directory -Force -Path $EvidenceDirectory | Out-Null
            Join-Path (Resolve-Path -LiteralPath $EvidenceDirectory).Path 'installed-self-test.json'
        } else {
            Join-Path $rootTmp 'installed-self-test.json'
        }
        $proc = Start-Process -FilePath $installedExe -ArgumentList @('--self-test', $diagJson) -Wait -PassThru -NoNewWindow
        if ($proc.ExitCode -ne 0) {
            throw "El ejecutable instalado fallo --self-test (exit $($proc.ExitCode))"
        }
        $proof = Get-Content -LiteralPath $diagJson -Raw -Encoding UTF8 | ConvertFrom-Json
        if (-not $proof.ok -or $proof.comprobaciones.Count -lt 1) {
            throw 'El informe --self-test del ejecutable instalado no reporto ok=true.'
        }
        Write-Host "APROBADO: --self-test desde ejecutable instalado ($($proof.comprobaciones.Count) comprobaciones)"
    }

    # 4. Reinstalacion y desinstalacion deben conservar actualizaciones/ (E-I2) y extensiones.
    $pendingExe = Join-Path $updatesDir 'flow_rust.exe'
    $pendingSha = Join-Path $updatesDir 'SHA256.txt'
    $appliedDir = Join-Path $updatesDir 'aplicadas'
    $appliedSentinel = Join-Path $appliedDir 'v0.txt'
    $extDir = Join-Path $dataBase 'extension-integral'
    $extManifest = Join-Path $extDir 'manifest.json'
    New-Item -ItemType Directory -Force -Path $appliedDir, $extDir | Out-Null
    'MZpendiente-sintetico' | Set-Content -LiteralPath $pendingExe -Encoding UTF8
    'abc  flow_rust.exe' | Set-Content -LiteralPath $pendingSha -Encoding UTF8
    'version-previa-aplicada' | Set-Content -LiteralPath $appliedSentinel -Encoding UTF8
    '{"name":"Sitfa Ficticia"}' | Set-Content -LiteralPath $extManifest -Encoding UTF8
    'MZanterior' | Set-Content -LiteralPath (Join-Path $dest 'flow_rust.exe.anterior') -Encoding UTF8

    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $installScript `
        -SourceDir $bundleFull -Destination $dest -DataBaseDir $dataBase `
        -StartMenuDir $startMenu -UninstallRegistryKey $regKey -RequireSha256 -Silent
    if ($LASTEXITCODE -ne 0) { throw "Fallo la reinstalacion (exit $LASTEXITCODE)" }

    Assert-FileEquals $pendingExe 'MZpendiente-sintetico'
    Assert-FileEquals $pendingSha 'abc  flow_rust.exe'
    Assert-FileEquals $appliedSentinel 'version-previa-aplicada'
    Assert-FileEquals $extManifest '{"name":"Sitfa Ficticia"}'
    Write-Host 'APROBADO: reinstalacion preserva actualizaciones (E-I2) y extension'

    # 5. Desinstalacion retira el programa, Flow.lnk y HKCU sin tocar actualizaciones ni extension.
    $sentinel = Join-Path $dest 'archivo ajeno.txt'
    'testigo ajeno' | Set-Content -LiteralPath $sentinel -Encoding UTF8
    $uninst = Join-Path $dest 'Desinstalar_Flow.ps1'
    $manifestPath = Join-Path $dest '.flow_rust_manifest.json'
    $manifestOriginal = [IO.File]::ReadAllBytes($manifestPath)
    $badManifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
    $badManifest.files += 'archivo ajeno.txt'
    try {
        $badManifest | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $manifestPath -Encoding UTF8
        $badExit = 0
        try {
            & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $uninst -Destination $dest -StartMenuDir $startMenu -SkipRegistry -Silent 2>$null
            $badExit = $LASTEXITCODE
        } catch { $badExit = 1 }
        if ($badExit -eq 0 -or -not (Test-Path -LiteralPath $installedExe)) { throw 'Desinstalador acepto manifiesto con archivo ajeno.' }
        Assert-FileEquals $sentinel 'testigo ajeno'
    } finally { [IO.File]::WriteAllBytes($manifestPath, $manifestOriginal) }
    Write-Host 'APROBADO: manifiesto adulterado no elimina archivos'
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $uninst `
        -Destination ($dest + '\') -StartMenuDir $startMenu -UninstallRegistryKey $regKey -Silent
    if ($LASTEXITCODE -ne 0) { throw "Fallo la desinstalacion (exit $LASTEXITCODE)" }

    Assert-FileEquals $sentinel 'testigo ajeno'
    foreach ($name in @('flow_rust.exe', 'SHA256.txt', 'Desinstalar_Flow.ps1', 'LEEME.txt', 'flow_rust.exe.anterior', 'flow_rust.exe.intercambio', '.flow_rust_manifest.json')) {
        if (Test-Path -LiteralPath (Join-Path $dest $name)) { throw "Archivo propio no retirado: $name" }
    }
    # El CMD instaló otra carpeta sin testigos: debe desaparecer si queda vacía.
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $cmdDest 'Desinstalar_Flow.ps1') `
        -Destination $cmdDest -StartMenuDir $cmdMenu -SkipRegistry -Silent
    if ($LASTEXITCODE -ne 0 -or (Test-Path -LiteralPath $cmdDest)) { throw 'La carpeta vacia de la instalacion CMD no se retiro.' }
    if (Test-Path -LiteralPath $shortcutPath) { throw "El acceso directo Flow.lnk sigue existiendo tras desinstalar." }
    if (Test-Path -LiteralPath $regKey) { throw "La clave HKCU Uninstall sigue existiendo tras desinstalar." }

    Assert-FileEquals $pendingExe 'MZpendiente-sintetico'
    Assert-FileEquals $pendingSha 'abc  flow_rust.exe'
    Assert-FileEquals $appliedSentinel 'version-previa-aplicada'
    Assert-FileEquals $extManifest '{"name":"Sitfa Ficticia"}'
    Write-Host 'APROBADO: desinstalacion limpia conservando actualizaciones (E-I2) y extension'

    # 6. Si se entrega instalador Inno Setup (.exe), verificar instalacion silenciosa, --self-test y unins000.exe.
    if ($InnoInstaller) {
        $innoExe = (Resolve-Path -LiteralPath $InnoInstaller).Path
        $innoDest = Join-Path $rootTmp 'inno_dest\FlowRust'
        $realUpdates = Join-Path $env:LOCALAPPDATA 'FlowRust\actualizaciones'
        New-Item -ItemType Directory -Force -Path $realUpdates | Out-Null
        $innoSentinel = Join-Path $realUpdates ("prueba-ei1-" + [guid]::NewGuid().ToString('N') + '.txt')
        'centinela-ei1' | Set-Content -LiteralPath $innoSentinel -Encoding UTF8
        try {
            $procInst = Start-Process -FilePath $innoExe -ArgumentList @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', "/DIR=`"$innoDest`"") -Wait -PassThru
            if ($procInst.ExitCode -ne 0) { throw "Fallo instalador Inno Setup (exit $($procInst.ExitCode))" }
            $innoProgram = Join-Path $innoDest 'flow_rust.exe'
            if (-not (Test-Path -LiteralPath $innoProgram -PathType Leaf)) {
                throw 'El instalador Inno Setup no copio flow_rust.exe'
            }
            if (-not $SkipSelfTest) {
                $innoDiag = Join-Path $rootTmp 'inno-self-test.json'
                $procDiag = Start-Process -FilePath $innoProgram -ArgumentList @('--self-test', $innoDiag) -Wait -PassThru -NoNewWindow
                if ($procDiag.ExitCode -ne 0) { throw 'Fallo --self-test desde instalacion Inno Setup' }
            }
            $uninsExe = Join-Path $innoDest 'unins000.exe'
            $procUninst = Start-Process -FilePath $uninsExe -ArgumentList @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART') -Wait -PassThru
            if ($procUninst.ExitCode -ne 0) { throw "Fallo unins000.exe (exit $($procUninst.ExitCode))" }
            $sw = [Diagnostics.Stopwatch]::StartNew()
            while ((Test-Path -LiteralPath $innoDest) -and $sw.Elapsed.TotalSeconds -lt 30) {
                Start-Sleep -Milliseconds 200
            }
            if (Test-Path -LiteralPath $innoDest) {
                throw 'unins000.exe no elimino la carpeta del programa.'
            }
            Assert-FileEquals $innoSentinel 'centinela-ei1'
            Write-Host 'APROBADO: instalador Inno Setup (.exe), --self-test y preservacion de actualizaciones'
        } finally {
            Remove-Item -LiteralPath $innoSentinel -ErrorAction SilentlyContinue
        }
    }
} finally {
    if (Test-Path -LiteralPath $regKey) {
        Remove-Item -LiteralPath $regKey -Recurse -Force -ErrorAction SilentlyContinue
    }
    Remove-Item -LiteralPath $rootTmp -Recurse -Force -ErrorAction SilentlyContinue
}
