static mut TPS: u64 = 141_120_000;

#[doc(hidden)]
pub unsafe fn change_tps(to: u64) {
    unsafe { TPS = to }
}

pub fn tps() -> u64 {
    unsafe { TPS }
}
