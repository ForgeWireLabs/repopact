// WI067 Checkpoint E: mirrors repopact-mobile-saf's own build.rs exactly.
// This crate exposes no frontend-invokable commands of its own (see
// src/lib.rs's module doc comment) -- COMMANDS is empty. `android_path`
// still wires this crate's `android/` Gradle module into the generated
// Android project's `tauri.settings.gradle`, which is what actually
// matters here: Kotlin-side native code, invoked only via
// `run_mobile_plugin` calls this crate's own Rust API makes internally.
const COMMANDS: &[&str] = &[];

fn main() {
    let result = tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .try_build();

    // Mirrors tauri-plugin-dialog's own build.rs: documentation builds for
    // the Android target run this build script in a context where the
    // Android Gradle project doesn't exist, and that failure is irrelevant
    // to a docs build.
    if !(cfg!(docsrs) && std::env::var("TARGET").unwrap().contains("android")) {
        result.unwrap();
    }
}
