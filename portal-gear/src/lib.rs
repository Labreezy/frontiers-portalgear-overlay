pub mod hooks;

use hudhook::hooks::dx11::ImguiDx11Hooks;
use hudhook::*;
use neohook::MidHook;
use std::fmt;
use std::sync::Mutex;
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
    pad_0: [u32;0x20],
    position: FVec3, //0x80
    pad_8c: [u8;4],
    rotation: Quaternion, //0x90
    pad_a0: [u16;0x18],
    speed: FVec3 //0xD0
}

#[derive(Default, Clone)]
pub struct SavestateData {
    save_slots : [StateInfo;10],
    current_save_slot : usize,
    current_info : StateInfo,
    hook_registry : Vec<*const MidHook>
}

impl fmt::Display for SavestateData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Current Slot: {}\r\nPosition: {}\r\nSpeed: {}", self.current_save_slot+1, self.current_info.position, self.current_info.speed)
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
        data.hook_registry = init_hooks().expect("Hook Failed!");
        data
    }
    pub fn update(&mut self) -> Option<()> {
        let pos_base_ptrval = POS_BASE.lock().unwrap();
        if *pos_base_ptrval == 0 {            
            return None;
        }
        let pos_addr = *pos_base_ptrval as *const StateInfo; 
        unsafe {
            self.current_info = std::ptr::read(pos_addr);
        }
        //deal with the camera eventually
        Some(())
    }
    pub fn save_to_slot(&mut self){
        let pos_base_ptrval = POS_BASE.lock().unwrap();
        if *pos_base_ptrval == 0 {            
            return;
        }
        println!("[PORTAL GEAR] Saving to slot {}", self.current_save_slot+1);
        self.save_slots[self.current_save_slot] = self.current_info;    
    }
    pub fn load_from_slot(self) -> Option<()> {
        
        let pos_base_ptrval = POS_BASE.lock().unwrap();
        if *pos_base_ptrval == 0 {            
            return Some(())
        }
        
        let slot_info: StateInfo = self.save_slots[self.current_save_slot];
        if slot_info.position.x == 0.0 && slot_info.position.y == 0.0 && slot_info.position.z == 0.0 {
            return Some(())
        }
        println!("[PORTAL GEAR] Loading from slot {}", self.current_save_slot+1);
        let info_addr = unsafe {
            &mut *(*pos_base_ptrval as *mut StateInfo)
        };
        info_addr.position = slot_info.position;
        info_addr.rotation = slot_info.rotation;
    
        Some(())
    }
    
}



#[derive(Default)]
struct MainRenderLoop{

    state_data: SavestateData,
    is_visible: bool

}


impl MainRenderLoop {
    fn new() -> Self {
        println!("[PORTAL GEAR] Initializing");
        
        
        let mrl : Self = MainRenderLoop{state_data: SavestateData::new(), is_visible: true};
        println!("[PORTAL GEAR] Successfully initialized!");
        mrl
    }

    fn toggle_visible(&mut self){
        self.is_visible = !self.is_visible;
    }

    fn increment_save_slot(&mut self){
        match self.state_data.current_save_slot {
            0..=8 => self.state_data.current_save_slot += 1,
            9 => self.state_data.current_save_slot = 0,
            _ => ()
        }
    }

    fn decrement_save_slot(&mut self){
        match self.state_data.current_save_slot {
            1..=9 => self.state_data.current_save_slot -= 1,
            0 => self.state_data.current_save_slot = 9,
            _ => ()
        }
    }
}

impl ImguiRenderLoop for MainRenderLoop {

    

    fn render(&mut self, ui: &mut imgui::Ui) {

        self.state_data.update();
        
        
        //F1 toggles
        if ui.is_key_pressed(Key::F1) {
            self.toggle_visible();
        }
        //F9 saves and F10 loads
        if ui.is_key_pressed(Key::F9) {
            self.state_data.save_to_slot();
        } else if ui.is_key_pressed(Key::F10){
            self.state_data.clone().load_from_slot();
        }
        //Up/Down arrow changes slot

        if ui.is_key_pressed(Key::UpArrow) {
           self.increment_save_slot();
        } else if ui.is_key_pressed(Key::DownArrow) {
           self.decrement_save_slot();
        }


        if self.is_visible {
            ui.window("Portal Gear v2")
                .position([10., 10.], imgui::Condition::FirstUseEver)
                .size([320., 180.], imgui::Condition::FirstUseEver)
                .build(|| {
                    ui.text(format!("{}", self.state_data));
                    ui.new_line();
                    if ui.button("Save State") {
                        self.state_data.save_to_slot();
                    }
                    ui.same_line();
                    if ui.button("Load State") {
                        self.state_data.clone().load_from_slot();
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
                });
        }
    }
}


hudhook::hudhook!(ImguiDx11Hooks, MainRenderLoop::new());