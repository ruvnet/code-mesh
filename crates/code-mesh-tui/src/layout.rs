use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
};

/// Layout manager for organizing UI components
#[derive(Debug, Clone)]
pub struct LayoutManager {
    /// Main content area
    pub main_area: Rect,
    /// Side panel area (optional)
    pub side_panel: Option<Rect>,
    /// Status bar area
    pub status_area: Rect,
    /// Input area
    pub input_area: Rect,
    /// Full terminal area
    pub terminal_area: Rect,
}

impl LayoutManager {
    /// Create a new layout manager for the given terminal area
    pub fn new(area: Rect) -> Self {
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),      // Main content
                Constraint::Length(1),   // Status bar
            ])
            .split(area);

        let content_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),      // Messages/content
                Constraint::Length(3),   // Input area
            ])
            .split(main_layout[0]);

        Self {
            main_area: content_layout[0],
            side_panel: None,
            status_area: main_layout[1],
            input_area: content_layout[1],
            terminal_area: area,
        }
    }

    /// Create layout with side panel
    pub fn with_side_panel(area: Rect, panel_width: u16) -> Self {
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),      // Main content
                Constraint::Length(1),   // Status bar
            ])
            .split(area);

        let horizontal_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(1),              // Main content
                Constraint::Length(panel_width), // Side panel
            ])
            .split(main_layout[0]);

        let content_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),      // Messages/content
                Constraint::Length(3),   // Input area
            ])
            .split(horizontal_layout[0]);

        Self {
            main_area: content_layout[0],
            side_panel: Some(horizontal_layout[1]),
            status_area: main_layout[1],
            input_area: content_layout[1],
            terminal_area: area,
        }
    }

    /// Update layout when terminal size changes
    pub fn resize(&mut self, new_area: Rect) {
        *self = if self.side_panel.is_some() {
            // Preserve side panel if it existed
            Self::with_side_panel(new_area, 40) // Default width
        } else {
            Self::new(new_area)
        };
    }
}

/// Popup layout for modals and dialogs
#[derive(Debug, Clone)]
pub struct PopupLayout;

impl PopupLayout {
    /// Create a centered popup area
    pub fn centered(area: Rect, width: u16, height: u16) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length((area.height.saturating_sub(height)) / 2),
                Constraint::Length(height),
                Constraint::Length((area.height.saturating_sub(height)) / 2),
            ])
            .split(area);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length((area.width.saturating_sub(width)) / 2),
                Constraint::Length(width),
                Constraint::Length((area.width.saturating_sub(width)) / 2),
            ])
            .split(popup_layout[1])[1]
    }

    /// Create a popup that takes a percentage of the screen
    pub fn percentage(area: Rect, width_percent: u16, height_percent: u16) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - height_percent) / 2),
                Constraint::Percentage(height_percent),
                Constraint::Percentage((100 - height_percent) / 2),
            ])
            .split(area);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - width_percent) / 2),
                Constraint::Percentage(width_percent),
                Constraint::Percentage((100 - width_percent) / 2),
            ])
            .split(popup_layout[1])[1]
    }
}

/// Flexible layout system similar to CSS Flexbox
#[derive(Debug, Clone)]
pub struct FlexLayout {
    direction: FlexDirection,
    justify_content: JustifyContent,
    align_items: AlignItems,
    wrap: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum FlexDirection {
    Row,
    Column,
}

#[derive(Debug, Clone, Copy)]
pub enum JustifyContent {
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Debug, Clone, Copy)]
pub enum AlignItems {
    FlexStart,
    FlexEnd,
    Center,
    Stretch,
}

impl FlexLayout {
    /// Create a new flex layout
    pub fn new() -> Self {
        Self {
            direction: FlexDirection::Column,
            justify_content: JustifyContent::FlexStart,
            align_items: AlignItems::Stretch,
            wrap: false,
        }
    }

    /// Set the flex direction
    pub fn direction(mut self, direction: FlexDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Set justify content
    pub fn justify_content(mut self, justify: JustifyContent) -> Self {
        self.justify_content = justify;
        self
    }

    /// Set align items
    pub fn align_items(mut self, align: AlignItems) -> Self {
        self.align_items = align;
        self
    }

    /// Enable/disable wrapping
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    /// Apply the flex layout to create ratatui constraints
    pub fn apply(&self, items: &[FlexItem]) -> Vec<Constraint> {
        let mut constraints = Vec::new();

        for item in items {
            match item.flex {
                FlexBasis::Fixed(size) => constraints.push(Constraint::Length(size)),
                FlexBasis::Percentage(percent) => constraints.push(Constraint::Percentage(percent)),
                FlexBasis::Flex(flex) => {
                    if flex == 1 {
                        constraints.push(Constraint::Min(1));
                    } else {
                        constraints.push(Constraint::Ratio(flex as u32, items.len() as u32));
                    }
                }
                FlexBasis::Auto => constraints.push(Constraint::Min(1)),
            }
        }

        constraints
    }
}

impl Default for FlexLayout {
    fn default() -> Self {
        Self::new()
    }
}

/// Flex item definition
#[derive(Debug, Clone)]
pub struct FlexItem {
    pub flex: FlexBasis,
    pub margin: Margin,
    pub padding: Padding,
}

impl FlexItem {
    /// Create a new flex item
    pub fn new(flex: FlexBasis) -> Self {
        Self {
            flex,
            margin: Margin::default(),
            padding: Padding::default(),
        }
    }

    /// Set margin
    pub fn margin(mut self, margin: Margin) -> Self {
        self.margin = margin;
        self
    }

    /// Set padding
    pub fn padding(mut self, padding: Padding) -> Self {
        self.padding = padding;
        self
    }
}

/// Flex basis for sizing items
#[derive(Debug, Clone, Copy)]
pub enum FlexBasis {
    /// Fixed size in characters
    Fixed(u16),
    /// Percentage of container
    Percentage(u16),
    /// Flex grow factor
    Flex(f32),
    /// Auto size based on content
    Auto,
}

/// Margin definition
#[derive(Debug, Clone, Copy, Default)]
pub struct Margin {
    pub top: u16,
    pub right: u16,
    pub bottom: u16,
    pub left: u16,
}

impl Margin {
    /// Create uniform margin
    pub fn uniform(size: u16) -> Self {
        Self {
            top: size,
            right: size,
            bottom: size,
            left: size,
        }
    }

    /// Create vertical/horizontal margin
    pub fn vh(vertical: u16, horizontal: u16) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }
}

/// Padding definition
#[derive(Debug, Clone, Copy, Default)]
pub struct Padding {
    pub top: u16,
    pub right: u16,
    pub bottom: u16,
    pub left: u16,
}

impl Padding {
    /// Create uniform padding
    pub fn uniform(size: u16) -> Self {
        Self {
            top: size,
            right: size,
            bottom: size,
            left: size,
        }
    }

    /// Create vertical/horizontal padding
    pub fn vh(vertical: u16, horizontal: u16) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }
}

/// Grid layout system
#[derive(Debug, Clone)]
pub struct GridLayout {
    rows: Vec<GridTrack>,
    columns: Vec<GridTrack>,
    gap: u16,
}

#[derive(Debug, Clone)]
pub enum GridTrack {
    Fixed(u16),
    Fraction(u16),
    Auto,
    MinMax(u16, u16),
}

impl GridLayout {
    /// Create a new grid layout
    pub fn new() -> Self {
        Self {
            rows: Vec::new(),
            columns: Vec::new(),
            gap: 0,
        }
    }

    /// Set grid template rows
    pub fn rows(mut self, rows: Vec<GridTrack>) -> Self {
        self.rows = rows;
        self
    }

    /// Set grid template columns
    pub fn columns(mut self, columns: Vec<GridTrack>) -> Self {
        self.columns = columns;
        self
    }

    /// Set grid gap
    pub fn gap(mut self, gap: u16) -> Self {
        self.gap = gap;
        self
    }

    /// Create the grid areas
    pub fn areas(&self, container: Rect) -> Vec<Vec<Rect>> {
        // This is a simplified grid implementation
        // In a full implementation, this would handle complex grid logic
        let mut areas = Vec::new();
        
        let row_constraints: Vec<Constraint> = self.rows.iter().map(|track| {
            match track {
                GridTrack::Fixed(size) => Constraint::Length(*size),
                GridTrack::Fraction(fr) => Constraint::Ratio(*fr as u32, self.rows.len() as u32),
                GridTrack::Auto => Constraint::Min(1),
                GridTrack::MinMax(min, _max) => Constraint::Min(*min),
            }
        }).collect();

        let col_constraints: Vec<Constraint> = self.columns.iter().map(|track| {
            match track {
                GridTrack::Fixed(size) => Constraint::Length(*size),
                GridTrack::Fraction(fr) => Constraint::Ratio(*fr as u32, self.columns.len() as u32),
                GridTrack::Auto => Constraint::Min(1),
                GridTrack::MinMax(min, _max) => Constraint::Min(*min),
            }
        }).collect();

        // Create row layout
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(row_constraints)
            .split(container);

        // Create column layouts for each row
        for row_area in rows {
            let columns = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(col_constraints.clone())
                .split(row_area);
            areas.push(columns);
        }

        areas
    }
}

impl Default for GridLayout {
    fn default() -> Self {
        Self::new()
    }
}

/// Responsive layout system that adapts to screen size
#[derive(Debug, Clone)]
pub struct ResponsiveLayout {
    breakpoints: Vec<Breakpoint>,
}

#[derive(Debug, Clone)]
pub struct Breakpoint {
    pub min_width: u16,
    pub layout: ResponsiveLayoutType,
}

#[derive(Debug, Clone)]
pub enum ResponsiveLayoutType {
    SingleColumn,
    TwoColumn { left_width: u16 },
    ThreeColumn { left_width: u16, right_width: u16 },
    Custom(Box<dyn Fn(Rect) -> Vec<Rect> + Send + Sync>),
}

impl ResponsiveLayout {
    /// Create a new responsive layout
    pub fn new() -> Self {
        let mut layout = Self {
            breakpoints: Vec::new(),
        };
        
        // Default breakpoints
        layout.add_breakpoint(0, ResponsiveLayoutType::SingleColumn);
        layout.add_breakpoint(80, ResponsiveLayoutType::TwoColumn { left_width: 40 });
        layout.add_breakpoint(120, ResponsiveLayoutType::ThreeColumn { 
            left_width: 30, 
            right_width: 30 
        });
        
        layout
    }

    /// Add a breakpoint
    pub fn add_breakpoint(&mut self, min_width: u16, layout: ResponsiveLayoutType) {
        self.breakpoints.push(Breakpoint { min_width, layout });
        self.breakpoints.sort_by_key(|bp| bp.min_width);
    }

    /// Get the appropriate layout for the given width
    pub fn get_layout(&self, width: u16) -> &ResponsiveLayoutType {
        for breakpoint in self.breakpoints.iter().rev() {
            if width >= breakpoint.min_width {
                return &breakpoint.layout;
            }
        }
        &self.breakpoints[0].layout
    }

    /// Apply the responsive layout
    pub fn apply(&self, area: Rect) -> Vec<Rect> {
        let layout_type = self.get_layout(area.width);
        
        match layout_type {
            ResponsiveLayoutType::SingleColumn => vec![area],
            ResponsiveLayoutType::TwoColumn { left_width } => {
                Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Length(*left_width),
                        Constraint::Min(1),
                    ])
                    .split(area)
            },
            ResponsiveLayoutType::ThreeColumn { left_width, right_width } => {
                Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Length(*left_width),
                        Constraint::Min(1),
                        Constraint::Length(*right_width),
                    ])
                    .split(area)
            },
            ResponsiveLayoutType::Custom(_custom_fn) => {
                // For now, fallback to single column
                // In a full implementation, we'd call the custom function
                vec![area]
            },
        }
    }
}

impl Default for ResponsiveLayout {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_manager_creation() {
        let area = Rect::new(0, 0, 100, 50);
        let layout = LayoutManager::new(area);
        
        assert_eq!(layout.terminal_area, area);
        assert!(layout.main_area.height > 0);
        assert!(layout.status_area.height == 1);
        assert!(layout.input_area.height == 3);
    }

    #[test]
    fn test_popup_centered() {
        let area = Rect::new(0, 0, 100, 50);
        let popup = PopupLayout::centered(area, 50, 20);
        
        assert_eq!(popup.width, 50);
        assert_eq!(popup.height, 20);
        assert_eq!(popup.x, 25); // Centered horizontally
        assert_eq!(popup.y, 15); // Centered vertically
    }

    #[test]
    fn test_flex_layout() {
        let flex = FlexLayout::new()
            .direction(FlexDirection::Row)
            .justify_content(JustifyContent::SpaceBetween);
        
        let items = vec![
            FlexItem::new(FlexBasis::Fixed(20)),
            FlexItem::new(FlexBasis::Flex(1.0)),
            FlexItem::new(FlexBasis::Fixed(30)),
        ];
        
        let constraints = flex.apply(&items);
        assert_eq!(constraints.len(), 3);
    }

    #[test]
    fn test_responsive_layout() {
        let layout = ResponsiveLayout::new();
        
        // Test small screen
        let small_layout = layout.get_layout(50);
        matches!(small_layout, ResponsiveLayoutType::SingleColumn);
        
        // Test medium screen
        let medium_layout = layout.get_layout(90);
        matches!(medium_layout, ResponsiveLayoutType::TwoColumn { .. });
        
        // Test large screen
        let large_layout = layout.get_layout(130);
        matches!(large_layout, ResponsiveLayoutType::ThreeColumn { .. });
    }
}