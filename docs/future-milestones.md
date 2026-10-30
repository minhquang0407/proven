# Planned Enhancements

## 1. File Watcher (`proven watch`)
- Keep the indexed `RepoGraph` resident across file saves and re-parse only modified files incrementally.

## 2. Additional Build & Test Runner Presets
- Add built-in test runner detection for Maven (`mvn test`), Gradle (`gradle test`), .NET (`dotnet test`), and CMake (`ctest`) alongside `pytest`, `cargo test`, `go test`, and `node` / `jest` / `vitest`.

## 3. GitHub Code Scanning (SARIF) Output
- Emit SARIF v2.1.0 reports from `proven scan --sarif` and `proven verify-proof --sarif` for inline Pull Request annotations.
