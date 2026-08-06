#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewTab {
    #[default]
    Monitor,
    Spectrum,
    Spectrogram,
}

impl ViewTab {
    pub fn label(self) -> &'static str {
        match self {
            Self::Monitor => "Monitor",
            Self::Spectrum => "Spectrum",
            Self::Spectrogram => "Spectrogram",
        }
    }
}
