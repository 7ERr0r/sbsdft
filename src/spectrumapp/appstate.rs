#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AppState {
    Uninit,
    Initializing,
    InitializingMem,
    InitRayon,
    AwaitingRayonThreads,
    AwaitingLastWorker,
    RayonBuildDone,
    StartAudioCapture,
    StartAudioCaptureAsync,
    WaitingForUserAudio,
    PreparingWasmWorker,
    MediaStreamTrack,

    GetUserMediaFailed,
    Playing,
}

static mut APP_STATE: AppState = AppState::Uninit;

pub fn get_app_state() -> AppState {
    unsafe { APP_STATE }
}
pub fn set_app_state(new_state: AppState) {
    unsafe {
        APP_STATE = new_state;
    }
}
