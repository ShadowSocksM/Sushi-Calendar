Sushi Calendar v2.5.60 - offline builder update (app code remains v2.5.60)
1. Run build.bat for offline compilation using shared Cargo cache.
2. If a missing package or index metadata error appears, run build_online_once.bat with internet once.
3. Future builds use build.bat. Keep Cargo cache in %USERPROFILE%\.cargo\registry.
4. Cargo.lock is not bundled in this source snapshot. Cargo may need to resolve dependencies again if a compatible lockfile is unavailable. After the first successful build, preserve Cargo.lock between versions to make builds reproducible.
5. The first build of each new folder may still COMPILE dependencies (without downloading) because target/ is per-project. Set CARGO_TARGET_DIR to a common folder to reuse compiled artifacts if desired.
