//! Background work: a thread on native, the main thread's task queue on the web.

#[cfg(not(target_arch = "wasm32"))]
pub fn spawn(job: impl FnOnce() + Send + 'static) {
    std::thread::spawn(job);
}

#[cfg(target_arch = "wasm32")]
pub fn spawn(job: impl FnOnce() + 'static) {
    wasm_bindgen_futures::spawn_local(async move { job() });
}

/// For file dialogs, whose async API is the one shared with the web.
#[cfg(not(target_arch = "wasm32"))]
pub fn spawn_async(future: impl Future<Output = ()> + Send + 'static) {
    std::thread::spawn(move || futures::executor::block_on(future));
}

#[cfg(target_arch = "wasm32")]
pub fn spawn_async(future: impl Future<Output = ()> + 'static) {
    wasm_bindgen_futures::spawn_local(future);
}
