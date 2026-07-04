use super::*;
use glium::backend::Backend;
use std::ffi::CStr;
use std::io::Error as IoError;
use std::os::raw::c_void;
use std::ptr::{null, null_mut};
use winapi::shared::windef::*;
use winapi::um::libloaderapi::{GetModuleHandleW, *};
use winapi::um::wingdi::*;
use winapi::um::winuser::*;

pub mod ffi {
    include!(concat!(env!("OUT_DIR"), "/wgl_bindings.rs"));
}
pub mod ffiextra {
    include!(concat!(env!("OUT_DIR"), "/wgl_extra_bindings.rs"));
}

struct WglWrapper {
    lib: libloading::Library,
    wgl: ffi::Wgl,
    ext: Option<ffiextra::Wgl>,
}

type GetProcAddressFunc =
    unsafe extern "system" fn(*const std::os::raw::c_char) -> *const std::os::raw::c_void;

impl Drop for WglWrapper {
    fn drop(&mut self) {
        log::trace!("dropping WglWrapper and libloading {:?}", self.lib);
    }
}

impl WglWrapper {
    fn load() -> anyhow::Result<Self> {
        let class_name = wide_string("wezterm wgl extension probing window");
        let h_inst = unsafe { GetModuleHandleW(null()) };
        let class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW | CS_OWNDC,
            lpfnWndProc: Some(DefWindowProcW),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: h_inst,
            hIcon: null_mut(),
            hCursor: null_mut(),
            hbrBackground: null_mut(),
            lpszMenuName: null(),
            lpszClassName: class_name.as_ptr(),
        };

        if unsafe { RegisterClassW(&class) } == 0 {
            let err = IoError::last_os_error();
            match err.raw_os_error() {
                Some(code)
                    if code == winapi::shared::winerror::ERROR_CLASS_ALREADY_EXISTS as i32 => {}
                _ => return Err(err.into()),
            }
        }

        let hwnd = unsafe {
            CreateWindowExW(
                0,
                class_name.as_ptr(),
                class_name.as_ptr(),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                1024,
                768,
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
            )
        };
        if hwnd.is_null() {
            let err = IoError::last_os_error();
            anyhow::bail!("CreateWindowExW: {}", err);
        }

        let mut state = GlState::create_basic(WglWrapper::create()?, hwnd)?;

        unsafe {
            state.make_current();
        }

        let _ = state.wgl.as_mut().unwrap().load_ext();

        state.make_not_current();

        Ok(state.into_wrapper())
    }

    fn create() -> anyhow::Result<Self> {
        if crate::configuration::prefer_swrast() {
            let mesa_dir = std::env::current_exe()
                .unwrap()
                .parent()
                .unwrap()
                .join("mesa");
            let mesa_dir = wide_string(mesa_dir.to_str().unwrap());

            unsafe {
                AddDllDirectory(mesa_dir.as_ptr());
                SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_DEFAULT_DIRS);
            }
        }

        let lib = unsafe { libloading::Library::new("opengl32.dll") }.map_err(|e| {
            log::error!("{:?}", e);
            e
        })?;
        log::trace!("loaded {:?}", lib);

        let get_proc_address: libloading::Symbol<GetProcAddressFunc> =
            unsafe { lib.get(b"wglGetProcAddress\0")? };
        let wgl = ffi::Wgl::load_with(|s: &'static str| {
            let sym_name = std::ffi::CString::new(s).expect("symbol to be cstring compatible");
            if let Ok(sym) = unsafe { lib.get(sym_name.as_bytes_with_nul()) } {
                return *sym;
            }
            unsafe { get_proc_address(sym_name.as_ptr()) }
        });
        Ok(Self {
            lib,
            wgl,
            ext: None,
        })
    }

    fn load_ext(&mut self) -> anyhow::Result<()> {
        let get_proc_address: libloading::Symbol<GetProcAddressFunc> =
            unsafe { self.lib.get(b"wglGetProcAddress\0")? };

        self.ext
            .replace(ffiextra::Wgl::load_with(|s: &'static str| {
                let sym_name = std::ffi::CString::new(s).expect("symbol to be cstring compatible");
                if let Ok(sym) = unsafe { self.lib.get(sym_name.as_bytes_with_nul()) } {
                    return *sym;
                }
                unsafe { get_proc_address(sym_name.as_ptr()) }
            }));

        Ok(())
    }
}

pub struct GlState {
    wgl: Option<WglWrapper>,
    hdc: HDC,
    rc: ffi::types::HGLRC,
}

/// A GL context fully created on a helper thread against a hidden probing
/// window, ready to be re-targeted at the first real window's DC. Producing
/// it ahead of time moves the expensive driver initialization + context
/// creation (hundreds of ms on a cold driver) off the startup critical path.
///
/// The probing window and its DC are torn down on the helper thread before
/// handover (windows die with their creating thread anyway), so only the
/// thread-independent pieces travel: the loaded library, the context handle
/// and the pixel format needed to make a compatible DC.
struct PrewarmedGl {
    wgl: WglWrapper,
    rc: ffi::types::HGLRC,
    format_id: i32,
    pfd: PIXELFORMATDESCRIPTOR,
}

// Raw handles make this !Send automatically, but the handover is safe:
// the context is made not-current on the helper thread before it is sent,
// and WGL contexts/libraries are process-scoped, not thread-scoped.
unsafe impl Send for PrewarmedGl {}

lazy_static::lazy_static! {
    static ref PREWARM_RX: std::sync::Mutex<Option<std::sync::mpsc::Receiver<Option<PrewarmedGl>>>> =
        std::sync::Mutex::new(None);
}

/// Kick off GL driver + context creation on a helper thread. Call as early
/// as possible; the first `GlState::create` will adopt the result (waiting
/// for it if it is still in flight, which is still much cheaper than racing
/// the driver's process-wide init locks with a second creation).
pub fn start_prewarm() {
    {
        let mut slot = match PREWARM_RX.lock() {
            Ok(slot) => slot,
            Err(_) => return,
        };
        if slot.is_some() {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        slot.replace(rx);
        let spawned = std::thread::Builder::new()
            .name("wgl-prewarm".to_string())
            .spawn(move || {
                let start = std::time::Instant::now();
                let result = prewarm_context();
                match &result {
                    Ok(_) => log::info!("wgl prewarm: context ready in {:?}", start.elapsed()),
                    Err(err) => log::debug!("wgl prewarm failed: {err:#}"),
                }
                tx.send(result.ok()).ok();
            });
        if spawned.is_err() {
            slot.take();
        }
    }
}

fn prewarm_context() -> anyhow::Result<PrewarmedGl> {
    let class_name = wide_string("unterm wgl prewarm window");
    let h_inst = unsafe { GetModuleHandleW(null()) };
    let class = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW | CS_OWNDC,
        lpfnWndProc: Some(DefWindowProcW),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: h_inst,
        hIcon: null_mut(),
        hCursor: null_mut(),
        hbrBackground: null_mut(),
        lpszMenuName: null(),
        lpszClassName: class_name.as_ptr(),
    };

    if unsafe { RegisterClassW(&class) } == 0 {
        let err = IoError::last_os_error();
        match err.raw_os_error() {
            Some(code)
                if code == winapi::shared::winerror::ERROR_CLASS_ALREADY_EXISTS as i32 => {}
            _ => return Err(err.into()),
        }
    }

    let hwnd = unsafe {
        CreateWindowExW(
            0,
            class_name.as_ptr(),
            class_name.as_ptr(),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            16,
            16,
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
        )
    };
    if hwnd.is_null() {
        anyhow::bail!("CreateWindowExW: {}", IoError::last_os_error());
    }

    let mut state = GlState::create_impl(hwnd)?;
    state.make_not_current();

    // Capture the pixel format while the probing DC is still alive; the
    // real window will replicate it so the context can drive its DC.
    let format_id = unsafe { GetPixelFormat(state.hdc) };
    let mut pfd: PIXELFORMATDESCRIPTOR = unsafe { std::mem::zeroed() };
    let described = unsafe {
        DescribePixelFormat(
            state.hdc,
            format_id,
            std::mem::size_of::<PIXELFORMATDESCRIPTOR>() as _,
            &mut pfd,
        )
    };

    // Take the library/context out of the GlState so that dropping the
    // husk doesn't DeleteContext (delete() is a no-op once wgl is None).
    let wgl = state
        .wgl
        .take()
        .ok_or_else(|| anyhow::anyhow!("wgl vanished"))?;
    let rc = state.rc;
    unsafe {
        ReleaseDC(hwnd, state.hdc);
        DestroyWindow(hwnd);
    }
    drop(state);

    if format_id == 0 || described == 0 {
        // Clean up the orphaned context rather than leaking it.
        unsafe {
            wgl.wgl.DeleteContext(rc);
        }
        anyhow::bail!("could not capture prewarmed pixel format");
    }

    Ok(PrewarmedGl {
        wgl,
        rc,
        format_id,
        pfd,
    })
}

fn take_prewarmed() -> Option<PrewarmedGl> {
    let rx = PREWARM_RX.lock().ok()?.take()?;
    // Normally the prewarm finished long ago; if it is still mid-flight,
    // waiting beats racing it for the driver's process-wide init locks.
    match rx.recv_timeout(std::time::Duration::from_secs(3)) {
        Ok(result) => result,
        Err(_) => None,
    }
}

fn has_extension(extensions: &str, wanted: &str) -> bool {
    extensions.split(' ').find(|&ext| ext == wanted).is_some()
}

impl GlState {
    fn into_wrapper(mut self) -> WglWrapper {
        self.delete();
        self.wgl.take().unwrap()
    }

    pub fn create(window: HWND) -> anyhow::Result<Self> {
        if let Some(pre) = take_prewarmed() {
            let timing = std::time::Instant::now();
            match Self::adopt_prewarmed(pre, window) {
                Ok(state) => {
                    log::debug!("wgl-timing: adopted prewarmed context {:?}", timing.elapsed());
                    return Ok(state);
                }
                Err(err) => {
                    log::warn!("failed to adopt prewarmed GL context ({err:#}); creating fresh");
                }
            }
        }
        Self::create_impl(window)
    }

    /// Re-target a context created on the prewarm thread at the real
    /// window: replicate the pixel format it was created against, then
    /// make it current on the window's DC.
    fn adopt_prewarmed(pre: PrewarmedGl, window: HWND) -> anyhow::Result<Self> {
        let hdc = unsafe { GetDC(window) };
        if hdc.is_null() {
            anyhow::bail!("GetDC failed: {}", IoError::last_os_error());
        }

        if unsafe { SetPixelFormat(hdc, pre.format_id, &pre.pfd) } == 0 {
            anyhow::bail!(
                "SetPixelFormat({}) failed: {}",
                pre.format_id,
                IoError::last_os_error()
            );
        }

        if unsafe { pre.wgl.wgl.MakeCurrent(hdc as *mut _, pre.rc) } == 0 {
            anyhow::bail!(
                "MakeCurrent with prewarmed context failed: {}",
                IoError::last_os_error()
            );
        }

        Ok(Self {
            wgl: Some(pre.wgl),
            rc: pre.rc,
            hdc,
        })
    }

    fn create_impl(window: HWND) -> anyhow::Result<Self> {
        let timing = std::time::Instant::now();
        let wgl = WglWrapper::load()?;
        log::debug!("wgl-timing: WglWrapper::load {:?}", timing.elapsed());

        if let Some(ext) = wgl.ext.as_ref() {
            let hdc = unsafe { GetDC(window) };

            fn cstr(data: *const i8) -> String {
                let data = unsafe { CStr::from_ptr(data).to_bytes().to_vec() };
                String::from_utf8(data).unwrap()
            }

            let extensions = if ext.GetExtensionsStringARB.is_loaded() {
                unsafe { cstr(ext.GetExtensionsStringARB(hdc as *const _)) }
            } else if ext.GetExtensionsStringEXT.is_loaded() {
                unsafe { cstr(ext.GetExtensionsStringEXT()) }
            } else {
                "".to_owned()
            };
            log::trace!("opengl extensions: {:?}", extensions);

            if has_extension(&extensions, "WGL_ARB_pixel_format") {
                let timing = std::time::Instant::now();
                return match Self::create_ext(wgl, extensions, hdc) {
                    Ok(state) => {
                        log::debug!("wgl-timing: create_ext {:?}", timing.elapsed());
                        Ok(state)
                    }
                    Err(err) => {
                        log::warn!(
                            "failed to created extended OpenGL context \
                            ({}), fall back to basic",
                            err
                        );
                        let wgl = WglWrapper::load()?;
                        Self::create_basic(wgl, window)
                    }
                };
            }
        }

        Self::create_basic(wgl, window)
    }

    fn create_ext(wgl: WglWrapper, extensions: String, hdc: HDC) -> anyhow::Result<Self> {
        use ffiextra::*;

        // Note: no SAMPLE_BUFFERS/SAMPLES (MSAA) here. The renderer draws
        // axis-aligned textured quads — glyph antialiasing happens at
        // rasterization time in the glyph atlas — so multisampling buys
        // nothing visually, while making pixel-format selection and
        // context creation measurably slower and the swap chain fatter.
        // The EGL path never requested MSAA either.
        let mut attribs: Vec<i32> = vec![
            DRAW_TO_WINDOW_ARB as i32,
            1,
            SUPPORT_OPENGL_ARB as i32,
            1,
            DOUBLE_BUFFER_ARB as i32,
            1,
            PIXEL_TYPE_ARB as i32,
            TYPE_RGBA_ARB as i32,
            COLOR_BITS_ARB as i32,
            24,
            ALPHA_BITS_ARB as i32,
            8,
            DEPTH_BITS_ARB as i32,
            24,
            STENCIL_BITS_ARB as i32,
            8,
        ];

        if has_extension(&extensions, "WGL_ARB_framebuffer_sRGB") {
            log::trace!("will request FRAMEBUFFER_SRGB_CAPABLE_ARB");
            attribs.push(FRAMEBUFFER_SRGB_CAPABLE_ARB as i32);
            attribs.push(1);
        } else if has_extension(&extensions, "WGL_EXT_framebuffer_sRGB") {
            log::trace!("will request FRAMEBUFFER_SRGB_CAPABLE_EXT");
            attribs.push(FRAMEBUFFER_SRGB_CAPABLE_EXT as i32);
            attribs.push(1);
        }

        attribs.push(0);

        let mut format_id = 0;
        let mut num_formats = 0;

        let res = unsafe {
            wgl.ext.as_ref().unwrap().ChoosePixelFormatARB(
                hdc as _,
                attribs.as_ptr(),
                null(),
                1,
                &mut format_id,
                &mut num_formats,
            )
        };
        if res == 0 {
            anyhow::bail!("ChoosePixelFormatARB returned 0");
        }

        if num_formats == 0 {
            anyhow::bail!("ChoosePixelFormatARB returned 0 formats");
        }

        let mut pfd: PIXELFORMATDESCRIPTOR = unsafe { std::mem::zeroed() };

        let res = unsafe {
            DescribePixelFormat(
                hdc,
                format_id,
                std::mem::size_of::<PIXELFORMATDESCRIPTOR>() as _,
                &mut pfd,
            )
        };
        if res == 0 {
            anyhow::bail!(
                "DescribePixelFormat function failed: {}",
                std::io::Error::last_os_error()
            );
        }

        let res = unsafe { SetPixelFormat(hdc, format_id, &pfd) };
        if res == 0 {
            anyhow::bail!(
                "SetPixelFormat function failed: {}",
                std::io::Error::last_os_error()
            );
        }

        let mut attribs = vec![
            CONTEXT_MAJOR_VERSION_ARB as i32,
            4,
            CONTEXT_MINOR_VERSION_ARB as i32,
            5,
            CONTEXT_PROFILE_MASK_ARB as i32,
            CONTEXT_CORE_PROFILE_BIT_ARB as i32,
        ];

        if has_extension(&extensions, "WGL_ARB_create_context_robustness") {
            log::trace!("requesting robustness features");
            attribs.push(CONTEXT_RESET_NOTIFICATION_STRATEGY_ARB as i32);
            attribs.push(LOSE_CONTEXT_ON_RESET_ARB as i32);
            attribs.push(CONTEXT_FLAGS_ARB as i32);
            attribs.push(CONTEXT_ROBUST_ACCESS_BIT_ARB as i32);
        }
        attribs.push(0);

        let rc = unsafe {
            wgl.ext
                .as_ref()
                .unwrap()
                .CreateContextAttribsARB(hdc as _, null(), attribs.as_ptr())
        };

        if rc.is_null() {
            let err = unsafe { winapi::um::errhandlingapi::GetLastError() };
            anyhow::bail!(
                "CreateContextAttribsARB failed, GetLastError={} {:x}",
                err,
                err
            );
        }

        unsafe {
            wgl.wgl.MakeCurrent(hdc as *mut _, rc);
        }

        Ok(Self {
            wgl: Some(wgl),
            rc,
            hdc,
        })
    }

    fn create_basic(wgl: WglWrapper, window: HWND) -> anyhow::Result<Self> {
        let hdc = unsafe { GetDC(window) };

        let pfd = PIXELFORMATDESCRIPTOR {
            nSize: std::mem::size_of::<PIXELFORMATDESCRIPTOR>() as u16,
            nVersion: 1,
            dwFlags: PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER,
            iPixelType: PFD_TYPE_RGBA,
            cColorBits: 24,
            cRedBits: 0,
            cRedShift: 0,
            cGreenBits: 0,
            cGreenShift: 0,
            cBlueBits: 0,
            cBlueShift: 0,
            cAlphaBits: 8,
            cAlphaShift: 0,
            cAccumBits: 0,
            cAccumRedBits: 0,
            cAccumGreenBits: 0,
            cAccumBlueBits: 0,
            cAccumAlphaBits: 0,
            cDepthBits: 24,
            cStencilBits: 8,
            cAuxBuffers: 0,
            iLayerType: PFD_MAIN_PLANE,
            bReserved: 0,
            dwLayerMask: 0,
            dwVisibleMask: 0,
            dwDamageMask: 0,
        };
        let format = unsafe { ChoosePixelFormat(hdc, &pfd) };
        unsafe {
            SetPixelFormat(hdc, format, &pfd);
        }

        let rc = unsafe { wgl.wgl.CreateContext(hdc as *mut _) };
        unsafe {
            wgl.wgl.MakeCurrent(hdc as *mut _, rc);
        }

        Ok(Self {
            wgl: Some(wgl),
            rc,
            hdc,
        })
    }

    fn make_not_current(&self) {
        if let Some(wgl) = self.wgl.as_ref() {
            unsafe {
                wgl.wgl.MakeCurrent(self.hdc as *mut _, std::ptr::null());
            }
        }
    }

    fn delete(&mut self) {
        self.make_not_current();
        if let Some(wgl) = self.wgl.as_ref() {
            unsafe {
                wgl.wgl.DeleteContext(self.rc);
            }
        }
    }
}

impl Drop for GlState {
    fn drop(&mut self) {
        self.delete();
    }
}

unsafe impl glium::backend::Backend for GlState {
    fn resize(&self, _: (u32, u32)) {
        todo!()
    }

    fn swap_buffers(&self) -> Result<(), glium::SwapBuffersError> {
        unsafe {
            SwapBuffers(self.hdc);
        }
        Ok(())
    }

    unsafe fn get_proc_address(&self, symbol: &str) -> *const c_void {
        let sym_name = std::ffi::CString::new(symbol).expect("symbol to be cstring compatible");
        if let Ok(sym) = self
            .wgl
            .as_ref()
            .unwrap()
            .lib
            .get(sym_name.as_bytes_with_nul())
        {
            //eprintln!("{} -> {:?}", symbol, sym);
            return *sym;
        }
        let res = self
            .wgl
            .as_ref()
            .unwrap()
            .wgl
            .GetProcAddress(sym_name.as_ptr()) as *const c_void;
        // eprintln!("{} -> {:?}", symbol, res);
        res
    }

    fn get_framebuffer_dimensions(&self) -> (u32, u32) {
        unimplemented!();
    }

    fn is_current(&self) -> bool {
        unsafe { self.wgl.as_ref().unwrap().wgl.GetCurrentContext() == self.rc }
    }

    unsafe fn make_current(&self) {
        self.wgl
            .as_ref()
            .unwrap()
            .wgl
            .MakeCurrent(self.hdc as *mut _, self.rc);
    }
}
