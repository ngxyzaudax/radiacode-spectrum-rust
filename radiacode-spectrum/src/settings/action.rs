#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsAction {
    LoadDevice,
    ConfirmLoad,
    CancelLoad,
    SaveDevice,
    DiscardChanges,
    SyncClock,
    AppChanged,
    SpectrogramChanged,
}
