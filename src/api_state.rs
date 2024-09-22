use axum::extract::FromRef;
use synx::Synx;

#[derive(Clone)]
pub struct ApiState {
    pub synx: Synx,
}

impl FromRef<ApiState> for Synx {
    fn from_ref(app_state: &ApiState) -> Synx {
        app_state.synx.clone()
    }
}
