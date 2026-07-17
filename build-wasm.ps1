# Builds the wasm bundle into web/pkg/ using wasm-pack.
# Prerequisites:
#   rustup target add wasm32-unknown-unknown --toolchain stable-x86_64-pc-windows-gnu
#   npm install -g wasm-pack
$ErrorActionPreference = "Stop"
# rustc needs a dlltool on PATH for raw-dylib linking on windows-gnu. The
# toolchain's bundled GNU dlltool can't run standalone (no `as`), so we use
# an LLVM dlltool shim (llvm-ar.exe copied as dlltool.exe — it's a multicall
# binary). Recreate it with:
#   rustup component add llvm-tools --toolchain stable-x86_64-pc-windows-gnu
#   $bin = "$env:USERPROFILE\.rustup\toolchains\stable-x86_64-pc-windows-gnu\lib\rustlib\x86_64-pc-windows-gnu\bin"
#   mkdir "$env:USERPROFILE\.cargo\dlltool-shim"
#   copy "$bin\llvm-ar.exe" "$env:USERPROFILE\.cargo\dlltool-shim\dlltool.exe"
#   copy "$bin\libgcc_s_seh-1.dll","$bin\libwinpthread-1.dll" "$env:USERPROFILE\.cargo\dlltool-shim"
$shim = "$env:USERPROFILE\.cargo\dlltool-shim"
$selfContained = "$env:USERPROFILE\.rustup\toolchains\stable-x86_64-pc-windows-gnu\lib\rustlib\x86_64-pc-windows-gnu\bin\self-contained"
$env:Path = "$env:USERPROFILE\.cargo\bin;$shim;$selfContained;$env:Path"

# --dev is deliberate: release-profile wasm builds stall at runtime with
# the audio stack enabled (see Cargo.toml profile notes). The dev profile
# has deps at opt-level 3 and debuginfo stripped.
wasm-pack build --dev --target web --no-typescript --out-dir web/pkg
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "Done. Serve the web/ folder, e.g.:  python -m http.server 8000 -d web"
