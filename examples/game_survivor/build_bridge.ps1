$ErrorActionPreference = "Stop"

$exampleRoot = $PSScriptRoot
$repositoryRoot = (Resolve-Path (Join-Path $exampleRoot "../..")).Path
$axiomSource = "examples/game_survivor/ax/survivor_core.ax"
$bridgeOutput = "examples/game_survivor/Assets/Scripts/AxiomBridge"
$generatedRust = "examples/game_survivor/native/src/generated_core.rs"
$pluginDirectory = "examples/game_survivor/Assets/Plugins/x86_64"

Push-Location $repositoryRoot
try {
    cargo run -p axiom-cli -- bridge $axiomSource --target unity --out $bridgeOutput `
        --library-name axiom_survivor
    cargo run -p axiom-cli -- emit-rust $axiomSource --output $generatedRust
    cargo fmt -p axiom-survivor-native
    cargo test -p axiom-survivor-native
    cargo build -p axiom-survivor-native --release

    New-Item -ItemType Directory -Force $pluginDirectory | Out-Null
    try {
        Copy-Item -LiteralPath "target/release/axiom_survivor.dll" `
            -Destination "$pluginDirectory/axiom_survivor.dll" -Force -ErrorAction Stop
        Write-Host "Unity bridge and native plugin are ready in examples/game_survivor/Assets."
    } catch {
        Write-Warning "axiom_survivor.dll is currently loaded and locked by Unity Editor. Please close Unity Play mode or restart Unity Editor, then re-run build_bridge.ps1."
    }
}
finally {
    Pop-Location
}
