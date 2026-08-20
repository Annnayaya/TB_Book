#[cfg(target_os = "linux")]
use std::fs::{File, OpenOptions};
#[cfg(target_os = "linux")]
use std::os::unix::io::AsRawFd;
#[cfg(target_os = "linux")]
use std::ptr::null_mut;

pub const FBIOGET_VSCREENINFO: libc::c_ulong = 0x4600;
pub const FBIOGET_FSCREENINFO: libc::c_ulong = 0x4602;
pub const FBIOPAN_DISPLAY: libc::c_ulong = 0x4606;

#[cfg(target_os = "linux")]
#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct fb_bitfield {
    pub offset: u32,
    pub length: u32,
    pub msb_right: u32,
}

#[cfg(target_os = "linux")]
#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct fb_var_screeninfo {
    pub xres: u32,
    pub yres: u32,
    pub xres_virtual: u32,
    pub yres_virtual: u32,
    pub xoffset: u32,
    pub yoffset: u32,
    pub bits_per_pixel: u32,
    pub grayscale: u32,
    pub red: fb_bitfield,
    pub green: fb_bitfield,
    pub blue: fb_bitfield,
    pub transp: fb_bitfield,
    pub nonstd: u32,
    pub activate: u32,
    pub height: u32,
    pub width: u32,
    pub accel_flags: u32,
    pub pixclock: u32,
    pub left_margin: u32,
    pub right_margin: u32,
    pub upper_margin: u32,
    pub lower_margin: u32,
    pub hsync_len: u32,
    pub vsync_len: u32,
    pub sync: u32,
    pub vmode: u32,
    pub rotate: u32,
    pub colorspace: u32,
    pub reserved: [u32; 4],
}

#[cfg(target_os = "linux")]
#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct fb_fix_screeninfo {
    pub id: [u8; 16],
    pub smem_start: usize,
    pub smem_len: u32,
    pub type_: u32,
    pub type_aux: u32,
    pub visual: u32,
    pub xpanstep: u16,
    pub ypanstep: u16,
    pub ywrapstep: u16,
    pub line_length: u32,
    pub mmio_start: usize,
    pub mmio_len: u32,
    pub accel: u32,
    pub capabilities: u16,
    pub reserved: [u16; 2],
}

pub struct FramebufferDisplay {
    #[cfg(target_os = "linux")]
    _file: File,
    #[cfg(target_os = "linux")]
    mmap_ptr: *mut u8,
    #[cfg(target_os = "linux")]
    smem_len: usize,
    pub width: u32,
    pub height: u32,
    pub xoffset: u32,
    pub yoffset: u32,
    pub line_length: u32,
    pub bpp: u32,
    pub is_bgr: bool,
}

impl FramebufferDisplay {
    #[cfg(target_os = "linux")]
    pub fn open_default() -> Result<Self, String> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/fb0")
            .map_err(|e| format!("无法打开 /dev/fb0: {}", e))?;

        let fd = file.as_raw_fd();

        let mut vinfo: fb_var_screeninfo = fb_var_screeninfo::default();
        let mut finfo: fb_fix_screeninfo = fb_fix_screeninfo::default();

        unsafe {
            // libc's ioctl request type differs between GNU and musl targets
            // (c_ulong vs c_int). Let the target-specific signature drive the
            // cast so the framebuffer backend can be built for both ABIs.
            if libc::ioctl(fd, FBIOGET_VSCREENINFO as _, &mut vinfo) < 0 {
                return Err("无法获取 fb_var_screeninfo (ioctl 失败)".to_string());
            }
            if libc::ioctl(fd, FBIOGET_FSCREENINFO as _, &mut finfo) < 0 {
                return Err("无法获取 fb_fix_screeninfo (ioctl 失败)".to_string());
            }
        }

        let width = if vinfo.xres > 0 { vinfo.xres } else { 1024 };
        let height = if vinfo.yres > 0 { vinfo.yres } else { 768 };
        let bpp = if vinfo.bits_per_pixel > 0 {
            vinfo.bits_per_pixel
        } else {
            32
        };
        let line_length = if finfo.line_length > 0 {
            finfo.line_length
        } else {
            width * (bpp / 8)
        };
        let smem_len = if finfo.smem_len > 0 {
            finfo.smem_len as usize
        } else {
            (line_length * height) as usize
        };

        let mmap_ptr = unsafe {
            libc::mmap(
                null_mut(),
                smem_len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            )
        };

        if mmap_ptr == libc::MAP_FAILED || mmap_ptr.is_null() {
            return Err("无法 mmap /dev/fb0 帧缓冲内存".to_string());
        }

        let is_bgr = vinfo.blue.offset > vinfo.red.offset;

        println!(
            "✓ Linux Framebuffer (/dev/fb0) 就绪: {}x{}, {}bpp, stride={}, is_bgr={}",
            width, height, bpp, line_length, is_bgr
        );

        Ok(Self {
            _file: file,
            mmap_ptr: mmap_ptr as *mut u8,
            smem_len,
            width,
            height,
            xoffset: vinfo.xoffset,
            yoffset: vinfo.yoffset,
            line_length,
            bpp,
            is_bgr,
        })
    }

    #[cfg(not(target_os = "linux"))]
    pub fn open_default() -> Result<Self, String> {
        Ok(Self {
            width: 1024,
            height: 768,
            xoffset: 0,
            yoffset: 0,
            line_length: 1024 * 4,
            bpp: 32,
            is_bgr: false,
        })
    }

    #[cfg(target_os = "linux")]
    pub fn present(&mut self, src_buffer: &[u32], src_width: usize, src_height: usize) {
        if self.mmap_ptr.is_null() {
            return;
        }

        let bytes_per_pixel = (self.bpp / 8) as usize;
        if bytes_per_pixel == 0 {
            return;
        }

        let dst_stride = self.line_length as usize;
        let x_byte_offset = self.xoffset as usize * bytes_per_pixel;
        let visible_offset = self.yoffset as usize * dst_stride + x_byte_offset;
        if dst_stride == 0 || x_byte_offset >= dst_stride || visible_offset >= self.smem_len {
            return;
        }

        let row_capacity = (dst_stride - x_byte_offset) / bytes_per_pixel;
        let copy_w = src_width.min(self.width as usize).min(row_capacity);
        if copy_w == 0 {
            return;
        }

        let row_bytes = copy_w * bytes_per_pixel;
        let available_bytes = self.smem_len - visible_offset;
        let max_rows = if available_bytes < row_bytes {
            0
        } else {
            1 + (available_bytes - row_bytes) / dst_stride
        };
        let src_rows = src_buffer.len() / src_width;
        let copy_h = src_height
            .min(self.height as usize)
            .min(max_rows)
            .min(src_rows);
        if copy_h == 0 {
            return;
        }

        unsafe {
            let dst_base = self.mmap_ptr.add(visible_offset);
            if self.bpp == 32 {
                let src_stride = src_width;

                if dst_stride == src_stride * 4
                    && !self.is_bgr
                    && copy_w == self.width as usize
                    && copy_h == self.height as usize
                {
                    // Fast full-buffer copy
                    let total_bytes = copy_w * copy_h * 4;
                    std::ptr::copy_nonoverlapping(
                        src_buffer.as_ptr() as *const u8,
                        dst_base,
                        total_bytes,
                    );
                } else {
                    for y in 0..copy_h {
                        let dst_row = dst_base.add(y * dst_stride) as *mut u32;
                        let src_row = &src_buffer[y * src_stride..(y * src_stride + copy_w)];

                        if self.is_bgr {
                            for (x, &c) in src_row.iter().enumerate() {
                                let r = (c >> 16) & 0xFF;
                                let g = (c >> 8) & 0xFF;
                                let b = c & 0xFF;
                                *dst_row.add(x) = (b << 16) | (g << 8) | r;
                            }
                        } else {
                            std::ptr::copy_nonoverlapping(src_row.as_ptr(), dst_row, copy_w);
                        }
                    }
                }
            } else if self.bpp == 16 {
                // RGB565 fallback conversion
                let src_stride = src_width;

                for y in 0..copy_h {
                    let dst_row = dst_base.add(y * dst_stride) as *mut u16;
                    for x in 0..copy_w {
                        let c = src_buffer[y * src_stride + x];
                        let r = ((c >> 16) & 0xFF) as u16;
                        let g = ((c >> 8) & 0xFF) as u16;
                        let b = (c & 0xFF) as u16;
                        let rgb565 = ((r & 0xF8) << 8) | ((g & 0xFC) << 3) | (b >> 3);
                        *dst_row.add(x) = rgb565;
                    }
                }
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    pub fn present(&mut self, _src_buffer: &[u32], _src_width: usize, _src_height: usize) {}
}

#[cfg(target_os = "linux")]
impl Drop for FramebufferDisplay {
    fn drop(&mut self) {
        if !self.mmap_ptr.is_null() && self.smem_len > 0 {
            unsafe {
                libc::munmap(self.mmap_ptr as *mut libc::c_void, self.smem_len);
            }
        }
    }
}
