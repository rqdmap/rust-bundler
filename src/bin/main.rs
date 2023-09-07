use rust_bundler::Bundler;

fn main() {
    let mut bundler = Bundler::new("rust_bundler", "src/bin/main.rs", "main-bundle.rs");
    bundler.run();
}
