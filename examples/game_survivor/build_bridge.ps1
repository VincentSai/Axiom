$ErrorActionPreference = "Stop"

$exampleRoot = $PSScriptRoot
$repositoryRoot = (Resolve-Path (Join-Path $exampleRoot "../..")).Path
$axiomSource = "examples/game_survivor/ax/survivor_core.ax"
$bridgeOutput = "examples/game_survivor/Assets/Scripts/AxiomBridge"
$generatedRust = "examples/game_survivor/native/src/generated_core.rs"
$pluginDirectory = "examples/game_survivor/Assets/Plugins/x86_64"

Push-Location $repositoryRoot
try {
    cargo run -p axiom-cli -- bridge $axiomSource --target unity --out $bridgeOutput
    cargo run -p axiom-cli -- emit-rust $axiomSource --output $generatedRust
    cargo fmt -p axiom-survivor-native
    cargo test -p axiom-survivor-native
    cargo build -p axiom-survivor-native --release

    New-Item -ItemType Directory -Force $pluginDirectory | Out-Null
    Copy-Item -LiteralPath "target/release/axiom_survivor.dll" `
        -Destination "$pluginDirectory/axiom_survivor.dll" -Force
    Write-Host "Unity bridge and native plugin are ready in examples/game_survivor/Assets."
}
finally {
    Pop-Location
}
