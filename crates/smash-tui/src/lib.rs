pub mod backend;
pub mod cell;
pub mod error;
pub mod pane;
pub mod renderer;
pub mod screen;
pub mod style;
pub mod theme;
pub mod viewport;

pub use backend::{MockBackend, TerminalBackend};
pub use cell::Cell;
pub use error::TuiError;
pub use pane::{PaneId, PaneTree, Rect, SplitDirection};
pub use renderer::{GutterDiagnostic, Renderer};
pub use screen::Screen;
pub use style::{Attributes, Color, Style};
pub use theme::{default_dark_theme, Theme};
pub use viewport::Viewport;
