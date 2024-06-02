use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rustc-link-search=native=third-party/my283/usr/lib");
    println!("cargo:rustc-link-lib=cam_os_wrapper");
    println!("cargo:rustc-link-lib=mi_sys");
    println!("cargo:rustc-link-lib=static=mi_ao");
    println!("cargo:rerun-if-changed=build.rs");

    let bindings = bindgen::Builder::default()
        .header("../../third-party/my283/usr/include/mi_ao.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
