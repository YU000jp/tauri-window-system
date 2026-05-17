param(
  [Parameter(Mandatory = $true)]
  [string]$Version
)

$semverPattern = '^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$'

if ($Version -notmatch $semverPattern) {
  throw "Release version must follow SemVer 2.0.0, for example 0.1.0"
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")

function Update-CargoVersion {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [Parameter(Mandatory = $true)]
    [string]$TargetVersion
  )

  $content = Get-Content $Path -Raw
  $updated = $content -replace '(?m)^version = "([^"]+)"$', "version = `"$TargetVersion`""

  if ($updated -eq $content) {
    throw "Could not update Cargo version in $Path"
  }

  Set-Content -Path $Path -Value $updated
}

function Update-PackageVersion {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [Parameter(Mandatory = $true)]
    [string]$TargetVersion
  )

  $content = Get-Content $Path -Raw
  $updated = $content -replace '(?m)^  "version": "([^"]+)",$', "  `"version`": `"$TargetVersion`","

  if ($updated -eq $content) {
    throw "Could not update package version in $Path"
  }

  Set-Content -Path $Path -Value $updated
}

Update-CargoVersion -Path (Join-Path $repoRoot "crates/tauri-plugin-window-system/Cargo.toml") -TargetVersion $Version
Update-PackageVersion -Path (Join-Path $repoRoot "packages/tauri-plugin-window-system-api/package.json") -TargetVersion $Version
Update-PackageVersion -Path (Join-Path $repoRoot "packages/tauri-window-ui/package.json") -TargetVersion $Version
