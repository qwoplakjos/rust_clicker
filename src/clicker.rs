use std::sync::atomic::{AtomicBool, AtomicUsize, AtomicU32, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use rand::Rng;

#[cfg(target_os = "windows")]
use winapi::um::winuser::*;
#[cfg(target_os = "windows")]
use winapi::shared::minwindef::*;
#[cfg(target_os = "windows")]
use std::ffi::OsString;
#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStringExt;
#[cfg(target_os = "windows")]
use std::mem::zeroed;

#[cfg(target_os = "linux")]
use x11::xlib;
#[cfg(target_os = "linux")]
use libxdo_sys::{xdo_new, xdo_mouse_down, xdo_mouse_up, xdo_free};
#[cfg(target_os = "linux")]
use std::ffi::CString;
#[cfg(target_os = "linux")]
use x11::xlib::{XEvent, KeyPress, KeyRelease};

#[derive(PartialEq, Clone, Debug)]
pub enum ClickMode {
    Left,
    Right,
    Both,
}

impl ClickMode {
    pub fn to_usize(&self) -> usize {
        match self {
            ClickMode::Left => 0,
            ClickMode::Right => 1,
            ClickMode::Both => 2,
        }
    }

    pub fn from_usize(value: usize) -> Self {
        match value {
            0 => ClickMode::Left,
            1 => ClickMode::Right,
            2 => ClickMode::Both,
            _ => ClickMode::Left,
        }
    }
}

pub struct AutoClicker {
    min_cps: Arc<AtomicU32>,
    max_cps: Arc<AtomicU32>,
    is_running: Arc<AtomicBool>,
    click_mode: Arc<AtomicUsize>,
    left_pressed: Arc<AtomicBool>,
    right_pressed: Arc<AtomicBool>,
    window_title: String,
}

impl Default for AutoClicker {
    fn default() -> Self {
        Self {
            min_cps: Arc::new(AtomicU32::new(5)),
            max_cps: Arc::new(AtomicU32::new(25)),
            is_running: Arc::new(AtomicBool::new(false)),
            click_mode: Arc::new(AtomicUsize::new(0)),
            left_pressed: Arc::new(AtomicBool::new(false)),
            right_pressed: Arc::new(AtomicBool::new(false)),
            window_title: "Auto Clicker".to_string(),
        }
    }
}

// Global state for the hooks
#[cfg(target_os = "windows")]
static mut LEFT_PRESSED: Option<Arc<AtomicBool>> = None;
#[cfg(target_os = "windows")]
static mut RIGHT_PRESSED: Option<Arc<AtomicBool>> = None;
#[cfg(target_os = "windows")]
static mut IS_RUNNING: Option<Arc<AtomicBool>> = None;

#[cfg(target_os = "windows")]
unsafe extern "system" fn mouse_hook_proc(n_code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    if n_code >= 0 {
        let mouse_hook_struct = *(l_param as *const MSLLHOOKSTRUCT);
        
        // Check if the event was injected
        let is_injected = (mouse_hook_struct.flags & LLMHF_INJECTED) != 0;
        
        if !is_injected {
            if let Some(left_pressed) = LEFT_PRESSED.as_ref() {
                if let Some(right_pressed) = RIGHT_PRESSED.as_ref() {
                    match w_param as u32 {
                        WM_LBUTTONDOWN | WM_NCLBUTTONDOWN => {
                            left_pressed.store(true, Ordering::Relaxed);
                        }
                        WM_LBUTTONUP | WM_NCLBUTTONUP => {
                            left_pressed.store(false, Ordering::Relaxed);
                        }
                        WM_RBUTTONDOWN | WM_NCRBUTTONDOWN => {
                            right_pressed.store(true, Ordering::Relaxed);
                        }
                        WM_RBUTTONUP | WM_NCRBUTTONUP => {
                            right_pressed.store(false, Ordering::Relaxed);
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    CallNextHookEx(std::ptr::null_mut(), n_code, w_param, l_param)
}

static mut LAST_KEY_STATE: bool = false;

#[cfg(target_os = "windows")]
unsafe extern "system" fn keyboard_hook_proc(n_code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    if n_code >= 0 {
        let keyboard_hook_struct = *(l_param as *const KBDLLHOOKSTRUCT);
        let vk_code = keyboard_hook_struct.vkCode;


        if vk_code == VK_F6 as u32 {
            match w_param as u32 {
                WM_KEYDOWN => {
                    if !LAST_KEY_STATE {
                        LAST_KEY_STATE = true;
                    }
                },
                WM_KEYUP => {
                    if LAST_KEY_STATE {
                      
                        if let Some(is_running) = IS_RUNNING.as_ref() {
                            let current = is_running.load(Ordering::Relaxed);
                         
                            is_running.store(!current, Ordering::Relaxed);
                        } else {

                        }
                        LAST_KEY_STATE = false;
                    }
                },
                _ => {}
            }
        }
    }

    CallNextHookEx(std::ptr::null_mut(), n_code, w_param, l_param)
}

#[cfg(target_os = "linux")]
fn get_active_window_title() -> String {
    unsafe {
        let display = x11::xlib::XOpenDisplay(std::ptr::null());
        if display.is_null() {
            return String::new();
        }

        let mut window: x11::xlib::Window = 0;
        let mut revert: i32 = 0;
        x11::xlib::XGetInputFocus(display, &mut window, &mut revert);
        
        let mut name: *mut i8 = std::ptr::null_mut();
        x11::xlib::XFetchName(display, window, &mut name);
        
        x11::xlib::XCloseDisplay(display);
        
        if !name.is_null() {
            let title = CString::from_raw(name).into_string().unwrap_or_default();
            title
        } else {
            String::new()
        }
    }
}

#[cfg(target_os = "linux")]
fn get_active_window() -> x11::xlib::Window {
    unsafe {
        let display = x11::xlib::XOpenDisplay(std::ptr::null());
        if display.is_null() {
            return 0;
        }

        let mut window: x11::xlib::Window = 0;
        let mut revert: i32 = 0;
        x11::xlib::XGetInputFocus(display, &mut window, &mut revert);
        
        x11::xlib::XCloseDisplay(display);
        window
    }
}

impl AutoClicker {
    pub fn new() -> Self {
        let auto_clicker = Self::default();
        
        // Start the clicker thread
        let is_running_clicker = auto_clicker.is_running.clone();
        let click_mode = auto_clicker.click_mode.clone();
        let min_cps = auto_clicker.min_cps.clone();
        let max_cps = auto_clicker.max_cps.clone();
        let left_pressed = auto_clicker.left_pressed.clone();
        let right_pressed = auto_clicker.right_pressed.clone();
        let window_title = auto_clicker.window_title.clone();
        
        #[cfg(target_os = "windows")]
        {
            // Set up the global state
            unsafe {
                LEFT_PRESSED = Some(left_pressed.clone());
                RIGHT_PRESSED = Some(right_pressed.clone());
                IS_RUNNING = Some(is_running_clicker.clone());
            }
            
            // Start mouse hook thread
            thread::spawn(move || {
                unsafe {
                    let hook = SetWindowsHookExW(
                        WH_MOUSE_LL,
                        Some(mouse_hook_proc),
                        std::ptr::null_mut(),
                        0,
                    );
                    
                    if hook.is_null() {
                        println!("Failed to set mouse hook");
                        return;
                    }
                    
                    // Keep the hook alive
                    let mut msg: MSG = zeroed();
                    while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) != 0 {
                        TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                    
                    UnhookWindowsHookEx(hook);
                }
            });
            
            // Start keyboard hook thread for F6
            thread::spawn(move || {
                unsafe {
                    let hook = SetWindowsHookExW(
                        WH_KEYBOARD_LL,
                        Some(keyboard_hook_proc),
                        std::ptr::null_mut(),
                        0,
                    );
                    
                    if hook.is_null() {
                        println!("Failed to set keyboard hook");
                        return;
                    }
                    
                    // Keep the hook alive
                    let mut msg: MSG = zeroed();
                    while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) != 0 {
                        TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                    
                    UnhookWindowsHookEx(hook);
                }
            });
        }
        
        #[cfg(target_os = "linux")]
        {
            // Start X11 event monitoring thread
            let left_pressed = left_pressed.clone();
            let right_pressed = right_pressed.clone();
            let is_running_event = is_running_clicker.clone();
            
            thread::spawn(move || {
                unsafe {
                    let display = x11::xlib::XOpenDisplay(std::ptr::null());
                    if display.is_null() {
                        println!("Failed to open X11 display");
                        return;
                    }

                    // Get the root window
                    let root = x11::xlib::XDefaultRootWindow(display);
                    
                    // Get the F6 keycode
                    let f6_keycode = x11::xlib::XKeysymToKeycode(display, x11::keysym::XK_F6 as u64);
                    
                    // Select input events for the root window
                    x11::xlib::XSelectInput(display, root, x11::xlib::KeyPressMask);
                    
                    let mut event: x11::xlib::XEvent = std::mem::zeroed();
                    
                    loop {
                        // Check mouse button states using XQueryPointer
                        let mut root_return: x11::xlib::Window = 0;
                        let mut child_return: x11::xlib::Window = 0;
                        let mut root_x_return: i32 = 0;
                        let mut root_y_return: i32 = 0;
                        let mut win_x_return: i32 = 0;
                        let mut win_y_return: i32 = 0;
                        let mut mask_return: u32 = 0;
                        
                        let result = x11::xlib::XQueryPointer(
                            display,
                            root,
                            &mut root_return,
                            &mut child_return,
                            &mut root_x_return,
                            &mut root_y_return,
                            &mut win_x_return,
                            &mut win_y_return,
                            &mut mask_return
                        );
                        
                        if result != 0 {
                            // Check for left mouse button (Button1Mask)
                            let left_state = (mask_return & x11::xlib::Button1Mask) != 0;
                            left_pressed.store(left_state, Ordering::Relaxed);
                            
                            // Check for right mouse button (Button3Mask)
                            let right_state = (mask_return & x11::xlib::Button3Mask) != 0;
                            right_pressed.store(right_state, Ordering::Relaxed);


                        } else {
                            println!("XQueryPointer failed");
                        }
                        
                        // Check for F6 key press without blocking
                        while x11::xlib::XPending(display) > 0 {
                            x11::xlib::XNextEvent(display, &mut event);
                            if event.get_type() == x11::xlib::KeyPress {
                                let key_event = event.key;
                                if key_event.keycode == f6_keycode as u32 {
                                    let current = is_running_event.load(Ordering::Relaxed);
                                    is_running_event.store(!current, Ordering::Relaxed);
                                    
                                    // Reset button states when toggling
                                    if !is_running_event.load(Ordering::Relaxed) {
                                        left_pressed.store(false, Ordering::Relaxed);
                                        right_pressed.store(false, Ordering::Relaxed);
                                    }
                                }
                            }
                        }
                        
                        // Small delay to prevent high CPU usage
                        thread::sleep(Duration::from_millis(10));
                    }
                }
            });
        }
        
        // Start clicker thread
        let min_cps_clone = auto_clicker.min_cps.clone();
        let max_cps_clone = auto_clicker.max_cps.clone();
        thread::spawn(move || {
            let mut rng = rand::thread_rng();
            
            loop {
                if is_running_clicker.load(Ordering::Relaxed) {
                    // Check if foreground window is not our application
                    let current_title = {
                        #[cfg(target_os = "windows")]
                        unsafe {
                            let hwnd = GetForegroundWindow();
                            let mut title = [0u16; 512];
                            let len = GetWindowTextW(hwnd, title.as_mut_ptr(), title.len() as i32);
                            if len > 0 {
                                OsString::from_wide(&title[..len as usize]).to_string_lossy().into_owned()
                            } else {
                                String::new()
                            }
                        }
                        
                        #[cfg(target_os = "linux")]
                        get_active_window_title()
                    };
                    
                    if current_title != window_title {
                        let left_state = left_pressed.load(Ordering::Relaxed);
                        let right_state = right_pressed.load(Ordering::Relaxed);
                        let current_mode = ClickMode::from_usize(click_mode.load(Ordering::Relaxed));
                        
                        match current_mode {
                            ClickMode::Left => {
                                if left_state {
                                    #[cfg(target_os = "windows")]
                                    unsafe {
                                        mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0, 0);
                                        thread::sleep(Duration::from_millis(1));
                                        mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0, 0);
                                    }
                                    #[cfg(target_os = "linux")]
                                    unsafe {
                                        let target_window = get_active_window();
                                        if target_window != 0 {
                                            let xdo = xdo_new(std::ptr::null());
                                            if !xdo.is_null() {
                                                xdo_mouse_down(xdo, target_window, 1);
                                                thread::sleep(Duration::from_millis(1));
                                                xdo_mouse_up(xdo, target_window, 1);
                                                xdo_free(xdo);
                                            }
                                        }
                                    }
                                }
                            }
                            ClickMode::Right => {
                                if right_state {
                                    #[cfg(target_os = "windows")]
                                    unsafe {
                                        mouse_event(MOUSEEVENTF_RIGHTDOWN, 0, 0, 0, 0);
                                        thread::sleep(Duration::from_millis(1));
                                        mouse_event(MOUSEEVENTF_RIGHTUP, 0, 0, 0, 0);
                                    }
                                    #[cfg(target_os = "linux")]
                                    unsafe {
                                        let target_window = get_active_window();
                                        if target_window != 0 {
                                            let xdo = xdo_new(std::ptr::null());
                                            if !xdo.is_null() {
                                                xdo_mouse_down(xdo, target_window, 3);
                                                thread::sleep(Duration::from_millis(1));
                                                xdo_mouse_up(xdo, target_window, 3);
                                                xdo_free(xdo);
                                            }
                                        }
                                    }
                                }
                            }
                            ClickMode::Both => {
                                if left_state {
                                    #[cfg(target_os = "windows")]
                                    unsafe {
                                        mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0, 0);
                                        thread::sleep(Duration::from_millis(1));
                                        mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0, 0);
                                    }
                                    #[cfg(target_os = "linux")]
                                    unsafe {
                                        let target_window = get_active_window();
                                        if target_window != 0 {
                                            let xdo = xdo_new(std::ptr::null());
                                            if !xdo.is_null() {
                                                xdo_mouse_down(xdo, target_window, 1);
                                                thread::sleep(Duration::from_millis(1));
                                                xdo_mouse_up(xdo, target_window, 1);
                                                xdo_free(xdo);
                                            }
                                        }
                                    }
                                }
                                if right_state {
                                    #[cfg(target_os = "windows")]
                                    unsafe {
                                        mouse_event(MOUSEEVENTF_RIGHTDOWN, 0, 0, 0, 0);
                                        thread::sleep(Duration::from_millis(1));
                                        mouse_event(MOUSEEVENTF_RIGHTUP, 0, 0, 0, 0);
                                    }
                                    #[cfg(target_os = "linux")]
                                    unsafe {
                                        let target_window = get_active_window();
                                        if target_window != 0 {
                                            let xdo = xdo_new(std::ptr::null());
                                            if !xdo.is_null() {
                                                xdo_mouse_down(xdo, target_window, 3);
                                                thread::sleep(Duration::from_millis(1));
                                                xdo_mouse_up(xdo, target_window, 3);
                                                xdo_free(xdo);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    // Load current min/max CPS values and calculate delay
                    let current_min = min_cps_clone.load(Ordering::SeqCst);
                    let current_max = max_cps_clone.load(Ordering::SeqCst);
                    
                    // Use the actual slider values for random CPS generation
                    let cps = rng.gen_range(current_min..=current_max);
                    
                    // Calculate total time for one click cycle (including click duration)
                    // 1000ms / CPS gives us the total time per click cycle
                    // Subtract 2ms for the click duration (1ms down + 1ms up)
                    let total_cycle_time = (1000.0 / cps as f32) as u64;
                    let delay_ms = total_cycle_time.saturating_sub(2);
                    
                    // Sleep for the calculated delay
                    thread::sleep(Duration::from_millis(delay_ms));
                } else {
                    thread::sleep(Duration::from_millis(10));
                }
            }
        });

        // Start the hotkey thread with its own clone of is_running
        let _is_running_hotkey = auto_clicker.is_running.clone();
        thread::spawn(move || {
            #[cfg(target_os = "windows")]
            unsafe {
                let hook = SetWindowsHookExW(
                    WH_KEYBOARD_LL,
                    Some(keyboard_hook_proc),
                    std::ptr::null_mut(),
                    0,
                );
                
                if hook.is_null() {
                    println!("Failed to set keyboard hook");
                    return;
                }
                
                // Keep the hook alive
                let mut msg: MSG = zeroed();
                while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) != 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
                
                UnhookWindowsHookEx(hook);
            }

            #[cfg(target_os = "linux")]
            unsafe {
                let display = x11::xlib::XOpenDisplay(std::ptr::null());
                if display.is_null() {
                    eprintln!("Failed to open X display");
                    return;
                }

                let f6_keycode = x11::xlib::XKeysymToKeycode(display, x11::keysym::XK_F6 as u64);
                let mut key_states = [0; 256];

                loop {
                    x11::xlib::XQueryKeymap(display, key_states.as_mut_ptr());
                    let f6_pressed = (key_states[f6_keycode as usize / 8] & (1 << (f6_keycode % 8))) != 0;
                    if f6_pressed {
                        _is_running_hotkey.store(!_is_running_hotkey.load(Ordering::Relaxed), Ordering::Relaxed);
                        thread::sleep(Duration::from_millis(200)); // Debounce
                    }
                    thread::sleep(Duration::from_millis(10));
                }
            }
        });

        auto_clicker
    }

    pub fn set_min_cps(&self, value: u32) {
        self.min_cps.store(value, Ordering::SeqCst);
    }

    pub fn set_max_cps(&self, value: u32) {
        self.max_cps.store(value, Ordering::SeqCst);
    }

    pub fn set_click_mode(&self, mode: ClickMode) {
        self.click_mode.store(mode.to_usize(), Ordering::Relaxed);
    }

    pub fn toggle_running(&self) {
        let current = self.is_running.load(Ordering::Relaxed);
        self.is_running.store(!current, Ordering::Relaxed);
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }
} 