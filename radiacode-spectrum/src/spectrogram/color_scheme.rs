use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub enum ColorScheme {
    #[default]
    Viridis,
    Inferno,
    Turbo,
}

impl ColorScheme {
    pub const ALL: [Self; 3] = [Self::Viridis, Self::Inferno, Self::Turbo];

    pub fn label(self) -> &'static str {
        match self {
            Self::Viridis => "Viridis",
            Self::Inferno => "Inferno",
            Self::Turbo => "Turbo",
        }
    }

    fn from_persisted(value: &str) -> Self {
        match value {
            "Inferno" => Self::Inferno,
            "Turbo" => Self::Turbo,
            _ => Self::Viridis,
        }
    }
}

impl<'de> Deserialize<'de> for ColorScheme {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(Self::from_persisted(&value))
    }
}
