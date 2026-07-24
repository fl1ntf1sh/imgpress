fn main() {
    let _ = embed_resource::compile("assets/icon.rc", embed_resource::NONE);
    slint_build::compile("src/ui/app.slint").expect("failed to compile Slint UI");
}
