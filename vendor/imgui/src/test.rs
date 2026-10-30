use parking_lot::{ReentrantMutex, ReentrantMutexGuard};
use std::ptr;

use crate::context::Context;

pub static TEST_MUTEX: ReentrantMutex<()> = parking_lot::const_reentrant_mutex(());

pub fn test_ctx() -> (ReentrantMutexGuard<'static, ()>, Context) {
    let guard = TEST_MUTEX.lock();
    let mut ctx = Context::create();
    ctx.io_mut().ini_filename = ptr::null();
    (guard, ctx)
}

pub fn test_ctx_initialized() -> (ReentrantMutexGuard<'static, ()>, Context) {
    let (guard, mut ctx) = test_ctx();
    let io = ctx.io_mut();
    io.display_size = [1024.0, 768.0];
    io.delta_time = 1.0 / 60.0;
    io.mouse_pos = [0.0, 0.0];
    io.backend_flags
        .insert(crate::io::BackendFlags::RENDERER_HAS_TEXTURES);
    if ctx.fonts().is_built() {
        ctx.fonts().build_rgba32_texture();
    }
    (guard, ctx)
}
