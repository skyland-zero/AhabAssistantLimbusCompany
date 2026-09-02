//! Small C ABI bridge around the FFmpeg decoder used by Scrcpy.
//!
//! The bridge deliberately loads FFmpeg at runtime.  This keeps the native
//! adapter small and lets the release bundle ship only the selected FFmpeg
//! DLLs instead of linking the Python PyAV wheel into the sidecar.

#![allow(clippy::missing_safety_doc)]

use std::ffi::{c_char, c_int, c_void, CStr, OsString};
use std::mem::transmute_copy;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::{Mutex, OnceLock};

const AV_CODEC_ID_H264: c_int = 27;
const AV_PIX_FMT_YUV420P: c_int = 0;
const AV_PIX_FMT_YUVJ420P: c_int = 12;
const AV_CODEC_FLAG_LOW_DELAY: i64 = 1 << 19;
const AV_INPUT_BUFFER_PADDING_SIZE: usize = 64;
const AVERROR_EAGAIN: c_int = -11;
const AVERROR_EOF: c_int = -541_478_725;

type FnAvcodecFindDecoder = unsafe extern "C" fn(c_int) -> *const c_void;
type FnAvcodecAllocContext3 = unsafe extern "C" fn(*const c_void) -> *mut c_void;
type FnAvcodecOpen2 = unsafe extern "C" fn(*mut c_void, *const c_void, *mut *mut c_void) -> c_int;
type FnAvcodecFreeContext = unsafe extern "C" fn(*mut *mut c_void);
type FnAvcodecFlushBuffers = unsafe extern "C" fn(*mut c_void);
type FnAvcodecSendPacket = unsafe extern "C" fn(*mut c_void, *const c_void) -> c_int;
type FnAvcodecReceiveFrame = unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int;

type FnAvPacketAlloc = unsafe extern "C" fn() -> *mut c_void;
type FnAvPacketFree = unsafe extern "C" fn(*mut *mut c_void);
type FnAvPacketFromData = unsafe extern "C" fn(*mut c_void, *mut u8, c_int) -> c_int;
type FnAvPacketUnref = unsafe extern "C" fn(*mut c_void);

type FnAvFrameAlloc = unsafe extern "C" fn() -> *mut c_void;
type FnAvFrameFree = unsafe extern "C" fn(*mut *mut c_void);
type FnAvFrameUnref = unsafe extern "C" fn(*mut c_void);

type FnAvMalloc = unsafe extern "C" fn(usize) -> *mut c_void;
type FnAvFree = unsafe extern "C" fn(*mut c_void);
type FnAvStrerror = unsafe extern "C" fn(c_int, *mut c_char, usize) -> c_int;
type FnAvOptSet = unsafe extern "C" fn(*mut c_void, *const c_char, *const c_char, c_int) -> c_int;
type FnAvOptSetInt = unsafe extern "C" fn(*mut c_void, *const c_char, i64, c_int) -> c_int;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn LoadLibraryW(name: *const u16) -> *mut c_void;
    fn FreeLibrary(module: *mut c_void) -> i32;
    fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
}

static LAST_ERROR: OnceLock<Mutex<String>> = OnceLock::new();

fn last_error_slot() -> &'static Mutex<String> {
    LAST_ERROR.get_or_init(|| Mutex::new(String::new()))
}

fn set_last_error(message: impl Into<String>) {
    if let Ok(mut error) = last_error_slot().lock() {
        *error = message.into();
    }
}

fn copy_last_error(output: *mut c_char, capacity: usize) -> usize {
    let message = last_error_slot()
        .lock()
        .map(|error| error.clone())
        .unwrap_or_else(|_| "native decoder error state is unavailable".to_owned());
    let bytes = message.as_bytes();
    if !output.is_null() && capacity > 0 {
        let copy_len = bytes.len().min(capacity - 1);
        unsafe {
            ptr::copy_nonoverlapping(bytes.as_ptr(), output.cast::<u8>(), copy_len);
            *output.add(copy_len) = 0;
        }
    }
    bytes.len()
}

fn dll_path(directory: &Path, name: &str) -> PathBuf {
    directory.join(name)
}

fn wide_null(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain([0]).collect()
}

unsafe fn load_library(path: &Path) -> Result<*mut c_void, String> {
    let path_wide = wide_null(path);
    let module = unsafe { LoadLibraryW(path_wide.as_ptr()) };
    if module.is_null() {
        return Err(format!("无法加载 FFmpeg DLL：{}", path.display()));
    }
    Ok(module)
}

unsafe fn load_symbol<T: Copy>(module: *mut c_void, name: &'static [u8]) -> Result<T, String> {
    let address = unsafe { GetProcAddress(module, name.as_ptr()) };
    if address.is_null() {
        let symbol = String::from_utf8_lossy(&name[..name.len().saturating_sub(1)]);
        return Err(format!("FFmpeg DLL 缺少导出符号：{symbol}"));
    }
    // Function pointers and data pointers have the same representation on the
    // Windows targets supported by this project.  The symbol is checked for
    // null above before converting it to its declared function type.
    Ok(unsafe { transmute_copy(&address) })
}

struct Api {
    avutil_module: *mut c_void,
    swresample_module: *mut c_void,
    avcodec_module: *mut c_void,

    avcodec_find_decoder: FnAvcodecFindDecoder,
    avcodec_alloc_context3: FnAvcodecAllocContext3,
    avcodec_open2: FnAvcodecOpen2,
    avcodec_free_context: FnAvcodecFreeContext,
    avcodec_flush_buffers: FnAvcodecFlushBuffers,
    avcodec_send_packet: FnAvcodecSendPacket,
    avcodec_receive_frame: FnAvcodecReceiveFrame,

    av_packet_alloc: FnAvPacketAlloc,
    av_packet_free: FnAvPacketFree,
    av_packet_from_data: FnAvPacketFromData,
    av_packet_unref: FnAvPacketUnref,

    av_frame_alloc: FnAvFrameAlloc,
    av_frame_free: FnAvFrameFree,
    av_frame_unref: FnAvFrameUnref,

    av_malloc: FnAvMalloc,
    av_free: FnAvFree,
    av_strerror: FnAvStrerror,
    av_opt_set: Option<FnAvOptSet>,
    av_opt_set_int: Option<FnAvOptSetInt>,
}

unsafe impl Send for Api {}
unsafe impl Sync for Api {}

impl Api {
    unsafe fn load(directory: &Path) -> Result<Self, String> {
        let avutil_module = unsafe { load_library(&dll_path(directory, "avutil-60.dll"))? };
        let swresample_module =
            match unsafe { load_library(&dll_path(directory, "swresample-6.dll")) } {
                Ok(module) => module,
                Err(error) => {
                    unsafe { FreeLibrary(avutil_module) };
                    return Err(error);
                }
            };
        let avcodec_module = match unsafe { load_library(&dll_path(directory, "avcodec-62.dll")) } {
            Ok(module) => module,
            Err(error) => {
                unsafe {
                    FreeLibrary(swresample_module);
                    FreeLibrary(avutil_module);
                }
                return Err(error);
            }
        };

        macro_rules! required {
            ($module:expr, $name:literal, $ty:ty) => {
                match unsafe { load_symbol::<$ty>($module, concat!($name, "\0").as_bytes()) } {
                    Ok(value) => value,
                    Err(error) => {
                        unsafe {
                            FreeLibrary(avcodec_module);
                            FreeLibrary(swresample_module);
                            FreeLibrary(avutil_module);
                        }
                        return Err(error);
                    }
                }
            };
        }

        let api = Self {
            avutil_module,
            swresample_module,
            avcodec_module,

            avcodec_find_decoder: required!(
                avcodec_module,
                "avcodec_find_decoder",
                FnAvcodecFindDecoder
            ),
            avcodec_alloc_context3: required!(
                avcodec_module,
                "avcodec_alloc_context3",
                FnAvcodecAllocContext3
            ),
            avcodec_open2: required!(avcodec_module, "avcodec_open2", FnAvcodecOpen2),
            avcodec_free_context: required!(
                avcodec_module,
                "avcodec_free_context",
                FnAvcodecFreeContext
            ),
            avcodec_flush_buffers: required!(
                avcodec_module,
                "avcodec_flush_buffers",
                FnAvcodecFlushBuffers
            ),
            avcodec_send_packet: required!(
                avcodec_module,
                "avcodec_send_packet",
                FnAvcodecSendPacket
            ),
            avcodec_receive_frame: required!(
                avcodec_module,
                "avcodec_receive_frame",
                FnAvcodecReceiveFrame
            ),

            av_packet_alloc: required!(avcodec_module, "av_packet_alloc", FnAvPacketAlloc),
            av_packet_free: required!(avcodec_module, "av_packet_free", FnAvPacketFree),
            av_packet_from_data: required!(
                avcodec_module,
                "av_packet_from_data",
                FnAvPacketFromData
            ),
            av_packet_unref: required!(avcodec_module, "av_packet_unref", FnAvPacketUnref),

            av_frame_alloc: required!(avutil_module, "av_frame_alloc", FnAvFrameAlloc),
            av_frame_free: required!(avutil_module, "av_frame_free", FnAvFrameFree),
            av_frame_unref: required!(avutil_module, "av_frame_unref", FnAvFrameUnref),

            av_malloc: required!(avutil_module, "av_malloc", FnAvMalloc),
            av_free: required!(avutil_module, "av_free", FnAvFree),
            av_strerror: required!(avutil_module, "av_strerror", FnAvStrerror),
            av_opt_set: unsafe { load_symbol(avutil_module, b"av_opt_set\0").ok() },
            av_opt_set_int: unsafe { load_symbol(avutil_module, b"av_opt_set_int\0").ok() },
        };
        Ok(api)
    }
}

impl Drop for Api {
    fn drop(&mut self) {
        unsafe {
            FreeLibrary(self.avcodec_module);
            FreeLibrary(self.swresample_module);
            FreeLibrary(self.avutil_module);
        }
    }
}

#[repr(C)]
struct AvFramePrefix {
    data: [*mut u8; 8],
    linesize: [c_int; 8],
    extended_data: *mut *mut u8,
    width: c_int,
    height: c_int,
    nb_samples: c_int,
    format: c_int,
}

#[repr(C)]
pub struct ScrcpyFrameInfo {
    pub width: u32,
    pub height: u32,
    pub y: *const u8,
    pub y_len: usize,
    pub u: *const u8,
    pub u_len: usize,
    pub v: *const u8,
    pub v_len: usize,
    pub y_stride: u32,
    pub uv_stride: u32,
}

impl Default for ScrcpyFrameInfo {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            y: ptr::null(),
            y_len: 0,
            u: ptr::null(),
            u_len: 0,
            v: ptr::null(),
            v_len: 0,
            y_stride: 0,
            uv_stride: 0,
        }
    }
}

struct OwnedFrame {
    width: u32,
    height: u32,
    y: Vec<u8>,
    u: Vec<u8>,
    v: Vec<u8>,
}

impl OwnedFrame {
    fn info(&self) -> ScrcpyFrameInfo {
        let chroma_width = self.width.div_ceil(2);
        ScrcpyFrameInfo {
            width: self.width,
            height: self.height,
            y: self.y.as_ptr(),
            y_len: self.y.len(),
            u: self.u.as_ptr(),
            u_len: self.u.len(),
            v: self.v.as_ptr(),
            v_len: self.v.len(),
            y_stride: self.width,
            uv_stride: chroma_width,
        }
    }
}

fn ffmpeg_error(api: &Api, code: c_int) -> String {
    let mut buffer = [0 as c_char; 256];
    let result = unsafe { (api.av_strerror)(code, buffer.as_mut_ptr(), buffer.len()) };
    if result < 0 {
        return format!("FFmpeg error code {code}");
    }
    let description = unsafe { CStr::from_ptr(buffer.as_ptr()) }.to_string_lossy();
    format!("FFmpeg error {code}: {description}")
}

fn copy_plane(
    data: *const u8,
    linesize: c_int,
    width: usize,
    height: usize,
) -> Result<Vec<u8>, String> {
    if data.is_null() || linesize == 0 {
        return Err("FFmpeg 返回了空的视频平面".to_owned());
    }
    let stride = linesize.unsigned_abs() as usize;
    if stride < width {
        return Err(format!("FFmpeg 视频平面 stride 无效：{stride} < {width}"));
    }
    let size = width
        .checked_mul(height)
        .ok_or_else(|| "视频平面尺寸溢出".to_owned())?;
    let mut output = vec![0_u8; size];
    unsafe {
        for row in 0..height {
            let source = data.offset((row as isize) * (linesize as isize));
            ptr::copy_nonoverlapping(source, output.as_mut_ptr().add(row * width), width);
        }
    }
    Ok(output)
}

struct Decoder {
    api: Api,
    codec_context: *mut c_void,
    packet: *mut c_void,
    frame: *mut c_void,
    pending_config: Vec<u8>,
    latest_frame: Option<OwnedFrame>,
}

unsafe impl Send for Decoder {}

impl Decoder {
    unsafe fn new(directory: &Path, width: u32, height: u32) -> Result<Self, String> {
        let api = unsafe { Api::load(directory)? };
        let codec = unsafe { (api.avcodec_find_decoder)(AV_CODEC_ID_H264) };
        if codec.is_null() {
            return Err("裁剪版 FFmpeg 中未找到 H.264 decoder".to_owned());
        }

        let codec_context = unsafe { (api.avcodec_alloc_context3)(codec) };
        if codec_context.is_null() {
            return Err("分配 H.264 decoder context 失败".to_owned());
        }
        let packet = unsafe { (api.av_packet_alloc)() };
        if packet.is_null() {
            let mut context = codec_context;
            unsafe { (api.avcodec_free_context)(&mut context) };
            return Err("分配 FFmpeg packet 失败".to_owned());
        }
        let frame = unsafe { (api.av_frame_alloc)() };
        if frame.is_null() {
            let mut packet_ptr = packet;
            let mut context = codec_context;
            unsafe {
                (api.av_packet_free)(&mut packet_ptr);
                (api.avcodec_free_context)(&mut context);
            }
            return Err("分配 FFmpeg frame 失败".to_owned());
        }

        // These are the same relevant decoder settings used by Scrcpy's
        // demuxer.  AVOptions keeps us from depending on the private layout of
        // AVCodecContext.
        if let Some(set_int) = api.av_opt_set_int {
            let _ = unsafe {
                set_int(
                    codec_context,
                    b"flags\0".as_ptr().cast(),
                    AV_CODEC_FLAG_LOW_DELAY,
                    0,
                )
            };
            if width > 0 {
                let _ =
                    unsafe { set_int(codec_context, b"width\0".as_ptr().cast(), width as i64, 0) };
            }
            if height > 0 {
                let _ = unsafe {
                    set_int(codec_context, b"height\0".as_ptr().cast(), height as i64, 0)
                };
            }
            let _ = unsafe {
                set_int(
                    codec_context,
                    b"pix_fmt\0".as_ptr().cast(),
                    AV_PIX_FMT_YUV420P as i64,
                    0,
                )
            };
            let thread_count = std::thread::available_parallelism()
                .map(|value| value.get().clamp(1, 4) as i64)
                .unwrap_or(1);
            let _ = unsafe {
                set_int(
                    codec_context,
                    b"thread_count\0".as_ptr().cast(),
                    thread_count,
                    0,
                )
            };
        }
        if let Some(set) = api.av_opt_set {
            let _ = unsafe {
                set(
                    codec_context,
                    b"thread_type\0".as_ptr().cast(),
                    b"slice\0".as_ptr().cast(),
                    0,
                )
            };
        }

        let open_result = unsafe { (api.avcodec_open2)(codec_context, codec, ptr::null_mut()) };
        if open_result < 0 {
            let mut frame_ptr = frame;
            let mut packet_ptr = packet;
            let mut context = codec_context;
            unsafe {
                (api.av_frame_free)(&mut frame_ptr);
                (api.av_packet_free)(&mut packet_ptr);
                (api.avcodec_free_context)(&mut context);
            }
            return Err(ffmpeg_error(&api, open_result));
        }

        Ok(Self {
            api,
            codec_context,
            packet,
            frame,
            pending_config: Vec::new(),
            latest_frame: None,
        })
    }

    unsafe fn send_payload(&mut self, payload: &[u8]) -> Result<(), String> {
        if payload.is_empty() {
            return Err("Scrcpy H.264 packet 为空".to_owned());
        }
        let size =
            i32::try_from(payload.len()).map_err(|_| "H.264 packet 超过 FFmpeg 限制".to_owned())?;
        let allocation_size = payload
            .len()
            .checked_add(AV_INPUT_BUFFER_PADDING_SIZE)
            .ok_or_else(|| "H.264 packet 缓冲区尺寸溢出".to_owned())?;
        let buffer = unsafe { (self.api.av_malloc)(allocation_size) }.cast::<u8>();
        if buffer.is_null() {
            return Err("分配 H.264 packet 缓冲区失败".to_owned());
        }
        unsafe {
            ptr::copy_nonoverlapping(payload.as_ptr(), buffer, payload.len());
            ptr::write_bytes(buffer.add(payload.len()), 0, AV_INPUT_BUFFER_PADDING_SIZE);
        }
        let packet_result = unsafe { (self.api.av_packet_from_data)(self.packet, buffer, size) };
        if packet_result < 0 {
            unsafe { (self.api.av_free)(buffer.cast()) };
            return Err(ffmpeg_error(&self.api, packet_result));
        }

        let send_result =
            unsafe { (self.api.avcodec_send_packet)(self.codec_context, self.packet) };
        unsafe { (self.api.av_packet_unref)(self.packet) };
        if send_result < 0 && send_result != AVERROR_EAGAIN {
            return Err(ffmpeg_error(&self.api, send_result));
        }
        Ok(())
    }

    unsafe fn push(&mut self, payload: &[u8], is_config: bool) -> Result<(), String> {
        if is_config {
            self.pending_config.clear();
            self.pending_config.extend_from_slice(payload);
            return Ok(());
        }

        if self.pending_config.is_empty() {
            unsafe { self.send_payload(payload) }
        } else {
            let mut merged = std::mem::take(&mut self.pending_config);
            merged.extend_from_slice(payload);
            unsafe { self.send_payload(&merged) }
        }
    }

    unsafe fn receive(&mut self) -> Result<bool, String> {
        let result = unsafe { (self.api.avcodec_receive_frame)(self.codec_context, self.frame) };
        if result == AVERROR_EAGAIN || result == AVERROR_EOF {
            return Ok(false);
        }
        if result < 0 {
            return Err(ffmpeg_error(&self.api, result));
        }

        let view = unsafe { &*(self.frame.cast::<AvFramePrefix>()) };
        let width =
            usize::try_from(view.width).map_err(|_| "FFmpeg 返回了无效视频宽度".to_owned())?;
        let height =
            usize::try_from(view.height).map_err(|_| "FFmpeg 返回了无效视频高度".to_owned())?;
        if width == 0 || height == 0 || width > 16_384 || height > 16_384 {
            unsafe { (self.api.av_frame_unref)(self.frame) };
            return Err(format!("FFmpeg 返回了无效视频尺寸：{width}x{height}"));
        }
        if view.format != AV_PIX_FMT_YUV420P && view.format != AV_PIX_FMT_YUVJ420P {
            let format = view.format;
            unsafe { (self.api.av_frame_unref)(self.frame) };
            return Err(format!("裁剪版 decoder 返回了不支持的像素格式：{format}"));
        }

        let chroma_width = width.div_ceil(2);
        let chroma_height = height.div_ceil(2);
        let copied = (
            copy_plane(view.data[0], view.linesize[0], width, height),
            copy_plane(view.data[1], view.linesize[1], chroma_width, chroma_height),
            copy_plane(view.data[2], view.linesize[2], chroma_width, chroma_height),
        );
        unsafe { (self.api.av_frame_unref)(self.frame) };
        let (y, u, v) = (copied.0?, copied.1?, copied.2?);
        self.latest_frame = Some(OwnedFrame {
            width: width as u32,
            height: height as u32,
            y,
            u,
            v,
        });
        Ok(true)
    }

    unsafe fn reset(&mut self) {
        unsafe { (self.api.avcodec_flush_buffers)(self.codec_context) };
        unsafe { (self.api.av_frame_unref)(self.frame) };
        self.pending_config.clear();
        self.latest_frame = None;
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        unsafe {
            if !self.frame.is_null() {
                let mut frame = self.frame;
                (self.api.av_frame_free)(&mut frame);
            }
            if !self.packet.is_null() {
                let mut packet = self.packet;
                (self.api.av_packet_free)(&mut packet);
            }
            if !self.codec_context.is_null() {
                let mut context = self.codec_context;
                (self.api.avcodec_free_context)(&mut context);
            }
        }
    }
}

unsafe fn path_from_wide(pointer: *const u16) -> Result<PathBuf, String> {
    if pointer.is_null() {
        return Ok(PathBuf::from("."));
    }
    let mut length = 0_usize;
    while length < 32_768 && unsafe { *pointer.add(length) } != 0 {
        length += 1;
    }
    if length == 32_768 {
        return Err("FFmpeg DLL 目录参数过长".to_owned());
    }
    let slice = unsafe { std::slice::from_raw_parts(pointer, length) };
    Ok(PathBuf::from(OsString::from_wide(slice)))
}

#[no_mangle]
pub unsafe extern "C" fn scrcpy_decoder_create(
    ffmpeg_directory: *const u16,
    width: u32,
    height: u32,
) -> *mut c_void {
    let directory = match unsafe { path_from_wide(ffmpeg_directory) } {
        Ok(value) => value,
        Err(error) => {
            set_last_error(error);
            return ptr::null_mut();
        }
    };
    match unsafe { Decoder::new(&directory, width, height) } {
        Ok(decoder) => Box::into_raw(Box::new(decoder)).cast(),
        Err(error) => {
            set_last_error(error);
            ptr::null_mut()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn scrcpy_decoder_push(
    handle: *mut c_void,
    payload: *const u8,
    payload_len: usize,
    is_config: c_int,
) -> c_int {
    if handle.is_null() || payload.is_null() || payload_len == 0 {
        set_last_error("native decoder 收到空句柄或空 packet");
        return -1;
    }
    let bytes = unsafe { std::slice::from_raw_parts(payload, payload_len) };
    let decoder = unsafe { &mut *handle.cast::<Decoder>() };
    match unsafe { decoder.push(bytes, is_config != 0) } {
        Ok(()) => 0,
        Err(error) => {
            set_last_error(error);
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn scrcpy_decoder_receive(
    handle: *mut c_void,
    output: *mut ScrcpyFrameInfo,
) -> c_int {
    if handle.is_null() || output.is_null() {
        set_last_error("native decoder 收到空句柄或空 frame 输出");
        return -1;
    }
    unsafe { *output = ScrcpyFrameInfo::default() };
    let decoder = unsafe { &mut *handle.cast::<Decoder>() };
    match unsafe { decoder.receive() } {
        Ok(false) => 0,
        Ok(true) => {
            let Some(frame) = decoder.latest_frame.as_ref() else {
                set_last_error("native decoder 没有保存刚刚解码的 frame");
                return -1;
            };
            unsafe { *output = frame.info() };
            1
        }
        Err(error) => {
            set_last_error(error);
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn scrcpy_decoder_reset(handle: *mut c_void) {
    if handle.is_null() {
        return;
    }
    let decoder = unsafe { &mut *handle.cast::<Decoder>() };
    unsafe { decoder.reset() };
}

#[no_mangle]
pub unsafe extern "C" fn scrcpy_decoder_destroy(handle: *mut c_void) {
    if handle.is_null() {
        return;
    }
    unsafe { drop(Box::from_raw(handle.cast::<Decoder>())) };
}

#[no_mangle]
pub unsafe extern "C" fn scrcpy_decoder_last_error(output: *mut c_char, capacity: usize) -> usize {
    copy_last_error(output, capacity)
}
