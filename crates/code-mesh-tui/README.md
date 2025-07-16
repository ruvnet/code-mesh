# Code-Mesh TUI 🖥️✨

[![Crates.io](https://img.shields.io/crates/v/code-mesh-tui.svg)](https://crates.io/crates/code-mesh-tui)
[![Documentation](https://docs.rs/code-mesh-tui/badge.svg)](https://docs.rs/code-mesh-tui)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/ruvnet/code-mesh)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)

**Interactive Terminal User Interface for the Code-Mesh distributed swarm intelligence system.**

Code-Mesh TUI provides a beautiful, feature-rich terminal interface to visualize, monitor, and control your Code-Mesh swarms in real-time. Experience the power of distributed computing through an intuitive, responsive interface.

## 🌟 Features

### 🎨 **Beautiful Interface**
- **Modern Design**: Clean, intuitive interface with customizable themes
- **Real-time Updates**: Live monitoring with smooth animations
- **Responsive Layout**: Adapts to any terminal size
- **Syntax Highlighting**: Code display with language-aware highlighting

### 📊 **Visual Monitoring**
- **Swarm Topology Visualization**: Interactive network graphs of agent relationships
- **Performance Dashboards**: Real-time metrics with charts and graphs
- **Agent Status Display**: Live status updates for all active agents
- **Task Progress Tracking**: Visual progress bars and completion indicators

### 🎮 **Interactive Controls**
- **Command Palette**: Quick access to all functions with fuzzy search
- **Keyboard Shortcuts**: Efficient navigation and control
- **Mouse Support**: Click and scroll support where available
- **Multi-panel Layout**: Split views for monitoring multiple aspects simultaneously

### 🧠 **Neural Intelligence Visualization**
- **Neural Network Display**: Visual representation of agent neural networks
- **Learning Progress**: Real-time learning metrics and pattern recognition
- **Cognitive Pattern Visualization**: See how different thinking patterns work
- **Cross-Agent Communication**: Visualize message passing between agents

## 🚀 Installation

### From Crates.io

```bash
cargo install code-mesh-tui
```

### From Source

```bash
git clone https://github.com/ruvnet/code-mesh
cd code-mesh
cargo install --path crates/code-mesh-tui
```

### As a Library

Add this to your `Cargo.toml`:

```toml
[dependencies]
code-mesh-tui = "0.1"
```

## 🚀 Quick Start

### Launch the TUI

```bash
# Start with default settings
code-mesh-tui

# Start with specific configuration
code-mesh-tui --config ~/.config/code-mesh/tui-config.toml

# Connect to remote Code-Mesh instance
code-mesh-tui --connect ws://localhost:8080

# Start in specific mode
code-mesh-tui --mode monitor
```

### Basic Navigation

```
Tab / Shift+Tab    - Navigate between panels
Ctrl+P             - Open command palette
Ctrl+Q             - Quit application
?                  - Show help
Space              - Toggle pause/resume
F5                 - Refresh data
```

## 🎨 Interface Overview

### Main Dashboard

```
┌─ Code-Mesh TUI v0.1.0 ─────────────────────────────────────────────────────┐
│ ⚡ Swarm: mesh-001 │ 🧠 Agents: 5/8 │ 📊 CPU: 45% │ 💾 RAM: 2.1GB │ ↗ Net │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─ Swarm Topology ─────────────┐ ┌─ Performance Metrics ──────────────────┐│
│  │                              │ │  CPU Usage        █████████░░ 89%      ││
│  │    [R1]────[C1]              │ │  Memory Usage     ███████░░░░ 67%      ││
│  │     │       │                │ │  Neural Ops/sec   ████████░░ 85%      ││
│  │    [A1]────[O1]              │ │  Network I/O      ██████░░░░ 59%      ││
│  │     │       │                │ │                                       ││
│  │    [A2]────[C2]              │ │  ┌─ Recent Tasks ─────────────────────┐││
│  │                              │ │  │ ✓ Code analysis    (2.3s)        │││
│  │  Legend:                     │ │  │ ⏳ Optimization    (45% done)    │││
│  │  [R] Researcher [C] Coder    │ │  │ ⏸ Documentation   (paused)       │││
│  │  [A] Analyst    [O] Optimizer│ │  │ ⌛ Testing         (queued)       │││
│  └──────────────────────────────┘ │  └───────────────────────────────────┘││
│                                   └─────────────────────────────────────────┘│
│                                                                             │
│  ┌─ Agent Details ──────────────────────────────────────────────────────────┐│
│  │ Agent ID: researcher-001                    Status: ⚡ Active             ││
│  │ Type: Researcher                           Uptime: 2h 15m 33s           ││
│  │ Cognitive Pattern: Adaptive                Tasks: 12 completed          ││
│  │ Neural Network: nn-1752..                  Success Rate: 98.3%          ││
│  │                                                                         ││
│  │ Current Task: "Analyze codebase structure and dependencies"             ││
│  │ Progress: ████████████████░░░░ 78%                                      ││
│  │ ETA: 45 seconds                                                         ││
│  └─────────────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────────────┘
```

### Available Views

#### 1. **Dashboard View** (Default)
- Overview of entire swarm
- Key performance metrics
- Active task monitoring
- Agent status summary

#### 2. **Swarm View**
- Detailed swarm topology visualization
- Agent relationships and communication flows
- Network health and connectivity status
- Real-time topology optimization

#### 3. **Agent View**
- Individual agent details
- Neural network visualization
- Performance metrics per agent
- Task history and success rates

#### 4. **Task View**
- Task queue and execution status
- Detailed progress tracking
- Task results and outputs
- Performance analytics

#### 5. **Neural View**
- Neural network architecture display
- Learning progress visualization
- Pattern recognition results
- Cross-agent knowledge sharing

#### 6. **Performance View**
- Comprehensive system metrics
- Resource utilization graphs
- Performance trends and analytics
- Bottleneck identification

## ⌨️ Keyboard Shortcuts

### Global Shortcuts

| Key | Action |
|-----|--------|
| `Ctrl+Q` | Quit application |
| `Ctrl+P` | Open command palette |
| `Tab` / `Shift+Tab` | Navigate panels |
| `?` | Show help |
| `F5` | Refresh data |
| `Space` | Pause/Resume updates |
| `Ctrl+R` | Reset view |

### View-Specific Shortcuts

#### Dashboard View
| Key | Action |
|-----|--------|
| `1-6` | Switch to specific view |
| `Enter` | View details of selected item |
| `D` | Toggle detailed mode |

#### Swarm View
| Key | Action |
|-----|--------|
| `+/-` | Zoom in/out |
| `Arrow Keys` | Pan around topology |
| `S` | Save topology layout |
| `R` | Reset layout |

#### Agent View
| Key | Action |
|-----|--------|
| `Up/Down` | Select agent |
| `Enter` | View agent details |
| `K` | Kill selected agent |
| `M` | Send message to agent |

#### Task View
| Key | Action |
|-----|--------|
| `Up/Down` | Select task |
| `Enter` | View task details |
| `C` | Cancel selected task |
| `N` | Create new task |

## 🎨 Themes and Customization

### Built-in Themes

```bash
# Dark theme (default)
code-mesh-tui --theme dark

# Light theme
code-mesh-tui --theme light

# High contrast
code-mesh-tui --theme high-contrast

# Cyberpunk theme
code-mesh-tui --theme cyberpunk

# Minimalist theme
code-mesh-tui --theme minimal
```

### Custom Theme Configuration

Create `~/.config/code-mesh/tui-theme.toml`:

```toml
[colors]
background = "#1e1e1e"
foreground = "#d4d4d4"
primary = "#007acc"
secondary = "#ff6b6b"
success = "#4ade80"
warning = "#facc15"
error = "#ef4444"
accent = "#a855f7"

[borders]
style = "rounded"  # rounded, sharp, double, thick
color = "primary"

[graphs]
line_color = "primary"
fill_color = "secondary"
grid_color = "foreground"
```

### Layout Customization

```toml
[layout]
show_borders = true
panel_spacing = 1
header_height = 3
footer_height = 2

[panels]
swarm_topology = { x = 0, y = 0, width = 50, height = 60 }
performance_metrics = { x = 50, y = 0, width = 50, height = 30 }
task_list = { x = 50, y = 30, width = 50, height = 30 }
agent_details = { x = 0, y = 60, width = 100, height = 40 }
```

## 🔧 Configuration

### Configuration File (`~/.config/code-mesh/tui-config.toml`)

```toml
[display]
theme = "dark"
refresh_rate = 1000  # milliseconds
enable_mouse = true
enable_unicode = true

[performance]
max_fps = 60
buffer_size = 1000
enable_vsync = true

[networking]
connection_timeout = 5000
retry_attempts = 3
keepalive_interval = 30

[features]
enable_neural_viz = true
enable_audio_alerts = false
enable_notifications = true
show_debug_info = false

[keybindings]
quit = "Ctrl+Q"
help = "?"
refresh = "F5"
command_palette = "Ctrl+P"
```

## 🚀 Advanced Usage

### Embedding in Applications

```rust
use code_mesh_tui::{TuiApp, TuiConfig};
use code_mesh_core::CodeMesh;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize Code-Mesh core
    let mesh = CodeMesh::new().await?;
    
    // Configure TUI
    let config = TuiConfig::default()
        .with_theme("cyberpunk")
        .with_refresh_rate(500)
        .with_mouse_support(true);
    
    // Create and run TUI
    let mut tui = TuiApp::new(mesh, config).await?;
    tui.run().await?;
    
    Ok(())
}
```

### Custom Views

```rust
use code_mesh_tui::{View, ViewContext, RenderResult};

struct CustomMetricsView {
    data: Vec<Metric>,
}

impl View for CustomMetricsView {
    fn render(&mut self, ctx: &ViewContext) -> RenderResult {
        // Custom rendering logic
        Ok(())
    }
    
    fn handle_input(&mut self, input: InputEvent) -> bool {
        // Custom input handling
        false
    }
}

// Register custom view
tui.register_view("custom_metrics", Box::new(CustomMetricsView::new()));
```

### Remote Monitoring

```bash
# Start Code-Mesh server with TUI endpoint
code-mesh serve --tui-port 8080

# Connect TUI to remote instance
code-mesh-tui --connect ws://remote-server:8080

# SSH tunnel for secure connection
ssh -L 8080:localhost:8080 remote-server
code-mesh-tui --connect ws://localhost:8080
```

## 🎯 Performance Optimization

### For Large Swarms (10+ agents)

```toml
[performance]
# Reduce refresh rate for better performance
refresh_rate = 2000
max_fps = 30

# Limit data retention
buffer_size = 500
max_history = 100

# Disable expensive features
enable_neural_viz = false
show_topology_animation = false
```

### For Resource-Constrained Systems

```toml
[display]
# Use minimal theme
theme = "minimal"
enable_unicode = false

[features]
# Disable non-essential features
enable_audio_alerts = false
enable_notifications = false
show_debug_info = false
```

## 🐛 Troubleshooting

### Common Issues

**Issue**: TUI doesn't display correctly
**Solution**: Ensure terminal supports 256 colors and Unicode

**Issue**: High CPU usage
**Solution**: Increase refresh_rate in config (e.g., 2000ms)

**Issue**: Connection timeouts
**Solution**: Check network connectivity and increase connection_timeout

**Issue**: Garbled display
**Solution**: Try `--theme minimal` or disable Unicode

### Debug Mode

```bash
# Enable debug logging
export CODE_MESH_TUI_LOG=debug

# Run with debug output
code-mesh-tui --debug 2>debug.log

# Capture screen output
code-mesh-tui --capture-output tui-output.txt
```

## 📚 Documentation

- [TUI User Guide](https://github.com/ruvnet/code-mesh/docs/tui-guide.md)
- [Theme Development](https://github.com/ruvnet/code-mesh/docs/tui-themes.md)
- [Custom Views](https://github.com/ruvnet/code-mesh/docs/tui-views.md)
- [API Reference](https://docs.rs/code-mesh-tui)

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](https://github.com/ruvnet/code-mesh/CONTRIBUTING.md) for details.

## 📜 License

This project is licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## 👨‍💻 Creator

**Created by [ruv](https://github.com/ruvnet)** - Innovator in AI-driven development tools and distributed systems.

**Repository**: [github.com/ruvnet/code-mesh](https://github.com/ruvnet/code-mesh)

---

<div align="center">

**Code-Mesh TUI - Visualize Your Swarm Intelligence** 🖥️✨

*Beautiful, interactive terminal interface for distributed computing*

</div>