# EN: Package the native MSVC binary with provenance; reject pre-existing outputs.
# JA: MSVCネイティブのバイナリを検証して梱包し、既存の成果物は上書きしません。
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
Set-Location $root
$version = '0.0.31.0'
$binary = Join-Path $root 'target/release/ghcap.exe'
$dist = Join-Path $root 'dist'
if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) { throw "Missing binary / 実行ファイルがありません: $binary" }
$result = & $binary --version
if ($LASTEXITCODE -ne 0 -or $result.Trim() -ne "ghcap v$version") { throw 'Binary version mismatch / バージョンが一致しません' }
& $binary --help > $null
if ($LASTEXITCODE -ne 0) { throw 'Binary help failed / ヘルプの起動に失敗' }
New-Item -ItemType Directory -Force -Path $dist | Out-Null
$exe = Join-Path $dist "ghcap-v$version-windows-x64.exe"
$archive = Join-Path $dist "ghcap-v$version-windows-x64.zip"
if ((Test-Path $exe) -or (Test-Path $archive)) { throw 'Refusing to overwrite outputs / 既存配布物の上書きを拒否します' }
$sha = (& git rev-parse --verify HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $sha -notmatch '^[a-fA-F0-9]{40}$') { throw 'Git commit lookup failed / Gitのコミット取得に失敗' }
$stage = Join-Path ([System.IO.Path]::GetTempPath()) ([System.Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $stage | Out-Null
try {
    Copy-Item -LiteralPath $binary -Destination (Join-Path $stage 'ghcap.exe')
    foreach ($doc in @('README.md', 'LICENSE', 'DEPENDENCIES.md', 'THIRD_PARTY_NOTICES.md')) {
        Copy-Item -LiteralPath (Join-Path $root $doc) -Destination (Join-Path $stage $doc)
    }
    Copy-Item -LiteralPath (Join-Path $root 'THIRD_PARTY_LICENSES') -Destination $stage -Recurse
    [System.IO.File]::WriteAllText((Join-Path $stage 'SOURCE_COMMIT.txt'), "$sha`n")
    $lockSha = (Get-FileHash -LiteralPath (Join-Path $root 'Cargo.lock') -Algorithm SHA256).Hash.ToLowerInvariant()
    $rust = (& rustc -Vv) -join "`n"
    if ($LASTEXITCODE -ne 0) { throw 'Rust toolchain inspection failed / Rust情報の取得に失敗' }
    [System.IO.File]::WriteAllText((Join-Path $stage 'BUILD_INFO.txt'), "$rust`nCargo.lock SHA-256: $lockSha`n")
    [System.IO.File]::WriteAllText((Join-Path $stage 'INSTALL.txt'), "Run ghcap.exe in a terminal. Git and compatible gh are required for related operations.`nターミナルからghcap.exeを起動してください。GitHub操作には対応版ghが必要です。`n")
    Copy-Item -LiteralPath $binary -Destination $exe
    Compress-Archive -Path (Join-Path $stage '*') -DestinationPath $archive -CompressionLevel Optimal
    $verify = Join-Path $stage '_verify'
    New-Item -ItemType Directory -Path $verify | Out-Null
    Expand-Archive -LiteralPath $archive -DestinationPath $verify
    $verified = & (Join-Path $verify 'ghcap.exe') --version
    if ($LASTEXITCODE -ne 0 -or $verified.Trim() -ne "ghcap v$version") { throw 'ZIP executable verification failed / ZIP内部の実行ファイルが異常' }
    $insideCommit = (Get-Content -LiteralPath (Join-Path $verify 'SOURCE_COMMIT.txt') -Raw).Trim()
    if ($insideCommit -ne $sha) { throw 'ZIP source commit mismatch / ZIPのソース履歴が異なります' }
    $insideHash = (Get-FileHash (Join-Path $verify 'ghcap.exe') -Algorithm SHA256).Hash
    $exeHash = (Get-FileHash $exe -Algorithm SHA256).Hash
    if ($insideHash -ne $exeHash) { throw 'ZIP binary differs / ZIPと実行ファイルが異なります' }
    foreach ($file in @($exe, $archive)) {
        $digest = (Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash.ToLowerInvariant()
        [System.IO.File]::WriteAllText("$file.sha256", "$digest  $([System.IO.Path]::GetFileName($file))`n")
    }
} finally {
    Remove-Item -LiteralPath $stage -Recurse -Force -ErrorAction SilentlyContinue
}
Write-Host 'Windows packages verified / Windows配布物の検証完了'
