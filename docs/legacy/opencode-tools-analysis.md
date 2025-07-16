# OpenCode Tools Analysis and Rust Implementation Comparison

## 1. Complete List of OpenCode Tools

### Core File Operations
1. **ReadTool** (`read`)
   - **Functionality**: Reads file contents with optional line offset and limit
   - **Key Features**: 
     - Line-numbered output
     - Offset and limit support
     - Image file detection
     - File existence suggestions
   - **Status in Rust**: ✅ Implemented

2. **WriteTool** (`write`)
   - **Functionality**: Creates or overwrites files with content
   - **Key Features**:
     - Creates parent directories automatically
     - LSP integration for diagnostics
     - Permission management
     - File existence tracking
   - **Status in Rust**: ✅ Implemented (basic version)

3. **EditTool** (`edit`)
   - **Functionality**: Advanced string replacement in files
   - **Key Features**:
     - Multiple replacement strategies (SimpleReplacer, LineTrimmedReplacer, BlockAnchorReplacer, etc.)
     - Fuzzy matching with indentation flexibility
     - Diff generation
     - LSP diagnostics after edit
   - **Status in Rust**: ❌ Not Implemented

4. **MultiEditTool** (`multiedit`)
   - **Functionality**: Batch edit operations on a single file
   - **Key Features**:
     - Sequential edit application
     - Atomic operations (all or nothing)
     - Reuses EditTool functionality
   - **Status in Rust**: ❌ Not Implemented

5. **PatchTool** (`patch`)
   - **Functionality**: Apply complex patches to multiple files
   - **Key Features**:
     - Supports add/update/delete operations
     - Context-aware patching
     - Fuzz detection
     - Atomic commits
   - **Status in Rust**: ❌ Not Implemented

### Command Execution
6. **BashTool** (`bash`)
   - **Functionality**: Execute shell commands
   - **Key Features**:
     - Timeout support (default 2 min, max 10 min)
     - Output truncation at 30KB
     - Abort signal handling
     - Working directory support
   - **Status in Rust**: ✅ Implemented

### Search and Navigation
7. **GrepTool** (`grep`)
   - **Functionality**: Search file contents using ripgrep
   - **Key Features**:
     - Regex pattern support
     - File pattern filtering
     - Sorted by modification time
     - Line-numbered results
   - **Status in Rust**: ❌ Not Implemented

8. **GlobTool** (`glob`)
   - **Functionality**: Find files by glob patterns
   - **Key Features**:
     - Recursive search
     - Modification time sorting
     - Result truncation
     - Path resolution
   - **Status in Rust**: ❌ Not Implemented

9. **ListTool** (`list`)
   - **Functionality**: List directory contents in tree format
   - **Key Features**:
     - Tree structure rendering
     - Built-in ignore patterns
     - Custom ignore support
     - Result limiting
   - **Status in Rust**: ❌ Not Implemented

### Web Operations
10. **WebFetchTool** (`webfetch`)
    - **Functionality**: Fetch and convert web content
    - **Key Features**:
      - HTML to Markdown conversion
      - Text extraction
      - Timeout support
      - Size limits (5MB)
    - **Status in Rust**: ❌ Not Implemented

### Language Server Protocol (LSP)
11. **LspDiagnosticTool** (`lsp_diagnostics`)
    - **Functionality**: Get code diagnostics from LSP
    - **Key Features**:
      - Error/warning detection
      - Pretty error formatting
      - File-specific diagnostics
    - **Status in Rust**: ❌ Not Implemented

12. **LspHoverTool** (`lsp_hover`)
    - **Functionality**: Get hover information at specific positions
    - **Key Features**:
      - Position-specific information
      - Type information
      - Documentation lookup
    - **Status in Rust**: ❌ Not Implemented

### Task Management
13. **TodoWriteTool** (`todowrite`)
    - **Functionality**: Manage todo lists
    - **Key Features**:
      - Status tracking (pending/in_progress/completed/cancelled)
      - Priority levels
      - Session-based storage
      - Batch operations
    - **Status in Rust**: ❌ Not Implemented

14. **TodoReadTool** (`todoread`)
    - **Functionality**: Read current todo list
    - **Key Features**:
      - Session-specific todos
      - Count filtering
      - JSON output
    - **Status in Rust**: ❌ Not Implemented

15. **TaskTool** (`task`)
    - **Functionality**: Spawn sub-agents for complex tasks
    - **Key Features**:
      - Creates new AI sessions
      - Real-time progress tracking
      - Nested tool execution
      - Abort propagation
    - **Status in Rust**: ❌ Not Implemented

## 2. Tools Currently Enabled in OpenCode

From the provider.ts file, these tools are actively registered:
- ✅ BashTool
- ✅ EditTool
- ✅ WebFetchTool
- ✅ GlobTool
- ✅ GrepTool
- ✅ ListTool
- ✅ PatchTool
- ✅ ReadTool
- ✅ WriteTool
- ✅ TodoWriteTool
- ✅ TodoReadTool
- ✅ TaskTool

Commented out (disabled):
- ❌ LspDiagnosticTool
- ❌ LspHoverTool
- ❌ MultiEditTool (though available)

## 3. Critical Missing Features in Rust Implementation

### High Priority
1. **EditTool** - Essential for code modifications
   - Complex replacement strategies
   - Diff generation
   - Integration with LSP

2. **GrepTool** - Critical for code search
   - Regex support
   - File filtering
   - Performance optimization

3. **GlobTool** - Important for file discovery
   - Pattern matching
   - Recursive search

4. **TaskTool** - Key for complex operations
   - Sub-agent spawning
   - Parallel execution
   - Progress tracking

### Medium Priority
5. **TodoWrite/ReadTool** - Task management
   - Progress tracking
   - Session persistence

6. **ListTool** - Directory navigation
   - Tree visualization
   - Smart filtering

7. **PatchTool** - Advanced editing
   - Multi-file operations
   - Atomic commits

### Lower Priority
8. **WebFetchTool** - External content
   - HTML processing
   - Content conversion

9. **LSP Tools** - Code intelligence
   - Error detection
   - Type information

10. **MultiEditTool** - Batch operations
    - Sequential edits
    - Atomic transactions

## 4. Implementation Recommendations

### Immediate Priorities
1. **EditTool** - Most critical for AI code editing
2. **GrepTool** - Essential for code navigation
3. **GlobTool** - Required for file discovery

### Architecture Considerations
1. **Tool Registry**: Already implemented ✅
2. **Permission System**: Need to add for write operations
3. **LSP Integration**: Required for diagnostics
4. **Session Management**: For todo/task persistence
5. **File Time Tracking**: For concurrent access control

### Missing Infrastructure
1. **HTMLRewriter** equivalent for WebFetch
2. **Ripgrep** integration for search tools
3. **Diff library** for edit operations
4. **LSP client** for code intelligence
5. **Permission management** system

## Summary

Your Rust implementation has successfully implemented 3 out of 15 tools (20%):
- ✅ ReadTool (basic version)
- ✅ WriteTool (basic version)
- ✅ BashTool

The most critical missing tools for a functional AI coding assistant are:
1. EditTool (for code modifications)
2. GrepTool (for searching)
3. GlobTool (for file discovery)
4. TaskTool (for complex operations)

These four tools would bring the implementation to ~50% feature parity and cover the most essential use cases.