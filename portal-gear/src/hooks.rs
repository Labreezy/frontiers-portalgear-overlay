use neohook::{HookContext, MidHook, Pattern, get_module_handle, scan_module};
use crate::{CAM_BASE, POS_BASE};
use std::sync::{Mutex, MutexGuard, OnceLock};
unsafe extern "system" fn pos_handler(ctx: *mut HookContext){
    let ctx = &mut *ctx;
    let new_pos_base = ctx.rbx as usize;
    let mut data = POS_BASE.lock().unwrap();
    *data = new_pos_base;

}

unsafe extern "system" fn cam_handler(ctx: *mut HookContext){
    let ctx = &mut *ctx;
    let new_cam_base = ctx.rax as usize;
    let mut data = CAM_BASE.lock().unwrap();
    *data = new_cam_base;

}
#[derive(Debug)]
struct MidHookWrapper(MidHook);



unsafe impl Send for MidHookWrapper {}
unsafe impl Sync for MidHookWrapper {}

static mut POS_HOOK : Mutex<Option<MidHookWrapper>> = Mutex::<Option<MidHookWrapper>>::new(None);



pub fn init_hooks() -> Result<(), String> {
    let h = get_module_handle("SonicFrontiers.exe").unwrap();
    let position_pattern = Pattern::parse("0F 58 B3 80 00 00 00").unwrap();
    let cam_pattern = Pattern::parse("48 8B D8 0F 11 00 0F 10 4F").unwrap();
    
    if let Some(pos_addr) = unsafe { scan_module(h, &position_pattern)} {
        
        let pos_offset_addr: *const u8 = pos_addr.wrapping_sub(8);
        let pos_hook= unsafe {MidHook::install(pos_offset_addr, pos_handler) }.expect("Position Hook Failed");
        unsafe { POS_HOOK = Mutex::new(Some(MidHookWrapper(pos_hook))); }
        println!("Position hook installed at {pos_offset_addr:p}");
        
        
    }
    /*
    if let Some(cam_addr) = unsafe {scan_module(h, &cam_pattern)} {
            println!("Found camera pattern at {cam_addr:p}");
            let cam_offset_addr = cam_addr.wrapping_sub(3);
            let cam_hook = unsafe { MidHook::install(cam_offset_addr, cam_handler) }.expect("Camera Hook Failed");
            HOOK_REGISTRY.get().unwrap()[0].lock() = Mutex::new(Some(MidHookWrapper(cam_hook)));
            println!("Camera hook installed at {cam_offset_addr:p}");
    }*/
    Ok(())
}

pub fn unhook_all() -> Option<()> {
    if let Ok(mut guard) = unsafe { POS_HOOK.lock() } {
        if let Some(mhook) = guard.take() {
            drop(mhook.0);
        }
    }
    println!("Registry destructed, ejecting hudhook");
    hudhook::eject();
    Some(())
}