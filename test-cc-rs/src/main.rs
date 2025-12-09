fn main() {
    println!("Hello from test-cc-rs!");
    // We don't actually need to call the C++ function from here
    // for the build script to compile the C++ code.
    // The build.rs will compile test.cpp into a static library,
    // and cargo build will attempt to link it.
}