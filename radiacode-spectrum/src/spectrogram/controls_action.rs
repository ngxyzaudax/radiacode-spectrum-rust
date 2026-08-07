use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpectrogramControlsAction {
    StartRecording,
    StopRecording,
    PauseCapture,
    ResumeCapture,
    ResumeRecording,
    CloseLoaded,
    Load(PathBuf),
    SettingsChanged,
    LibraryChanged,
}
