//! Screenshot capture for webviews using Windows GDI API

#[cfg(target_os = "windows")]
use windows::{
    Win32::Foundation::{HWND, RECT},
    Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject,
        GetDC, GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER,
        BI_RGB, DIB_RGB_COLORS, SRCCOPY,
    },
    Win32::UI::WindowsAndMessaging::GetClientRect,
};

use tauri::{AppHandle, Manager};

/// Capture a screenshot of a webview window and return it as PNG bytes
#[cfg(target_os = "windows")]
pub fn capture_webview_screenshot(app: &AppHandle, label: &str) -> Result<Vec<u8>, String> {
    use tauri::WebviewWindow;

    let window: WebviewWindow = app
        .get_webview_window(label)
        .ok_or_else(|| format!("Webview '{}' not found", label))?;

    // Get the raw window handle using Tauri's built-in method
    let raw_handle = window.hwnd()
        .map_err(|e| format!("Failed to get HWND: {}", e))?;

    let hwnd = HWND(raw_handle.0 as *mut _);

    unsafe {
        // Get window dimensions
        let mut rect = RECT::default();
        GetClientRect(hwnd, &mut rect)
            .map_err(|e| format!("GetClientRect failed: {}", e))?;

        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;

        if width <= 0 || height <= 0 {
            return Err(format!("Invalid window dimensions: {}x{}", width, height));
        }

        eprintln!("[Screenshot] Capturing {}x{} from webview '{}'", width, height, label);

        // Get device context
        let hdc_window = GetDC(hwnd);
        if hdc_window.is_invalid() {
            return Err("GetDC failed".to_string());
        }

        // Create compatible DC and bitmap
        let hdc_mem = CreateCompatibleDC(hdc_window);
        if hdc_mem.is_invalid() {
            ReleaseDC(hwnd, hdc_window);
            return Err("CreateCompatibleDC failed".to_string());
        }

        let hbitmap = CreateCompatibleBitmap(hdc_window, width, height);
        if hbitmap.is_invalid() {
            let _ = DeleteDC(hdc_mem);
            ReleaseDC(hwnd, hdc_window);
            return Err("CreateCompatibleBitmap failed".to_string());
        }

        // Select bitmap into DC
        let old_bitmap = SelectObject(hdc_mem, hbitmap);

        // Copy screen content to bitmap
        BitBlt(hdc_mem, 0, 0, width, height, hdc_window, 0, 0, SRCCOPY)
            .map_err(|e| format!("BitBlt failed: {}", e))?;

        // Prepare bitmap info for GetDIBits
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // Negative for top-down DIB
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0 as u32,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [Default::default()],
        };

        // Allocate buffer for pixel data (BGRA format)
        let row_size = ((width * 4 + 3) / 4) * 4; // 4-byte aligned
        let mut pixels: Vec<u8> = vec![0; (row_size * height) as usize];

        // Get the bitmap bits
        let lines = GetDIBits(
            hdc_mem,
            hbitmap,
            0,
            height as u32,
            Some(pixels.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        );

        // Clean up GDI objects
        SelectObject(hdc_mem, old_bitmap);
        let _ = DeleteObject(hbitmap);
        let _ = DeleteDC(hdc_mem);
        ReleaseDC(hwnd, hdc_window);

        if lines == 0 {
            return Err("GetDIBits failed".to_string());
        }

        // Convert BGRA to RGBA
        for chunk in pixels.chunks_exact_mut(4) {
            chunk.swap(0, 2); // Swap B and R
        }

        // Create image and encode to PNG
        let img = image::RgbaImage::from_raw(width as u32, height as u32, pixels)
            .ok_or("Failed to create image from pixels")?;

        let mut png_bytes: Vec<u8> = Vec::new();
        use image::ImageEncoder;
        let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
        encoder
            .write_image(
                img.as_raw(),
                width as u32,
                height as u32,
                image::ColorType::Rgba8,
            )
            .map_err(|e| format!("PNG encoding failed: {}", e))?;

        eprintln!("[Screenshot] Captured {} bytes PNG", png_bytes.len());
        Ok(png_bytes)
    }
}

/// Fallback for non-Windows platforms (not implemented)
#[cfg(not(target_os = "windows"))]
pub fn capture_webview_screenshot(_app: &AppHandle, _label: &str) -> Result<Vec<u8>, String> {
    Err("Screenshot capture is only supported on Windows".to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_module_compiles() {
        // Just verify the module compiles
        assert!(true);
    }
}
