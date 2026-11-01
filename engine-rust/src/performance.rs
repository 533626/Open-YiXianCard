//! Opt-in diagnostic timers. No counters enter battle state or RNG.
//! Nested intervals are inclusive; only disjoint categories may be summed.
use std::cell::{Cell, RefCell};
use std::sync::atomic::{compiler_fence, Ordering};

pub const CATEGORIES: [&str; 14] = [
    "initialization",
    "battle_loop",
    "card_body",
    "before_card",
    "after_card_regular",
    "turn_start",
    "turn_end",
    "preview",
    "prepare_card",
    "completed_hooks",
    "begin_invocation",
    "finish_card",
    "after_card",
    "end_invocation",
];
pub const CATEGORY_COUNT: usize = CATEGORIES.len();
#[derive(Clone, Copy, Default, serde::Serialize)]
pub struct Sample {
    pub ticks: [u64; CATEGORY_COUNT],
    pub calls: [u64; CATEGORY_COUNT],
    pub migrations: u64,
    pub preview_paths: [u64; 3],
}
thread_local! {
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
    static SAMPLE: RefCell<Sample> = RefCell::new(Sample::default());
    static DEPTH: RefCell<[u32; CATEGORY_COUNT]> = const { RefCell::new([0; CATEGORY_COUNT]) };
}

pub fn begin() {
    SAMPLE.with(|s| *s.borrow_mut() = Sample::default());
    ACTIVE.with(|a| a.set(true));
}
pub fn finish() -> Sample {
    ACTIVE.with(|a| a.set(false));
    SAMPLE.with(|s| *s.borrow())
}
pub fn preview_path(index: usize) {
    if ACTIVE.with(|a| a.get()) {
        SAMPLE.with(|s| s.borrow_mut().preview_paths[index] += 1);
    }
}
pub struct Timer {
    category: usize,
    start: Option<(u64, u32)>,
    active: bool,
}

/// RDTSCP is calibrated against the system clock on the pinned probe thread.
/// It is never used for gameplay, deadlines or a process-global wall clock.
#[inline]
pub fn ticks() -> (u64, u32) {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        compiler_fence(Ordering::SeqCst);
        core::arch::x86_64::_mm_lfence();
        let mut cpu = 0;
        let counter = core::arch::x86_64::__rdtscp(&mut cpu);
        core::arch::x86_64::_mm_lfence();
        compiler_fence(Ordering::SeqCst);
        (counter, cpu)
    }
    #[cfg(not(target_arch = "x86_64"))]
    panic!("perf-ceiling currently requires x86_64 RDTSCP");
}
impl Timer {
    #[inline]
    pub fn new(category: usize) -> Self {
        let active = ACTIVE.with(|a| a.get());
        let outer = active
            && DEPTH.with(|d| {
                let mut d = d.borrow_mut();
                d[category] += 1;
                d[category] == 1
            });
        Self {
            category,
            start: outer.then(ticks),
            active,
        }
    }
}
impl Drop for Timer {
    #[inline]
    fn drop(&mut self) {
        if self.active {
            DEPTH.with(|d| d.borrow_mut()[self.category] -= 1);
            SAMPLE.with(|s| s.borrow_mut().calls[self.category] += 1);
        }
        if let Some((start, cpu)) = self.start {
            let (end, final_cpu) = ticks();
            SAMPLE.with(|s| {
                let mut s = s.borrow_mut();
                if cpu != final_cpu || end < start {
                    s.migrations += 1;
                } else {
                    s.ticks[self.category] += end - start;
                }
            });
        }
    }
}
