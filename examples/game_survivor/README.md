# Survivor Unity bridge example

The gameplay rules live in `ax/survivor_core.ax`. `build_bridge.ps1` regenerates
the C# bridge and Rust core, runs the native runtime tests, builds
`axiom_survivor.dll`, and copies it into Unity's `Assets/Plugins/x86_64` folder.

```powershell
powershell -ExecutionPolicy Bypass -File examples/game_survivor/build_bridge.ps1
dotnet run --project examples/game_survivor/Tests/Standalone/SurvivorRuntimeTests.csproj
```

In Unity, place `AxiomRunner` and `SurvivorGameController` on the same object.
Assign prefabs in the generated `AxiomAssetRegistry`, optional sprite/animation
and visual-event bindings on the controller, and the health/level/score HUD
references. `SurvivorGameController` subscribes to the runner's
`OnEntityUpdated` and `OnVisualEvent` streams and owns the entity view lifecycle.
