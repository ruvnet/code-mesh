# Code Mesh NPM Package

A universal TypeScript/JavaScript library for Code Mesh that works seamlessly across Node.js, browsers, and web workers with WebAssembly performance.

## Features

- 🦀 **WebAssembly Performance**: Rust-powered core with WASM bindings
- 🌐 **Universal Compatibility**: Works in Node.js, browsers, and web workers
- 💾 **Browser Storage**: IndexedDB integration for persistent sessions
- 🔄 **Offline Support**: Service Worker and PWA capabilities
- 🧵 **Web Workers**: Background processing support
- 🔐 **Authentication**: Multiple provider support (API keys, OAuth2)
- ⚡ **Performance Monitoring**: Built-in metrics and optimization
- 📱 **Progressive Web App**: Full PWA support with manifest
- 🎯 **TypeScript**: Complete type safety and IntelliSense support

## Installation

```bash
npm install code-mesh
```

Or use with NPX:

```bash
npx code-mesh
```

## Quick Start

### Node.js

```typescript
import { CodeMesh } from 'code-mesh';

const codeMesh = new CodeMesh();
await codeMesh.initialize();

// Send a message and get AI response
const response = await codeMesh.chat(
  'Hello, can you help me with JavaScript?',
  'claude-3-sonnet-20240229',
  'your-api-key'
);

console.log(response);
```

### Browser

```html
<!DOCTYPE html>
<html>
<head>
  <script type="module">
    import { CodeMeshBrowser } from 'code-mesh/browser';
    
    const codeMesh = new CodeMeshBrowser();
    await codeMesh.initialize();
    
    // Browser-specific features
    await codeMesh.saveSession();
    const sessions = await codeMesh.listSessions();
  </script>
</head>
</html>
```

### Auto-initialization (Browser)

```html
<script src="https://unpkg.com/code-mesh/dist/browser.js"></script>
<script>
  window.addEventListener('codemesh-ready', (event) => {
    const codeMesh = event.detail;
    // CodeMesh is ready to use
  });
</script>
```

## API Reference

### Core API

#### `CodeMesh`

Universal Code Mesh interface that adapts to the current environment.

```typescript
const codeMesh = new CodeMesh(config);
await codeMesh.initialize();

// Basic operations
const sessionId = codeMesh.getSessionId();
await codeMesh.addMessage('Your message');
const response = await codeMesh.generateResponse('claude-3-sonnet', 'api-key');

// Convenience method
const response = await codeMesh.chat('Hello!', 'claude-3-sonnet', 'api-key');

// Session management (browser only)
await codeMesh.saveSession();
await codeMesh.loadSession(sessionId);
const sessions = await codeMesh.listSessions();

// Performance monitoring
const metrics = codeMesh.getPerformanceMetrics();
const memory = codeMesh.getMemoryUsage();
```

#### `CodeMeshBrowser`

Browser-specific implementation with additional features.

```typescript
const codeMesh = new CodeMeshBrowser(config);
await codeMesh.initialize();

// Browser-specific features
await codeMesh.setClipboard('text to copy');
const clipboardText = await codeMesh.getClipboard();

// PWA features
await codeMesh.installPWA();

// Data export/import
const jsonData = await codeMesh.exportSession('json');
const markdownData = await codeMesh.exportSession('markdown');
await codeMesh.importSession(jsonData);

// Web workers
const worker = await codeMesh.createWorker('/worker.js');
```

#### `WasmRunner`

Low-level WASM module management.

```typescript
const runner = new WasmRunner(config);
await runner.loadWasm();

const codeMesh = await runner.createCodeMesh();
const providers = await runner.getProviders();
const models = await runner.getModels('anthropic');
const platformInfo = await runner.getPlatformInfo();
```

### Configuration

```typescript
interface CodeMeshConfig {
  enablePerformanceMonitoring?: boolean;
  maxMemoryMB?: number;
  useWebWorkers?: boolean;
  enableOffline?: boolean;
  debug?: boolean;
  useBrowserStorage?: boolean;
  authProvider?: string;
  apiEndpoint?: string;
}

// Environment-specific defaults
const config = WasmRunner.getRecommendedConfig();
```

### Utilities

```typescript
import { Utils } from 'code-mesh';

// Feature detection
const isSupported = Utils.isWasmSupported();
const config = Utils.getRecommendedConfig();

// Data formatting
const readableSize = Utils.formatBytes(1024000);

// API key validation
const isValid = Utils.validateApiKey('sk-ant-...', 'anthropic');
```

## Environment Detection

```typescript
import { Environment } from 'code-mesh';

if (Environment.isBrowser) {
  // Browser-specific code
} else if (Environment.isNode) {
  // Node.js-specific code
} else if (Environment.isWebWorker) {
  // Web worker code
}
```

## Supported Providers

- **Anthropic**: Claude models (claude-3-opus, claude-3-sonnet, claude-3-haiku)
- **OpenAI**: GPT models (gpt-4o, gpt-4-turbo, gpt-3.5-turbo)
- **Mistral**: Mistral models (mistral-large, mistral-medium)
- **Cohere**: Command models (command-r-plus, command-r)
- **Hugging Face**: Various open-source models

## Progressive Web App (PWA)

Code Mesh includes full PWA support:

```html
<link rel="manifest" href="/manifest.json">
<script>
  // Install prompt handling
  window.addEventListener('beforeinstallprompt', (e) => {
    e.preventDefault();
    // Show install button
  });
</script>
```

Features:
- Offline functionality with Service Worker
- App-like experience on mobile and desktop  
- File handling for code analysis
- Shortcuts for quick actions
- Background sync for session data

## Performance Optimization

### Memory Management

```typescript
// Optimize memory usage
await codeMesh.optimizePerformance();

// Monitor memory
const memory = codeMesh.getMemoryUsage();
console.log(`Memory used: ${Utils.formatBytes(memory.used)}`);
```

### Web Workers

```typescript
// Create worker for background processing
const worker = await codeMesh.createWorker('/code-analysis-worker.js');

// Process large files in background
worker.postMessage({
  type: 'analyze',
  code: largeCodeFile
});
```

### WASM Features

The package automatically detects and uses available WASM features:
- SIMD for faster computation
- Threads for parallel processing
- Bulk memory operations
- Reference types

## Browser Compatibility

### Required Features

- WebAssembly support
- IndexedDB (for storage)
- Fetch API (for HTTP requests)
- Secure context (HTTPS) for advanced features

### Optional Features

- Service Workers (for offline support)
- Web Workers (for background processing)
- Clipboard API (for copy/paste)
- Notifications API
- Wake Lock API

### Fallbacks

The package includes automatic fallbacks:
- Base64 encoding when SubtleCrypto is unavailable
- Memory storage when IndexedDB is unavailable
- Fetch polyfill for older browsers
- WASM fallback to JavaScript implementation

## Building from Source

```bash
# Install dependencies
npm install

# Build WASM modules
npm run build:wasm:node    # Node.js target
npm run build:wasm:web     # Web target
npm run build:wasm:bundler # Bundler target

# Build TypeScript
npm run build:ts

# Build everything
npm run build

# Run tests
npm test                   # Node.js tests
npm run test:browser      # Browser tests

# Development mode
npm run dev
```

## Examples

### Chat Application

```typescript
import { CodeMeshBrowser } from 'code-mesh/browser';

class ChatApp {
  private codeMesh: CodeMeshBrowser;
  
  async initialize() {
    this.codeMesh = new CodeMeshBrowser();
    await this.codeMesh.initialize();
  }
  
  async sendMessage(message: string) {
    const response = await this.codeMesh.sendMessage(
      message,
      'claude-3-sonnet-20240229',
      this.getApiKey()
    );
    
    this.displayMessage('user', message);
    this.displayMessage('assistant', response);
    
    // Auto-save session
    await this.codeMesh.saveSession();
  }
  
  private getApiKey(): string {
    return localStorage.getItem('anthropic-api-key') || '';
  }
  
  private displayMessage(role: string, content: string) {
    // Update UI
  }
}
```

### Code Analysis Tool

```typescript
import { CodeMesh } from 'code-mesh';

class CodeAnalyzer {
  private codeMesh: CodeMesh;
  
  async analyzeCode(code: string, language: string) {
    if (!this.codeMesh) {
      this.codeMesh = new CodeMesh();
      await this.codeMesh.initialize();
    }
    
    const prompt = `Analyze this ${language} code and provide suggestions for improvement:\n\n${code}`;
    
    const analysis = await this.codeMesh.chat(
      prompt,
      'claude-3-sonnet-20240229'
    );
    
    return this.parseAnalysis(analysis);
  }
  
  private parseAnalysis(analysis: string) {
    // Parse and structure the analysis
    return {
      suggestions: [],
      issues: [],
      improvements: []
    };
  }
}
```

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Support

- 📧 Email: support@code-mesh.dev
- 🐛 Issues: [GitHub Issues](https://github.com/yourusername/code-mesh/issues)
- 💬 Discussions: [GitHub Discussions](https://github.com/yourusername/code-mesh/discussions)
- 📖 Documentation: [docs.code-mesh.dev](https://docs.code-mesh.dev)