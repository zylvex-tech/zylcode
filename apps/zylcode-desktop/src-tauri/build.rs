fn main() {
    // A RELEASE build without `custom-protocol` embeds no frontend assets and
    // loads `devUrl` (http://localhost:1420) instead — so the executable shows
    // "localhost refused to connect" even though it compiled cleanly. That is
    // GitHub issue #1, and it is a silent failure: nothing warns you until the
    // app is running and empty.
    //
    // `tauri build` sets the feature. A bare `cargo build --release` does not.
    // Debug builds legitimately omit it (`tauri dev` expects the Vite server),
    // so only the release profile is refused.
    let release = std::env::var("PROFILE").as_deref() == Ok("release");
    let has_feature = std::env::var_os("CARGO_FEATURE_CUSTOM_PROTOCOL").is_some();
    if release && !has_feature {
        panic!(
            "\n\n\
             ZylCode: refusing to finish a RELEASE build without `custom-protocol`.\n\n\
             Without that feature the app loads the Vite dev server instead of the\n\
             embedded frontend, so the resulting executable shows\n\
             \"localhost refused to connect\". Nothing else warns you — the build\n\
             itself succeeds. See GitHub issue #1.\n\n\
             Build it one of these ways instead:\n  \
             cargo build --release -p zylcode-desktop --features custom-protocol\n  \
             pnpm --filter zylcode-desktop tauri build\n\n\
             (Debug builds are unaffected: `tauri dev` wants the dev server.)\n\n"
        );
    }

    tauri_build::build()
}
