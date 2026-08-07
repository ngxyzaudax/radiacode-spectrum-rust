mod compare;
mod selection;
mod spectrum;
mod state;
mod ui_controls;
mod ui_plot;
mod ui_plot_bars;
mod ui_plot_geometry;
mod ui_plot_legend;
mod ui_plot_values;
mod ui_role_cards;
mod ui_view;

pub use state::AnalysisState;
pub use ui_controls::{draw_analysis_controls, AnalysisAction};
pub use ui_view::draw_analysis_view;
