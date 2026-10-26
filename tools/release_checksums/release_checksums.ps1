# Release checksum generation for XCX (4.4.1) — Windows-native,
# no installation required (PowerShell 5.1 is built into Windows).
#
# Collects release artifacts and writes two coreutils-format checksum
# sheets — `checksums.sha256` and `checksums.sha512` — so verification is
# a stock command with no warnings on every platform:
#
#     sha256sum -c checksums.sha256      (Linux / FreeBSD)
#     shasum -a 256 -c checksums.sha256  (macOS)
#     Get-FileHash <file> -Algorithm SHA256  (Windows; compare per file)
#
# Each sheet is meant to be signed once (minisign) — the signature then
# covers every artifact listed in it.
#
# Usage:
#   powershell -NoProfile -File tools\release_checksums\release_checksums.ps1 `
#       -Artifacts DIR [DIR ...] [-Out DIR] [-PatchIss FILE]
#
# -PatchIss fills the installer's embedded SHA-256 defines
# (`ChecksumXcxExe` -> `xcx.exe`, matched case/punctuation-insensitively,
# an `Exe` suffix on the define name is ignored) BEFORE the installer is
# built. Sheets, `checksums.txt` leftovers and `*.minisig` files are
# always skipped, so re-runs are safe.

param(
    [Parameter(Mandatory = $true)][string[]]$Artifacts,
    [string]$Out = ".",
    [string]$PatchIss = ""
)

$ErrorActionPreference = 'Stop'
$SkipNames = @('checksums.sha256', 'checksums.sha512', 'checksums.txt')
$SkipSuffixes = @('.minisig')

function Get-Hash([string]$path, [string]$algo) {
    return (Get-FileHash -LiteralPath $path -Algorithm $algo).Hash.ToLower()
}

function Test-Skipped([string]$name) {
    if ($SkipNames -contains $name) { return $true }
    foreach ($s in $SkipSuffixes) { if ($name.EndsWith($s)) { return $true } }
    return $false
}

$files = @()
foreach ($root in $Artifacts) {
    if (Test-Path $root -PathType Leaf) { $files += (Resolve-Path $root).Path; continue }
    if (-not (Test-Path $root)) { Write-Output ("root not found: {0}" -f $root); continue }
    $rootFull = (Resolve-Path $root).Path.TrimEnd('\') + '\'
    $all = @(Get-ChildItem -LiteralPath $rootFull -Recurse -File | ForEach-Object { $_.FullName } | Sort-Object)
    foreach ($f in $all) {
        $name = [IO.Path]::GetFileName($f)
        if (-not (Test-Skipped $name)) { $files += $f }
    }
}
if ($PatchIss -ne "") {
    $drop = (Resolve-Path $PatchIss).Path
    $files = @($files | Where-Object { $_ -ne $drop })
}
if ($files.Count -eq 0) { Write-Output 'no artifacts found'; exit 1 }

function Get-NormStem([string]$p) {
    $stem = [IO.Path]::GetFileNameWithoutExtension($p)
    return ($stem.ToLower() -replace '[^a-z0-9]', '')
}

function Write-Sheet([string]$algo, [string]$path) {
    $lines = @()
    foreach ($f in $files) {
        $lines += ("{0}  {1}" -f (Get-Hash $f $algo), [IO.Path]::GetFileName($f))
    }
    [System.IO.File]::WriteAllText($path, ($lines -join "`n") + "`n", (New-Object System.Text.UTF8Encoding($false)))
}

if ($PatchIss -ne "") {
    Write-Output ("patching {0}:" -f $PatchIss)
    $issText = [System.IO.File]::ReadAllText($PatchIss)
    $defines = [regex]::Matches($issText, '#define\s+(Checksum\w+)\s*"[0-9A-Fa-f]*"') |
               ForEach-Object { $_.Groups[1].Value } | Sort-Object -Unique
    $patched = 0
    foreach ($d in $defines) {
        $base = $d.Substring('Checksum'.Length)
        $cands = @($base)
        if ($base.ToLower().EndsWith('exe')) { $cands = @($base.Substring(0, $base.Length - 3)) + $cands }
        $hit = $null
        foreach ($cand in $cands) {
            $cn = ($cand.ToLower() -replace '[^a-z0-9]', '')
            foreach ($f in $files) {
                if ((Get-NormStem $f) -eq $cn) { $hit = $f; break }
            }
            if ($hit) { break }
        }
        if (-not $hit) {
            Write-Output ("  {0} SKIPPED (no artifact matches '{1}')" -f $d, $base)
            continue
        }
        $hex = (Get-Hash $hit 'SHA256').ToUpper()
        $issText = [regex]::Replace($issText,
            ('(#define\s+' + $d + '\s*)"[0-9A-Fa-f]*"'),
            { param($m) $m.Groups[1].Value + '"' + $hex + '"' })
        Write-Output ("  {0} <- {1} ({2}...)" -f $d, [IO.Path]::GetFileName($hit), $hex.Substring(0, 16))
        $patched++
    }
    if ($patched -gt 0) {
        [System.IO.File]::WriteAllText($PatchIss, $issText, (New-Object System.Text.UTF8Encoding($false)))
    }
}

$sha256Path = [IO.Path]::Combine($Out, 'checksums.sha256')
$sha512Path = [IO.Path]::Combine($Out, 'checksums.sha512')
Write-Sheet 'SHA256' $sha256Path
Write-Sheet 'SHA512' $sha512Path
Write-Output ("wrote 2 sheets ({0} files x 2 algorithms)" -f $files.Count)
foreach ($f in $files) { Write-Output ("  {0}" -f [IO.Path]::GetFileName($f)) }
Write-Output "next: sign every sheet ->"
Write-Output ("  minisign -Sm {0} -s <secret key>" -f [IO.Path]::GetFileName($sha256Path))
Write-Output ("  minisign -Sm {0} -s <secret key>" -f [IO.Path]::GetFileName($sha512Path))
exit 0
