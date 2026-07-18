fn main() {
    println!("cargo:rustc-cdylib-link-arg=/DEF:portal-gear/exports.def");
}