
#[derive(Clone)]
pub struct StabilizerState {
    pub current_angle: f32,
    pub anchor_angle: f32,
    pub threshold: f32,
    pub freezed: bool,
}

impl StabilizerState {
    pub fn default() -> Self {
        StabilizerState { current_angle: 0_f32, anchor_angle: 0_f32, threshold: 0_f32, freezed: false }
    }

    pub fn toggle_freeze(self: &mut Self) {
        self.freezed = !self.freezed;
    }

    pub fn set_anchor(self: &mut Self) {
        self.anchor_angle = self.current_angle;
    }
}

impl Copy for StabilizerState {}
