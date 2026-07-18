pub mod hooks;

use hudhook::hooks::dx11::ImguiDx11Hooks;
use hudhook::windows::Win32::System::Threading::GetCurrentProcess;

use hudhook::windows::Win32::System::Diagnostics::Debug::{ReadProcessMemory, WriteProcessMemory};
//use hudhook::windows::Win32::System::Console::{GetStdHandle, STD_OUTPUT_HANDLE, WriteConsoleA};
use hudhook::*;
use neohook::registry::unhook_all;
use std::ffi::c_void;
use std::fmt;
use std::net::Shutdown::Write;
use std::sync::Mutex;
use hudhook::tracing::*;
use imgui::Key;
use std::{usize};

use crate::hooks::{HOOK_REGISTRY, init_hooks};



#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
struct FVec3 {
    x:f32,
    y:f32,
    z:f32
}

impl fmt::Display for FVec3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "X: {:.2} Y: {:.2} Z: {:.2}", self.x, self.y, self.z)
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct Quaternion {
    x:f32,
    y:f32,
    z:f32,
    w:f32
}



#[derive(Default, Clone, Copy)]
struct StateInfo{
    position: FVec3, //0x80
    rotation: Quaternion, //0x90
    speed: FVec3 //0xD0
}

#[derive(Default, Clone, Copy)]
pub struct SavestateData {
    saveSlots : [StateInfo;10],
    currentSaveSlot : usize,
    currentInfo : StateInfo,
    posBase : usize,
    camBase : usize,
}

impl fmt::Display for SavestateData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Current Slot: {}\r\nPosition: {}\r\nSpeed: {}", self.currentSaveSlot+1, self.currentInfo.position, self.currentInfo.speed)
    }
}

unsafe impl Send for SavestateData {}
unsafe impl Sync for SavestateData {}

pub static POS_BASE: Mutex<usize> = Mutex::new(0);
pub static CAM_BASE: Mutex<usize> = Mutex::new(0);




impl SavestateData {
    pub fn new() -> SavestateData
    {   
        let _ = init_hooks();
        SavestateData::default()
    }

    pub fn updateCurrent(&self) -> Option<StateInfo> {
        let mut posBaseData = POS_BASE.lock().unwrap();
        if *posBaseData == 0 {            
            return Some(StateInfo::default())
        }
        
        let mut pos: FVec3 = unsafe {std::mem::zeroed()};
        let mut spd: FVec3 = unsafe {std::mem::zeroed()};
        let mut rot: Quaternion = unsafe {std::mem::zeroed()};

    
            let pos_addr = *posBaseData as *const u8; 
            let pos_base = pos_addr.wrapping_add(0x80) as *const FVec3;
            let rot_base = pos_addr.wrapping_add(0x90) as *const Quaternion;
            let spd_base = pos_addr.wrapping_add(0xD0) as *const FVec3;
            unsafe {
                pos = *pos_base;
                rot = *rot_base;
                spd = *spd_base;
            }
        //deal with the camera eventually
        let info = StateInfo { position: pos, rotation: rot, speed: spd };
        
        Some(info)
    }
    pub fn saveInfoToSlot(&mut self){
        let mut posBaseData = POS_BASE.lock().unwrap();
        if *posBaseData == 0 {            
            return;
        }
        println!("[PORTAL GEAR] Saving to slot {}", self.currentSaveSlot+1);
        self.saveSlots[self.currentSaveSlot] = self.currentInfo;    
    }
    pub fn writeInfoFromSlot(self) -> Option<()> {
        
        let mut posBaseData = POS_BASE.lock().unwrap();
        if *posBaseData == 0 {            
            return Some(())
        }
        
        let info: StateInfo = self.saveSlots[self.currentSaveSlot];
        if info.position.x == 0.0 && info.position.y == 0.0 && info.position.z == 0.0 {
            return Some(())
        }
        println!("[PORTAL GEAR] Loading from slot {}", self.currentSaveSlot+1);
        let pos_addr = *posBaseData as *const u8; 
        let pos_base = pos_addr.wrapping_add(0x80) as *mut FVec3;
        let rot_base = pos_addr.wrapping_add(0x90) as *mut Quaternion;
        unsafe {
            core::ptr::write(pos_base, info.position);
            core::ptr::write(rot_base, info.rotation);
        }
        Some(())

    }

}



#[derive(Default)]
struct MainRenderLoop{

    stateData: SavestateData,
    isVisible: bool

}


impl MainRenderLoop {
    fn new() -> Self {
        println!("[PORTAL GEAR] Initializing");
        
        
        let mut mrl : Self = MainRenderLoop{stateData: SavestateData::new(), isVisible: true};
        
        unsafe {
            mrl.console_println(format!("{}", "[PORTAL GEAR] Successfully initialized!"));
        }
        mrl
    }


    unsafe fn console_println(&self, line: String)  {
        let mut line_nl = line + "\r\n";
        let line_nl_bytes = line_nl.as_bytes();
        print!("{}", line_nl);
        
        
    }
}

impl ImguiRenderLoop for MainRenderLoop {



    fn render(&mut self, ui: &mut imgui::Ui) {

        let info = self.stateData.updateCurrent().unwrap();
        self.stateData.currentInfo = info;
        
        //ctrl+F4 uninjects the overlay
        if ui.io().key_ctrl {
            if ui.is_key_pressed(Key::F4) {
                unhook_all();
                hudhook::eject();
            }
        }
        //F1 toggles
        if ui.is_key_pressed(Key::F1) {
            self.isVisible = !self.isVisible;
        }
        //F9 saves and F10 loads
        if ui.is_key_pressed(Key::F9) {
            self.stateData.saveInfoToSlot();
        } else if ui.is_key_pressed(Key::F10){
            self.stateData.writeInfoFromSlot();
        }
        //Up/Down arrow change slot

        if ui.is_key_pressed(Key::UpArrow) {
            match self.stateData.currentSaveSlot {
                0..=8 => self.stateData.currentSaveSlot += 1,
                9 => self.stateData.currentSaveSlot = 0,
                _ => ()
            }
        } else if ui.is_key_pressed(Key::DownArrow) {
            match self.stateData.currentSaveSlot {
                1..=9 => self.stateData.currentSaveSlot -= 1,
                0 => self.stateData.currentSaveSlot = 9,
                _ => ()
            }
        }


        if self.isVisible {
            ui.window("Portal Gear v2")
                .position([10., 10.], imgui::Condition::FirstUseEver)
                .size([320., 180.], imgui::Condition::FirstUseEver)
                .build(|| {
                    ui.text(format!("{}", self.stateData));
                });
        }
    }
}


hudhook::hudhook!(ImguiDx11Hooks, MainRenderLoop::new());