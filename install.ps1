# Install geodesic-wallpaper on Windows.
#
#   irm https://raw.githubusercontent.com/Mattbusel/geodesic-wallpaper/master/install.ps1 | iex
#
# Downloads the latest release zip, checks its SHA-256 against SHA256SUMS.txt,
# unpacks it to %LOCALAPPDATA%\Programs\geodesic-wallpaper (exe, config.toml,
# presets\), adds that folder to your user PATH and adds a Start Menu shortcut.
# It does not start the wallpaper. Set $env:GEODESIC_VERSION = "v1.5.1" to pin
# a version. To uninstall, delete that folder and the Start Menu shortcut.

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$repo   = 'Mattbusel/geodesic-wallpaper'
$name   = 'geodesic-wallpaper'
$target = 'x86_64-pc-windows-msvc'

if (-not [Environment]::Is64BitOperatingSystem) {
    throw "$name needs 64-bit Windows 10 or 11."
}

$tag = $env:GEODESIC_VERSION
if (-not $tag) {
    Write-Host "Looking up the latest release..."
    $tag = (Invoke-RestMethod "https://api.github.com/repos/$repo/releases/latest" -Headers @{ 'User-Agent' = 'install.ps1' }).tag_name
}

$zipName = "$name-$tag-$target.zip"
$base    = "https://github.com/$repo/releases/download/$tag"
$tmp     = Join-Path ([IO.Path]::GetTempPath()) "$name-install-$([guid]::NewGuid().ToString('N'))"
New-Item -ItemType Directory -Path $tmp | Out-Null

try {
    Write-Host "Downloading $zipName"
    $zip = Join-Path $tmp $zipName
    Invoke-WebRequest "$base/$zipName" -OutFile $zip -UseBasicParsing
    $sums = (Invoke-WebRequest "$base/SHA256SUMS.txt" -UseBasicParsing).Content
    if ($sums -is [byte[]]) { $sums = [Text.Encoding]::UTF8.GetString($sums) }

    $line = ($sums -split "`n") | Where-Object { $_ -match [regex]::Escape($zipName) } | Select-Object -First 1
    if (-not $line) { throw "No checksum for $zipName in SHA256SUMS.txt; not installing." }
    $expected = ($line.Trim() -split '\s+')[0].ToLower()
    $actual   = (Get-FileHash $zip -Algorithm SHA256).Hash.ToLower()
    if ($expected -ne $actual) { throw "Checksum mismatch for $zipName (expected $expected, got $actual); not installing." }
    Write-Host "Checksum OK"

    Expand-Archive $zip -DestinationPath $tmp -Force
    $src = Join-Path $tmp "$name-$tag-$target"
    if (-not (Test-Path (Join-Path $src "$name.exe"))) { $src = $tmp }

    $dest = Join-Path $env:LOCALAPPDATA "Programs\$name"
    New-Item -ItemType Directory -Force -Path $dest | Out-Null
    $keepConfig = Test-Path (Join-Path $dest 'config.toml')
    Copy-Item (Join-Path $src "$name.exe") $dest -Force
    Copy-Item (Join-Path $src 'presets') $dest -Recurse -Force
    foreach ($f in 'README.md', 'LICENSE') {
        if (Test-Path (Join-Path $src $f)) { Copy-Item (Join-Path $src $f) $dest -Force }
    }
    if (-not $keepConfig) { Copy-Item (Join-Path $src 'config.toml') $dest -Force }

    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    if (-not (($userPath -split ';') -contains $dest)) {
        $newPath = if ($userPath) { "$userPath;$dest" } else { $dest }
        [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
        Write-Host "Added $dest to your user PATH (open a new terminal to use it)."
    }

    $startMenu = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs'
    $lnk = Join-Path $startMenu 'Geodesic Wallpaper.lnk'
    try {
        $shell = New-Object -ComObject WScript.Shell
        $sc = $shell.CreateShortcut($lnk)
        $sc.TargetPath = Join-Path $dest "$name.exe"
        $sc.WorkingDirectory = $dest
        $sc.Description = 'Animated geodesic wallpaper'
        $sc.Save()
    } catch {
        Write-Host "Could not create the Start Menu shortcut ($_); the exe still works."
    }

    $ver = & (Join-Path $dest "$name.exe") --version
    Write-Host ""
    Write-Host "Installed $ver to $dest"
    if ($keepConfig) { Write-Host "Kept your existing config.toml." }
    Write-Host ""
    Write-Host "Next:"
    Write-Host "  Start it:     Start Menu > Geodesic Wallpaper, or run  $name"
    Write-Host "  Try a look:   $name --preset ocean   (also cosmic, fire, matrix, neon)"
    Write-Host "  Customize:    edit $dest\config.toml while it runs; it reloads"
    Write-Host "  Stop it:      right-click the tray icon > Quit"
} finally {
    Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
}
