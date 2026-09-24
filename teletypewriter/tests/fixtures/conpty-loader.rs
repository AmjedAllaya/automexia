//! Inert native ABI oracle: never starts a process or creates a pseudoconsole.
#![allow(non_snake_case, dead_code, unused_variables)]

#[repr(C)]
pub struct Coord {
    x: i16,
    y: i16,
}

#[cfg(dependency)]
#[no_mangle]
pub extern "system" fn FixtureDependency() -> u32 {
    7
}

#[cfg(needs_dependency)]
#[link(name = "conpty_fixture_dependency", kind = "raw-dylib")]
extern "system" {
    fn FixtureDependency() -> u32;
}

#[cfg(all(not(dependency), not(missing_create)))]
#[no_mangle]
pub unsafe extern "system" fn CreatePseudoConsole(
    _: Coord,
    input: *mut std::ffi::c_void,
    _: *mut std::ffi::c_void,
    _: u32,
    output: *mut isize,
) -> i32 {
    if cfg!(create_failure) {
        return 0x80070057u32 as i32;
    }
    #[cfg(needs_dependency)]
    if unsafe { FixtureDependency() } != 7 {
        return 0x80070057u32 as i32;
    }
    // SAFETY: the helper supplies initialized, exclusive output storage.
    unsafe { *output = input as isize };
    0
}

#[cfg(all(not(dependency), not(missing_resize)))]
#[no_mangle]
pub extern "system" fn ResizePseudoConsole(_: isize, _: Coord) -> i32 {
    0
}

#[cfg(all(not(dependency), not(missing_close)))]
#[no_mangle]
pub unsafe extern "system" fn ClosePseudoConsole(handle: isize) {
    // SAFETY: the helper retains its exclusive sentinel until Conpty::drop returns.
    unsafe { *(handle as *mut usize) = 0xC105E };
}
