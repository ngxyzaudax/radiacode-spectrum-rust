#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    GetStatus = 0x0005,
    SetExchange = 0x0007,
    GetVersion = 0x000A,
    GetSerial = 0x000B,
    FwSignature = 0x0101,
    RdVirtSfr = 0x0824,
    WrVirtSfr = 0x0825,
    RdVirtString = 0x0826,
    WrVirtString = 0x0827,
    RdVirtSfrBatch = 0x082A,
    WrVirtSfrBatch = 0x082B,
    SetTime = 0x0A04,
}

impl From<Command> for u16 {
    fn from(value: Command) -> Self {
        value as u16
    }
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtString {
    Configuration = 2,
    SerialNumber = 8,
    TextMessage = 0x0F,
    DataBuf = 0x100,
    SfrFile = 0x101,
    Spectrum = 0x200,
    EnergyCalib = 0x202,
    SpecAccum = 0x205,
}

impl From<VirtString> for u32 {
    fn from(value: VirtString) -> Self {
        value as u32
    }
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtSfr {
    DeviceLang = 0x0502,
    DeviceOn = 0x0503,
    DeviceTime = 0x0504,
    DispBrt = 0x0511,
    DispOffTime = 0x0513,
    DispDir = 0x0515,
    SoundOn = 0x0522,
    VibroOn = 0x0531,
    DoseReset = 0x8007,
    DrLev1UrH = 0x8000,
    DrLev2UrH = 0x8001,
    DsUnits = 0x8004,
    CrLev1Cp10s = 0x8008,
    CrLev2Cp10s = 0x8009,
    CrUnits = 0x8013,
    Cps = 0x8020,
    DrUrH = 0x8021,
    TempDegC = 0x8024,
}

impl From<VirtSfr> for u32 {
    fn from(value: VirtSfr) -> Self {
        value as u32
    }
}
