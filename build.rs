fn main() {
    println!("cargo::rustc-link-arg=-Wl,-pagezero_size,0x4000");
}
