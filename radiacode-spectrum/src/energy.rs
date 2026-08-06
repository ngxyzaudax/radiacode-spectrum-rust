use radiacode_core::channel_to_energy;

use crate::model::SpectrumView;

pub const ENERGY_MIN_KEV: f64 = 0.0;
pub const ENERGY_MAX_KEV: f64 = 3000.0;

pub struct EnergyGrid {
    pub indices: Vec<usize>,
    pub energies_kev: Vec<f64>,
}

pub fn energy_grid(spectrum: &SpectrumView) -> EnergyGrid {
    let mut indices = Vec::new();
    let mut energies_kev = Vec::new();
    for (channel, _) in spectrum.counts.iter().enumerate() {
        let energy = channel_to_energy(channel as u32, spectrum.a0, spectrum.a1, spectrum.a2) as f64;
        if (ENERGY_MIN_KEV..=ENERGY_MAX_KEV).contains(&energy) {
            indices.push(channel);
            energies_kev.push(energy);
        }
    }
    EnergyGrid {
        indices,
        energies_kev,
    }
}

pub fn bar_energy_width(energies: &[f64], index: usize, fallback: f64) -> f64 {
    let energy = energies[index];
    let left = if index > 0 {
        (energies[index - 1] + energy) / 2.0
    } else if energies.len() > 1 {
        energy - (energies[1] - energy) / 2.0
    } else {
        energy - fallback / 2.0
    };
    let right = if index + 1 < energies.len() {
        (energy + energies[index + 1]) / 2.0
    } else if index > 0 {
        energy + (energy - energies[index - 1]) / 2.0
    } else {
        energy + fallback / 2.0
    };
    (right - left).max(fallback * 0.5)
}

pub fn clamp_energy_range(min_x: f64, max_x: f64) -> (f64, f64) {
    let span = ENERGY_MAX_KEV - ENERGY_MIN_KEV;
    let width = (max_x - min_x).clamp(1.0, span);
    let mut min = min_x.clamp(ENERGY_MIN_KEV, ENERGY_MAX_KEV - width);
    let mut max = min + width;
    if max > ENERGY_MAX_KEV {
        max = ENERGY_MAX_KEV;
        min = max - width;
    }
    (min, max)
}

pub fn sample_indices(grid: &EnergyGrid, counts: &[u32]) -> Vec<u32> {
    grid.indices
        .iter()
        .map(|&index| counts.get(index).copied().unwrap_or(0))
        .collect()
}
