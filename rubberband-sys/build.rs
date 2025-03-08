use std::env;
use std::path::{Path, PathBuf};

fn main() {
    // Path to the Rubberband source directory
    let rubberband_src = Path::new("rubberband-c");

    // Build the single-file version
    let mut build = cc::Build::new();
    build.cpp(true)
        .file(format!("{}/single/RubberBandSingle.cpp", rubberband_src.display()));
    build.flag_if_supported("-std=c++11");

    // On Apple platforms, the single file build would use vDSP for FFT by default.
    // Therefore, we need to link the Accelerate framework.
    if cfg!(target_os = "macos") || cfg!(target_os = "ios") {
        println!("cargo:rustc-link-lib=framework=Accelerate");
    }

    // Compile the library
    build.compile("rubberband");

    // Generate bindings
    let bindings = bindgen::Builder::default()
        .header(format!("{}/rubberband/rubberband-c.h", rubberband_src.display()))
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    // Tell cargo to invalidate the built crate whenever the source files change
    println!("cargo:rerun-if-changed={}/single/RubberBandSingle.cpp", rubberband_src.display());
}
