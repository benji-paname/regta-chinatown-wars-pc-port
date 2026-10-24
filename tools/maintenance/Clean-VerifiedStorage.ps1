param([switch]$Apply)
$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../..')).Path
$reportRoot = Join-Path $repoRoot 'work/restructure'
$plan = Get-Content -LiteralPath (Join-Path $reportRoot 'cleanup-plan.json') -Raw | ConvertFrom-Json
$cacheNames = @('work/ctw-build', 'work/ctw-roll-build', 'work/ctw-slice-build')

function Resolve-SafeLocalPath([string]$relative) {
    $full = [IO.Path]::GetFullPath((Join-Path $repoRoot $relative))
    if (-not $full.StartsWith($repoRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Path outside workspace: $relative"
    }
    $item = Get-Item -LiteralPath $full -Force
    $cursor = $item
    while ($cursor -and $cursor.FullName -ne $repoRoot) {
        if ($cursor.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "Reparse point: $($cursor.FullName)" }
        if ($cursor -is [IO.DirectoryInfo]) { $cursor = $cursor.Parent } else { $cursor = $cursor.Directory }
    }
    return $item.FullName
}

$deletions = @()
foreach ($entry in $plan.caches) {
    if ($entry.path -notin $cacheNames) { throw 'Unapproved cache in plan' }
    $path = Resolve-SafeLocalPath $entry.path
    if ($path -ne [IO.Path]::GetFullPath((Join-Path $repoRoot $entry.path))) { throw 'Unexpected resolved cache path' }
    $tag = Get-Content -LiteralPath (Join-Path $path 'CACHEDIR.TAG') -Raw
    if ($tag -notmatch 'Signature: 8a477f597d28d172789f06886806bc55') { throw 'Not a recognized regenerable cache' }
    if (-not (Test-Path -LiteralPath (Join-Path $path '.rustc_info.json'))) { throw 'Missing Cargo cache metadata' }
    $children = @(Get-ChildItem -LiteralPath $path -Recurse -Force)
    if ($children | Where-Object { $_.Attributes -band [IO.FileAttributes]::ReparsePoint }) { throw 'Cache contains a reparse point' }
    $size = ($children | Where-Object { -not $_.PSIsContainer } | Measure-Object -Property Length -Sum).Sum
    if ($size -ne $entry.bytes) { throw 'Cache changed since inventory; recreate the cleanup plan' }
    $deletions += [pscustomobject]@{path=$path; relative=$entry.path; bytes=$size; directory=$true}
}
foreach ($entry in $plan.identical_images) {
    if ($entry.bmp -notmatch '^(work|outputs)/.+\.bmp$' -or
        $entry.png -ne [IO.Path]::ChangeExtension($entry.bmp, '.png')) { throw 'Unexpected image pair' }
    $bmp = Resolve-SafeLocalPath $entry.bmp
    $png = Resolve-SafeLocalPath $entry.png
    if ((Get-FileHash -LiteralPath $bmp -Algorithm SHA256).Hash -ne $entry.bmp_sha256 -or
        (Get-FileHash -LiteralPath $png -Algorithm SHA256).Hash -ne $entry.png_sha256) {
        throw 'Image pair changed since exact pixel comparison; recreate the cleanup plan'
    }
    $deletions += [pscustomobject]@{path=$bmp; relative=$entry.bmp; bytes=$entry.bytes; directory=$false}
}
Write-Host "Verified $($deletions.Count) targets; $((($deletions | Measure-Object bytes -Sum).Sum)) bytes."
if (-not $Apply) { Write-Host 'Dry run only. Pass -Apply to perform this verified plan.'; return }

$completed = @()
foreach ($entry in $deletions) {
    # All final absolute paths were checked above; one shell performs the removals.
    if ($entry.directory) { Remove-Item -LiteralPath $entry.path -Recurse -Force }
    else { Remove-Item -LiteralPath $entry.path -Force }
    $completed += $entry.relative
    $result = @{timestamp=(Get-Date -Format 'o'); removed=$completed; removed_bytes=(($deletions | Where-Object { $_.relative -in $completed } | Measure-Object bytes -Sum).Sum)}
    $result | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $reportRoot 'cleanup-result.json') -Encoding UTF8
}
