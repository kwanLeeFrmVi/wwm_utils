#[cfg(target_os = "windows")]
fn main() {
    use embed_manifest::{
        embed_manifest,
        manifest::{ActiveCodePage, DpiAwareness, ExecutionLevel, HeapType, Setting},
        new_manifest,
    };

    // Manifest
    embed_manifest(
        new_manifest("exe.manifest")
            .active_code_page(ActiveCodePage::Utf8)
            .dpi_awareness(DpiAwareness::PerMonitorV2)
            .heap_type(HeapType::SegmentHeap)
            .long_path_aware(Setting::Enabled)
            .requested_execution_level(ExecutionLevel::AsInvoker)
            .ui_access(false),
    )
    .expect("unable to embed manifest file");

    // Resource
    winresource::WindowsResource::new()
        /*.set_icon("res/exe.ico")*/
        .set_language(0x0409)
        .set("CompanyName", "dest1yo")
        .set("LegalCopyright", "© 2025 dest1yo")
        .compile()
        .expect("unable to compile Windows resource");

    println!("cargo:rerun-if-changed=build.rs");
}

#[cfg(not(target_os = "windows"))]
fn main() {}
