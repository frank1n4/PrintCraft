//! Sending RIP output to a Windows printer at its real physical size (GDI).

#[cfg(windows)]
mod imp {
    use std::path::Path;
    use windows_sys::Win32::Graphics::Gdi::*;
    use windows_sys::Win32::Graphics::Printing::*;
    use windows_sys::Win32::Storage::Xps::*;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    unsafe fn from_wide(p: *const u16) -> String {
        if p.is_null() {
            return String::new();
        }
        let mut n = 0;
        while *p.add(n) != 0 {
            n += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(p, n))
    }

    pub fn list_printers() -> Vec<String> {
        let mut out = Vec::new();
        unsafe {
            let flags = PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS;
            let mut needed = 0u32;
            let mut count = 0u32;
            EnumPrintersW(flags, std::ptr::null(), 4, std::ptr::null_mut(), 0, &mut needed, &mut count);
            if needed == 0 {
                return out;
            }
            let mut buf = vec![0u8; needed as usize];
            if EnumPrintersW(flags, std::ptr::null(), 4, buf.as_mut_ptr(), needed, &mut needed, &mut count) == 0 {
                return out;
            }
            let infos = buf.as_ptr() as *const PRINTER_INFO_4W;
            for i in 0..count as usize {
                out.push(from_wide((*infos.add(i)).pPrinterName));
            }
        }
        out
    }

    /// Prints each PNG on its own page at `dpi` (so 1 px = 1/dpi inch on the media).
    pub fn print_files(printer: &str, files: &[std::path::PathBuf], dpi: f32, doc: &str) -> Result<(), String> {
        unsafe {
            let pname = wide(printer);
            let hdc = CreateDCW(std::ptr::null(), pname.as_ptr(), std::ptr::null(), std::ptr::null());
            if hdc.is_null() {
                return Err(format!("não consegui abrir a impressora \"{printer}\""));
            }
            let dname = wide(doc);
            let di = DOCINFOW {
                cbSize: std::mem::size_of::<DOCINFOW>() as i32,
                lpszDocName: dname.as_ptr(),
                lpszOutput: std::ptr::null(),
                lpszDatatype: std::ptr::null(),
                fwType: 0,
            };
            if StartDocW(hdc, &di) <= 0 {
                DeleteDC(hdc);
                return Err("a impressora recusou o trabalho".into());
            }
            let dev_x = GetDeviceCaps(hdc, LOGPIXELSX as _) as f32;
            let dev_y = GetDeviceCaps(hdc, LOGPIXELSY as _) as f32;
            let mut result = Ok(());
            for f in files {
                if let Err(e) = page(hdc, f, dpi, dev_x, dev_y) {
                    result = Err(e);
                    break;
                }
            }
            EndDoc(hdc);
            DeleteDC(hdc);
            result
        }
    }

    unsafe fn page(hdc: HDC, file: &Path, dpi: f32, dev_x: f32, dev_y: f32) -> Result<(), String> {
        let img = image::open(file).map_err(|e| e.to_string())?.to_rgb8();
        let (w, h) = (img.width() as i32, img.height() as i32);
        StartPage(hdc);
        let band = 1024i32;
        let mut y = 0;
        while y < h {
            let bh = band.min(h - y);
            let mut bgra = Vec::with_capacity((w * bh * 4) as usize);
            for yy in y..y + bh {
                for xx in 0..w {
                    let p = img.get_pixel(xx as u32, yy as u32);
                    bgra.extend_from_slice(&[p[2], p[1], p[0], 0]);
                }
            }
            let mut bmi: BITMAPINFO = std::mem::zeroed();
            bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bmi.bmiHeader.biWidth = w;
            bmi.bmiHeader.biHeight = -bh;
            bmi.bmiHeader.biPlanes = 1;
            bmi.bmiHeader.biBitCount = 32;
            bmi.bmiHeader.biCompression = BI_RGB as u32;
            let dx = 0;
            let dy = (y as f32 / dpi * dev_y).round() as i32;
            let dw = (w as f32 / dpi * dev_x).round() as i32;
            let dh = ((y + bh) as f32 / dpi * dev_y).round() as i32 - dy;
            StretchDIBits(
                hdc,
                dx,
                dy,
                dw,
                dh,
                0,
                0,
                w,
                bh,
                bgra.as_ptr() as *const _,
                &bmi,
                DIB_RGB_COLORS,
                SRCCOPY,
            );
            y += bh;
        }
        if EndPage(hdc) <= 0 {
            return Err("falha ao enviar a página".into());
        }
        Ok(())
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn list_printers() -> Vec<String> {
        Vec::new()
    }
    pub fn print_files(_: &str, _: &[std::path::PathBuf], _: f32, _: &str) -> Result<(), String> {
        Err("impressão direta só no Windows".into())
    }
}

pub use imp::*;
