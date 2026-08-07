#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewTab {
    #[default]
    Monitor,
    Spectrum,
    Spectrogram,
    Dosimeter,
    Settings,
}

impl ViewTab {
    pub fn label(self) -> &'static str {
        match self {
            Self::Monitor => "Monitor",
            Self::Spectrum => "Spectrum",
            Self::Spectrogram => "Spectrogram",
            Self::Dosimeter => "Dosimeter",
            Self::Settings => "Settings",
        }
    }
}
