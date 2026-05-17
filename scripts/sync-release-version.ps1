param(
  [Parameter(Mandatory = $true)]
  [string]$Version
)

$semverPattern = '^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$'

if ($Version -notmatch $semverPattern) {
  throw "Release version must follow SemVer 2.0.0, for example 0.1.0"
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")

function Sync-VersionField {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [Parameter(Mandatory = $true)]
    [string]$Pattern,
    [Parameter(Mandatory = $true)]
    [string]$FieldName,
    [Parameter(Mandatory = $true)]
    [string]$TargetVersion
  )

  $content = Get-Content -LiteralPath $Path -Raw
  $versionMatch = [regex]::Match($content, $Pattern)

  if (-not $versionMatch.Success) {
    throw "Could not find a version field in $Path"
  }

  $currentVersion = $versionMatch.Groups[2].Value

  if ($currentVersion -eq $TargetVersion) {
    return
  }

  # Rebuild the matched line directly so replacement syntax cannot misread SemVer digits.
  $updated = $content.Substring(0, $versionMatch.Index) +
    $versionMatch.Groups[1].Value +
    $TargetVersion +
    $versionMatch.Groups[3].Value +
    $content.Substring($versionMatch.Index + $versionMatch.Length)

  if ($updated -eq $content) {
    throw "Could not update $FieldName version in $Path"
  }

  [System.IO.File]::WriteAllText($Path, $updated, [System.Text.UTF8Encoding]::new($false))
}

Sync-VersionField `
  -Path (Join-Path $repoRoot "crates/tauri-plugin-window-system/Cargo.toml") `
  -Pattern '(?m)^(\s*version\s*=\s*")([^"]+)("\s*)$' `
  -FieldName "Cargo" `
  -TargetVersion $Version

Sync-VersionField `
  -Path (Join-Path $repoRoot "Cargo.lock") `
  -Pattern '(?ms)(\[\[package\]\]\r?\nname = "tauri-plugin-window-system"\r?\nversion = ")([^"]+)(")' `
  -FieldName "Cargo.lock" `
  -TargetVersion $Version

Sync-VersionField `
  -Path (Join-Path $repoRoot "packages/tauri-plugin-window-system-api/package.json") `
  -Pattern '(?m)^(\s*"version"\s*:\s*")([^"]+)(",\s*)$' `
  -FieldName "package" `
  -TargetVersion $Version

Sync-VersionField `
  -Path (Join-Path $repoRoot "packages/tauri-window-ui/package.json") `
  -Pattern '(?m)^(\s*"version"\s*:\s*")([^"]+)(",\s*)$' `
  -FieldName "package" `
  -TargetVersion $Version
