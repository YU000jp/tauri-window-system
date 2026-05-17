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
    [string]$Replacement,
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

  $currentVersion = $versionMatch.Groups[1].Value

  if ($currentVersion -eq $TargetVersion) {
    return
  }

  $updated = [regex]::Replace($content, $Pattern, $Replacement, 1)

  if ($updated -eq $content) {
    throw "Could not update $FieldName version in $Path"
  }

  [System.IO.File]::WriteAllText($Path, $updated, [System.Text.UTF8Encoding]::new($false))
}

Sync-VersionField `
  -Path (Join-Path $repoRoot "crates/tauri-plugin-window-system/Cargo.toml") `
  -Pattern '(?m)^(\s*version\s*=\s*")([^"]+)("\s*)$' `
  -Replacement ('${1}' + $Version + '${3}') `
  -FieldName "Cargo" `
  -TargetVersion $Version

Sync-VersionField `
  -Path (Join-Path $repoRoot "packages/tauri-plugin-window-system-api/package.json") `
  -Pattern '(?m)^(\s*"version"\s*:\s*")([^"]+)(",\s*)$' `
  -Replacement ('${1}' + $Version + '${3}') `
  -FieldName "package" `
  -TargetVersion $Version

Sync-VersionField `
  -Path (Join-Path $repoRoot "packages/tauri-window-ui/package.json") `
  -Pattern '(?m)^(\s*"version"\s*:\s*")([^"]+)(",\s*)$' `
  -Replacement ('${1}' + $Version + '${3}') `
  -FieldName "package" `
  -TargetVersion $Version
