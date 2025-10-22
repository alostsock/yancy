fn main() {
    // Build the C++ bridge code
    cxx_build::bridge("src/raw_processor.rs")
        .file("src/raw_processor.cc")
        .std("c++14")
        .compile("raw_processor");

    println!("cargo:rustc-link-search=native=/usr/lib/x86_64-linux-gnu");
    println!("cargo:rustc-link-lib=static=jpeg");
    println!("cargo:rustc-link-lib=dylib=lcms2");
    println!("cargo:rustc-link-lib=dylib=z");

    println!("cargo:rustc-link-search=native=/usr/local/lib");
    println!("cargo:rustc-link-lib=static=raw");

    // Tell cargo to rerun if the C++ or bridge files change
    println!("cargo:rerun-if-changed=src/raw_processor.rs");
    println!("cargo:rerun-if-changed=src/raw_processor.cc");
}
