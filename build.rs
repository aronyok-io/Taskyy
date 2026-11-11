fn main() {
    let config = slint_build::CompilerConfiguration::new().with_style("fluent".into()).embed_resources(slint_build::EmbedResourcesKind::EmbedFiles);
    slint_build::compile_with_config("ui/main.slint", config).expect("compile Taskyy interface");
}
