use std::ptr;
use winapi::shared::minwindef::{LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::HWND;
use winapi::um::winuser::*;

const WM_COPYDATA: UINT = 0x004A;

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_COPYDATA {
        let data = lparam as *const COPYDATASTRUCT;
        let event_id = (*data).dwData as u32;
        let size = (*data).cbData as usize;
        let payload = std::slice::from_raw_parts((*data).lpData as *const u8, size);
        
        println!("\n=== Event {} ({} bytes) ===", event_id, size);
        println!("Hex: {:02X?}", payload);
        
        if event_id == 6 {
            let filename = format!("event6_working_{}.bin", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs());
            std::fs::write(&filename, payload).ok();
            println!("Saved to {}", filename);
        }
        
        return 1;
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

fn main() {
    unsafe {
        let class_name: Vec<u16> = "LAUNCHER_CLASS\0".encode_utf16().collect();
        let window_name: Vec<u16> = "LAUNCHER_WINDOW\0".encode_utf16().collect();
        
        let wc = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(window_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: ptr::null_mut(),
            hIcon: ptr::null_mut(),
            hCursor: ptr::null_mut(),
            hbrBackground: ptr::null_mut(),
            lpszMenuName: ptr::null_mut(),
            lpszClassName: class_name.as_ptr(),
        };
        
        RegisterClassW(&wc);
        
        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            window_name.as_ptr(),
            0,
            0, 0, 0, 0,
            HWND_MESSAGE,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
        );
        
        println!("Logger window created: {:?}", hwnd);
        println!("Waiting for messages from TERA...\n");
        
        let mut msg = std::mem::zeroed();
        while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
