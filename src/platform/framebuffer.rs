use std::fs::{File, OpenOptions};
use std::io;
use std::os::fd::AsRawFd;
use std::ptr;

#[repr(C)] #[derive(Default, Copy, Clone)]
struct Bitfield { offset: u32, length: u32, msb_right: u32 }
#[repr(C)] #[derive(Default)]
struct VarInfo {
    xres:u32,yres:u32,xres_virtual:u32,yres_virtual:u32,xoffset:u32,yoffset:u32,
    bits_per_pixel:u32,grayscale:u32,red:Bitfield,green:Bitfield,blue:Bitfield,transp:Bitfield,
    nonstd:u32,activate:u32,height:u32,width:u32,accel_flags:u32,pixclock:u32,left_margin:u32,
    right_margin:u32,upper_margin:u32,lower_margin:u32,hsync_len:u32,vsync_len:u32,sync:u32,
    vmode:u32,rotate:u32,colorspace:u32,reserved:[u32;4]
}
#[repr(C)] #[derive(Default)]
struct FixInfo { id:[u8;16],smem_start:libc::c_ulong,smem_len:u32,fb_type:u32,type_aux:u32,visual:u32,xpanstep:u16,ypanstep:u16,ywrapstep:u16,line_length:u32,mmio_start:libc::c_ulong,mmio_len:u32,accel:u32,capabilities:u16,reserved:[u16;2] }

pub struct Framebuffer { _file: File, ptr: *mut u8, len: usize, var: VarInfo, fix: FixInfo }
impl Framebuffer {
    pub fn open() -> io::Result<Self> {
        let file = OpenOptions::new().read(true).write(true).open("/dev/fb0")?;
        let mut var=VarInfo::default(); let mut fix=FixInfo::default();
        unsafe {
            if libc::ioctl(file.as_raw_fd(), 0x4600 as _, &mut var) < 0 || libc::ioctl(file.as_raw_fd(), 0x4602 as _, &mut fix) < 0 { return Err(io::Error::last_os_error()); }
            let len=fix.smem_len as usize;
            let ptr=libc::mmap(ptr::null_mut(),len,libc::PROT_READ|libc::PROT_WRITE,libc::MAP_SHARED,file.as_raw_fd(),0);
            if ptr==libc::MAP_FAILED { return Err(io::Error::last_os_error()); }
            Ok(Self{_file:file,ptr:ptr.cast(),len,var,fix})
        }
    }
    pub fn present(&mut self, src:&[u32], width:usize, height:usize) -> io::Result<()> {
        let dw=self.var.xres as usize; let dh=self.var.yres as usize; let bpp=(self.var.bits_per_pixel/8) as usize;
        if !matches!(bpp,2|4) { return Err(io::Error::new(io::ErrorKind::Unsupported,"仅支持 RGB565/32-bit framebuffer")); }
        let copy_w=width.min(dw); let copy_h=height.min(dh);
        for y in 0..copy_h { for x in 0..copy_w {
            let c=src[y*width+x]; let r=(c>>16)&255; let g=(c>>8)&255; let b=c&255;
            let off=(y+self.var.yoffset as usize)*self.fix.line_length as usize+(x+self.var.xoffset as usize)*bpp;
            if off+bpp>self.len { continue; }
            unsafe { if bpp==2 { ptr::write_unaligned(self.ptr.add(off).cast::<u16>(),(((r>>3)<<11)|((g>>2)<<5)|(b>>3)) as u16); }
            else { let pack=Self::field(r,self.var.red)|Self::field(g,self.var.green)|Self::field(b,self.var.blue); ptr::write_unaligned(self.ptr.add(off).cast::<u32>(),pack); } }
        }}
        Ok(())
    }
    fn field(v:u32, f:Bitfield)->u32 { if f.length==0 {0} else { (v*((1u32<<f.length)-1)/255)<<f.offset } }
}
impl Drop for Framebuffer { fn drop(&mut self){ unsafe { libc::munmap(self.ptr.cast(),self.len); } } }
