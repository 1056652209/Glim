use std::{env, path::PathBuf, process::Command};
fn main() {
    println!("cargo:rerun-if-changed=../../assets/macos/Info.plist");
    let metadata =
        std::fs::read_to_string("../../assets/macos/Info.plist").expect("Missing release metadata");
    let build = metadata
        .split("<key>CFBundleVersion</key><string>")
        .nth(1)
        .and_then(|s| s.split("</string>").next())
        .expect("Missing release build number");
    assert!(build.parse::<u64>().is_ok(), "Invalid release build number");
    println!("cargo:rustc-env=GLIM_BUILD_NUMBER={build}");
    println!("cargo:rerun-if-env-changed=GLIM_UPDATE_PUBLIC_KEY");
    assert!(
        env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos"),
        "Glim supports only macOS (Apple Silicon and Intel)"
    );
    build_native_preview();
    if env::var_os("CARGO_FEATURE_MACOS_UPDATER").is_some() {
        build_updater();
    }
}

fn build_native_preview() {
    for source in [
        "Preview.swift",
        "SQLitePreview.swift",
        "EmbeddedPreview.swift",
    ] {
        println!("cargo:rerun-if-changed=../../native/{source}");
    }
    let arch = match env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("aarch64") => "arm64",
        Ok("x86_64") => "x86_64",
        _ => panic!("Unsupported macOS architecture"),
    };
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let status = Command::new("xcrun")
        .args([
            "swiftc",
            "-O",
            "-swift-version",
            "5",
            "-emit-library",
            "-static",
            "-module-name",
            "GlimNativePreview",
            "-target",
            &format!("{arch}-apple-macos11.0"),
            "../../native/Preview.swift",
            "../../native/SQLitePreview.swift",
            "../../native/EmbeddedPreview.swift",
            "-o",
        ])
        .arg(output.join("libGlimNativePreview.a"))
        .status()
        .expect("Native previews require Apple Command Line Tools (swiftc)");
    assert!(status.success(), "Embedded preview compilation failed");
    let compiler = Command::new("xcrun")
        .args(["--find", "swiftc"])
        .output()
        .unwrap();
    assert!(compiler.status.success(), "Cannot locate Swift runtime");
    let compiler = PathBuf::from(String::from_utf8(compiler.stdout).unwrap().trim());
    let runtime = compiler
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("lib/swift/macosx");
    println!("cargo:rustc-link-search=native={}", output.display());
    println!("cargo:rustc-link-search=native={}", runtime.display());
    println!("cargo:rustc-link-search=native=/usr/lib/swift");
    println!("cargo:rustc-link-lib=static=GlimNativePreview");
    println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
}

fn build_updater() {
    println!("cargo:rerun-if-changed=../../native/Updater.swift");
    println!("cargo:rerun-if-env-changed=GLIM_SPARKLE_DIR");
    let framework = PathBuf::from(env::var_os("GLIM_SPARKLE_DIR").expect(
        "Run scripts/fetch-sparkle.sh and set GLIM_SPARKLE_DIR before enabling macos-updater",
    ));
    assert!(
        framework.join("Sparkle.framework/Sparkle").is_file(),
        "Missing Sparkle.framework"
    );
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let arch = match env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("aarch64") => "arm64",
        Ok("x86_64") => "x86_64",
        _ => panic!("Unsupported macOS architecture"),
    };
    let status = Command::new("xcrun")
        .args([
            "swiftc",
            "-O",
            "-swift-version",
            "5",
            "-emit-library",
            "-static",
            "-module-name",
            "GlimUpdater",
            "-target",
            &format!("{arch}-apple-macos11.0"),
            "-F",
        ])
        .arg(&framework)
        .args(["../../native/Updater.swift", "-o"])
        .arg(output.join("libGlimUpdater.a"))
        .status()
        .expect("Swift compiler is required");
    assert!(status.success(), "Updater compilation failed");
    println!("cargo:rustc-link-search=framework={}", framework.display());
    println!("cargo:rustc-link-lib=framework=Sparkle");
    println!("cargo:rustc-link-lib=static=GlimUpdater");
    println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../Frameworks");
    println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../../../../Frameworks");
    // Unbundled test executables find the same pinned framework without installing it.
    if env::var("PROFILE").as_deref() != Ok("release") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", framework.display());
    }
}
