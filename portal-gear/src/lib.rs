pub mod hooks;

use hudhook::hooks::dx11::ImguiDx11Hooks;
use hudhook::windows::Win32::System::Threading::GetCurrentProcess;
//use hudhook::windows::Win32::System::Console::{GetStdHandle, STD_OUTPUT_HANDLE, WriteConsoleA};
use hudhook::*;
use neohook::MidHook;
use neohook::registry::unhook_all;
use std::ffi::c_void;
use std::fmt;
use std::net::Shutdown::Write;
use std::sync::Mutex;
use hudhook::tracing::*;
use imgui::Key;
use std::{usize};

use crate::hooks::{init_hooks};



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
#[repr(C)]
struct Quaternion {
    x:f32,
    y:f32,
    z:f32,
    w:f32
}



#[derive(Default, Clone, Copy)]
#[repr(C)]
struct StateInfo{
    pad0: [u32;0x20],
    position: FVec3, //0x80
    pad8C: [u8;4],
    rotation: Quaternion, //0x90
    padA0: [u16;0x18],
    speed: FVec3 //0xD0
}

#[derive(Default, Clone)]
pub struct SavestateData {
    saveSlots : [StateInfo;10],
    currentSaveSlot : usize,
    currentInfo : StateInfo,
    posBase : usize,
    camBase : usize,
    hookRegistry : Vec<*const MidHook>
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
        
        let mut data = SavestateData::default();
        data.hookRegistry = init_hooks().expect("Hook Failed!");
        data
    }
    pub fn updateCurrent(&mut self) -> Option<()> {
        let mut posBaseData = POS_BASE.lock().unwrap();
        if *posBaseData == 0 {            
            return None;
        }
        let pos_addr = *posBaseData as *const StateInfo; 
        unsafe {
            self.currentInfo = std::ptr::read(pos_addr);
        }
        //deal with the camera eventually
        Some(())
    }
    pub fn saveInfoToSlot(&mut self){
        let mut posBaseData = POS_BASE.lock().unwrap();
        if *posBaseData == 0 {            
            return;
        }
        println!("[PORTAL GEAR] Saving to slot {}", self.currentSaveSlot+1);
        self.saveSlots[self.currentSaveSlot] = self.currentInfo;    
    }
    pub fn loadInfoFromSlot(self) -> Option<()> {
        
        let mut posBaseData = POS_BASE.lock().unwrap();
        if *posBaseData == 0 {            
            return Some(())
        }
        
        let slot_info: StateInfo = self.saveSlots[self.currentSaveSlot];
        if slot_info.position.x == 0.0 && slot_info.position.y == 0.0 && slot_info.position.z == 0.0 {
            return Some(())
        }
        println!("[PORTAL GEAR] Loading from slot {}", self.currentSaveSlot+1);
        let info_addr = *posBaseData as *const StateInfo;
        
        unsafe {
            let mut info = *info_addr;
            info.position = slot_info.position;
            info.rotation = slot_info.rotation;
        }
        Some(())
    }
    fn unhook_all(&mut self){
        for midhook in self.hookRegistry.clone() {
            unsafe { midhook.read().unhook().expect("Unhook Failed!"); }
        }
        hudhook::eject();
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
        
        
        let mrl : Self = MainRenderLoop{stateData: SavestateData::new(), isVisible: true};
        println!("[PORTAL GEAR] Successfully initialized!");
        mrl
    }

    fn toggle_visible(&mut self){
        self.isVisible = !self.isVisible;
    }

    fn increment_save_slot(&mut self){
        match self.stateData.currentSaveSlot {
            0..=8 => self.stateData.currentSaveSlot += 1,
            9 => self.stateData.currentSaveSlot = 0,
            _ => ()
        }
    }

    fn decrement_save_slot(&mut self){
        match self.stateData.currentSaveSlot {
            1..=9 => self.stateData.currentSaveSlot -= 1,
            0 => self.stateData.currentSaveSlot = 9,
            _ => ()
        }
    }
}

impl ImguiRenderLoop for MainRenderLoop {

    

    fn render(&mut self, ui: &mut imgui::Ui) {

        self.stateData.updateCurrent();
        
        //ctrl+F4 uninjects the overlay
        if ui.io().key_ctrl {
            if ui.is_key_pressed(Key::F4) {
                self.stateData.unhook_all();
            }
        }
        //F1 toggles
        if ui.is_key_pressed(Key::F1) {
            self.toggle_visible();
        }
        //F9 saves and F10 loads
        if ui.is_key_pressed(Key::F9) {
            self.stateData.saveInfoToSlot();
        } else if ui.is_key_pressed(Key::F10){
            self.stateData.clone().loadInfoFromSlot();
        }
        //Up/Down arrow change slot

        if ui.is_key_pressed(Key::UpArrow) {
           self.increment_save_slot();
        } else if ui.is_key_pressed(Key::DownArrow) {
           self.decrement_save_slot();
        }


        if self.isVisible {
            ui.window("Portal Gear v2")
                .position([10., 10.], imgui::Condition::FirstUseEver)
                .size([320., 180.], imgui::Condition::FirstUseEver)
                .build(|| {
                    ui.text(format!("{}", self.stateData));
                    ui.new_line();
                    if(ui.button("Save State")){
                        self.stateData.saveInfoToSlot();
                    }
                    ui.same_line();
                    if(ui.button("Load State")){
                        self.stateData.clone().loadInfoFromSlot();
                    }
                    ui.text("Slot Controls: ");
                    ui.same_line();
                    if ui.button("-") {
                        self.decrement_save_slot();
                    }
                    ui.same_line();
                    if ui.button("+") {
                        self.increment_save_slot();
                    }
                    if ui.button("Exit Portal Gear") {
                        self.stateData.unhook_all();
                    }
                });
        }
    }
}


hudhook::hudhook!(ImguiDx11Hooks, MainRenderLoop::new());