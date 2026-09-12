# WI062 Discovered-Gap Baseline

Recorded before any packaging work began, from Jeremy's direct post-WI060
inspection of the Dell Precision.

## Windows

WI060 rebuilt the native desktop binary (`cargo build` / `cargo test`
producing `target/debug/repopact-desktop.exe`) and used that binary directly
for its own regression evidence. It never built or installed an NSIS/MSI
package, and therefore never proved:

- an installed application location outside `target/debug` or `target/release`;
- a Start-menu entry;
- a Desktop shortcut;
- Add/Remove Programs (uninstall) registration;
- uninstall behavior;
- launch from an installed shortcut rather than a raw EXE path.

Jeremy confirmed this directly: there is no normal RepoPact launcher or
shortcut visible on Windows.

## Android

WI060 installed and launched the app via `adb install` / `adb shell am
start`, and proved real Rust-backed runtime behavior once running. It did
not separately prove operator-visible launcher discovery — every launch in
WI060 was adb-driven, not tap-driven from the app drawer.

The committed `AndroidManifest.xml` already declares:

```xml
<action android:name="android.intent.action.MAIN" />
<category android:name="android.intent.category.LAUNCHER" />
```

and:

```xml
android:icon="@mipmap/ic_launcher"
```

so Android should expose a normal launcher entry — but a correct manifest is
not the same claim as operator-confirmed discoverability, and must not be
treated as equivalent to it.

## Linux

The Debian 13 WSL2 environment was used for Rust CLI validation in WI059
(`cargo test`, `python -m unittest`, conformance). The Tauri GUI package
(`.deb`, desktop integration, WebKitGTK runtime) was never built, installed,
or launched there. Linux desktop integration is entirely unproven.

## What this means for WI062's scope

1. Do not claim any platform's launcher/installer story is proven by a
   successful build alone. A build is necessary, not sufficient.
2. Windows and Linux require a real installed package (NSIS `.exe` / `.deb`),
   not the raw `target/debug` or `target/release` binary, as the install
   proof.
3. Android's manifest already supports launcher discovery; WI062's job there
   is to *prove* it (package-manager query, app-drawer screenshot, tap-to-
   launch, reboot persistence) rather than re-declare it.
4. Every platform's closeout criterion that depends on visual/tactile
   confirmation (Start-menu launch, Desktop-shortcut launch, app-drawer
   launch, WSLg window appearance) requires Jeremy's own confirmation. An
   agent clicking a shortcut via automation is a useful sanity check but is
   explicitly not a substitute for that confirmation per this item's
   acceptance criteria.
