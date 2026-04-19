
use std::sync::{Arc, atomic::{AtomicBool, AtomicU32, Ordering}};

// Thanks to this kind soul:
// https://github.com/rust-lang/rust/issues/72353#issuecomment-1093729062
struct AtomicF32 {
    storage: AtomicU32
}

impl AtomicF32 {
    pub fn new(value: f32) -> Self {
        let as_u32 = value.to_bits();
        Self { storage: AtomicU32::new(as_u32) }
    }

    pub fn store(&self, value: f32, order: Ordering) {
        let as_u32 = value.to_bits();
        self.storage.store(as_u32, order);
    }

    pub fn load(&self, order: Ordering) -> f32 {
        let as_u32 = self.storage.load(order);
        f32::from_bits(as_u32)
    }
}

pub struct StabilizerState {
    current_angle: AtomicF32,
    anchor_angle: AtomicF32,
    threshold: AtomicF32,
    freezed: AtomicBool,
}

impl StabilizerState {
    pub fn default() -> Self {
        StabilizerState { 
            current_angle: AtomicF32::new(0_f32), 
            anchor_angle: AtomicF32::new(0_f32), 
            threshold: AtomicF32::new(0_f32),
            freezed: AtomicBool::new(false)
        }
    }

    pub fn toggle_freeze(&self) {
        self.freezed.fetch_not(Ordering::Release);
    }

    pub fn set_anchor(&self) {
        let new_anchor = self.current_angle.load(Ordering::Acquire);
        self.anchor_angle.store(new_anchor, Ordering::Release);
    }

    pub fn set_current_angle(&self, angle: f32) {
        self.current_angle.store(angle, Ordering::Release);
    }

    pub fn set_threshold(&self, threshold: f32) {
        self.threshold.store(threshold, Ordering::Release);
    }

    pub fn get_current_angle(&self) -> f32 {
        self.current_angle.load(Ordering::Acquire)
    }

    pub fn get_anchor_angle(&self) -> f32 {
        self.anchor_angle.load(Ordering::Acquire)
    }

    pub fn get_threshold(&self) -> f32 {
        self.threshold.load(Ordering::Acquire)
    }

    pub fn get_freezed(&self) -> bool {
        self.freezed.load(Ordering::Acquire)
    }
}

#[derive(Clone)]
pub struct SharedState(Arc<StabilizerState>);

impl SharedState {

    pub fn new(state: StabilizerState) -> Self {
        SharedState(Arc::new(state))
    }

    pub fn set_current_angle(&self, angle: f32) { 
        self.0.set_current_angle(angle);
    }

    pub fn set_threshold(&self, threshold: f32) {
        self.0.set_threshold(threshold);
    }

    pub fn set_anchor(&self) {
        self.0.set_anchor();
    }

    pub fn toggle_freeze(&self) {
        self.0.toggle_freeze();
    }

    pub fn snapshot(&self) -> (f32, f32, f32, bool) {
        let current_angle = self.0.get_current_angle();
        let anchor_angle = self.0.get_anchor_angle();
        let threshold = self.0.get_threshold();
        let freezed = self.0.get_freezed();
        (current_angle, anchor_angle, threshold, freezed)
    }
}
