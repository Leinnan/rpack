#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

#[cfg(all(not(target_arch = "wasm32"), feature = "profiler"))]
fn start_puffin_server() {
    puffin::set_scopes_on(true); // tell puffin to collect data

    match puffin_http::Server::new("127.0.0.1:8585") {
        Ok(puffin_server) => {
            log::info!("Run:  cargo install puffin_viewer && puffin_viewer --url 127.0.0.1:8585");

            std::process::Command::new("puffin_viewer")
                .arg("--url")
                .arg("127.0.0.1:8585")
                .spawn()
                .ok();

            // We can store the server if we want, but in this case we just want
            // it to keep running. Dropping it closes the server, so let's not drop it!
            #[expect(clippy::mem_forget)]
            std::mem::forget(puffin_server);
        }
        Err(err) => {
            log::error!("Failed to start puffin server: {err}");
        }
    }
}

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    #[cfg(all(not(target_arch = "wasm32"), feature = "profiler"))]
    start_puffin_server();
    let file_arg: Option<String> = if std::env::args().len() > 1 {
        std::env::args().nth(1)
    } else {
        None
    };
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 820.0])
            .with_icon(eframe::icon_data::from_png_bytes(rpack_egui::ICON_DATA).unwrap_or_default())
            .with_min_inner_size([720.0, 460.0])
            // macOS: the toolbar and the traffic lights share one unified bar.
            .with_fullsize_content_view(cfg!(target_os = "macos"))
            .with_titlebar_shown(!cfg!(target_os = "macos"))
            .with_title_shown(!cfg!(target_os = "macos")),
        ..Default::default()
    };
    eframe::run_native(
        "rPack",
        native_options,
        Box::new(|cc| Ok(Box::new(rpack_egui::Application::new(cc, file_arg)))),
    )
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {
    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        use web_sys::wasm_bindgen::JsCast;
        use web_sys::{HtmlCanvasElement, window};
        let canvas = window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("the_canvas_id"))
            .expect("No Canvas found")
            .dyn_into::<HtmlCanvasElement>()
            .expect("Could not cast to Canvas");
        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(rpack_egui::Application::new(cc, None)))),
            )
            .await;

        // Remove the loading text, or say why the app did not start.
        let loading_text = window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("loading_text"));
        match (start_result, loading_text) {
            (Ok(_), Some(loading_text)) => loading_text.remove(),
            (Ok(_), None) => {}
            (Err(error), loading_text) => {
                log::error!("Failed to start eframe: {error:?}");
                if let Some(loading_text) = loading_text {
                    loading_text.set_inner_html(
                        "<p>rPack could not start. See the developer console for details.</p>",
                    );
                }
            }
        }
    });
}
