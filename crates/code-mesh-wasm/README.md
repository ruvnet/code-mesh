# Code-Mesh WASM 🌐⚡

[![Crates.io](https://img.shields.io/crates/v/code-mesh-wasm.svg)](https://crates.io/crates/code-mesh-wasm)
[![npm](https://img.shields.io/npm/v/@ruvnet/code-mesh)](https://www.npmjs.com/package/@ruvnet/code-mesh)
[![Documentation](https://docs.rs/code-mesh-wasm/badge.svg)](https://docs.rs/code-mesh-wasm)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/ruvnet/code-mesh)

**WebAssembly bindings for the Code-Mesh distributed swarm intelligence system.**

Code-Mesh WASM brings the full power of Rust-based distributed computing to JavaScript and browser environments. Experience blazing-fast performance with native WASM execution while maintaining the rich ecosystem of web technologies.

## 🌟 Features

### 🚀 **Native WASM Performance**
- **Rust-to-WASM Compilation**: Near-native execution speed in browsers
- **SIMD Optimization**: Hardware-accelerated operations where supported
- **Memory Efficiency**: Smart memory management with minimal overhead
- **Zero-Copy Operations**: Direct memory access for maximum performance

### 🌐 **Universal JavaScript Support**
- **Browser Compatible**: Works in all modern browsers
- **Node.js Ready**: Full server-side JavaScript support
- **TypeScript Definitions**: Complete type safety and IntelliSense
- **Module Formats**: ESM, CommonJS, and UMD builds available

### 🧠 **Neural WASM Networks**
- **WebAssembly ML**: Neural networks compiled to WASM
- **Browser-based AI**: Client-side machine learning capabilities
- **Real-time Processing**: Low-latency neural operations
- **Offline Capabilities**: No server required for AI processing

### ⚡ **Swarm in the Browser**
- **Web Workers**: Multi-threaded agent execution
- **Shared Array Buffers**: High-performance inter-agent communication
- **Service Workers**: Background swarm processing
- **Progressive Web Apps**: Offline-capable distributed applications

## 🚀 Installation

### NPM Package

```bash
# Install the npm package
npm install @ruvnet/code-mesh

# Or with yarn
yarn add @ruvnet/code-mesh

# Or with pnpm
pnpm add @ruvnet/code-mesh
```

### Rust Crate

```toml
[dependencies]
code-mesh-wasm = "0.1"
wasm-bindgen = "0.2"
```

### CDN (Browser)

```html
<!-- Modern ES6 modules -->
<script type="module">
  import { CodeMesh } from 'https://unpkg.com/@ruvnet/code-mesh/dist/browser.js';
  // Your code here
</script>

<!-- Traditional script tag -->
<script src="https://unpkg.com/@ruvnet/code-mesh/dist/browser.umd.js"></script>
```

## 🚀 Quick Start

### Browser Usage

```html
<!DOCTYPE html>
<html>
<head>
    <title>Code-Mesh WASM Demo</title>
</head>
<body>
    <script type="module">
        import { CodeMesh } from '@ruvnet/code-mesh';
        
        async function main() {
            // Initialize Code-Mesh WASM
            const mesh = new CodeMesh();
            await mesh.init();
            
            // Create a browser-based swarm
            const swarm = await mesh.createSwarm({
                topology: 'mesh',
                agents: 3,
                useWebWorkers: true
            });
            
            // Execute a task across web workers
            const result = await swarm.executeTask({
                type: 'data-processing',
                data: largeDataset,
                operation: 'analyze'
            });
            
            console.log('Processing complete:', result);
        }
        
        main();
    </script>
</body>
</html>
```

### Node.js Usage

```javascript
import { CodeMesh } from '@ruvnet/code-mesh';

async function main() {
    // Initialize Code-Mesh for Node.js
    const mesh = new CodeMesh();
    await mesh.init();
    
    // Create high-performance swarm
    const swarm = await mesh.createSwarm({
        topology: 'hierarchical',
        agents: 8,
        enableSIMD: true
    });
    
    // Process files with WASM speed
    const files = await fs.readdir('./src');
    const results = await swarm.processFiles(files, {
        operation: 'analyze',
        parallel: true
    });
    
    console.log('Analysis results:', results);
}

main().catch(console.error);
```

### TypeScript Usage

```typescript
import { 
    CodeMesh, 
    SwarmConfig, 
    AgentType, 
    TaskResult 
} from '@ruvnet/code-mesh';

interface AnalysisTask {
    files: string[];
    operation: 'analyze' | 'optimize' | 'test';
    options?: {
        parallel?: boolean;
        neural?: boolean;
    };
}

async function analyzeCodebase(): Promise<TaskResult> {
    const mesh = new CodeMesh();
    await mesh.init();
    
    const config: SwarmConfig = {
        topology: 'mesh',
        agents: 5,
        agentTypes: [
            AgentType.Researcher,
            AgentType.Coder, 
            AgentType.Analyst
        ],
        enableNeuralNetworks: true
    };
    
    const swarm = await mesh.createSwarm(config);
    
    const task: AnalysisTask = {
        files: ['src/**/*.ts'],
        operation: 'analyze',
        options: {
            parallel: true,
            neural: true
        }
    };
    
    return await swarm.executeTask(task);
}
```

## 🛠️ API Reference

### Core Classes

#### `CodeMesh`

Main entry point for the WASM module.

```javascript
class CodeMesh {
    constructor(config?: CodeMeshConfig)
    async init(): Promise<void>
    async createSwarm(config: SwarmConfig): Promise<Swarm>
    async getMetrics(): Promise<PerformanceMetrics>
    destroy(): void
}
```

#### `Swarm`

Represents a distributed agent swarm.

```javascript
class Swarm {
    async spawnAgent(type: AgentType, config?: AgentConfig): Promise<Agent>
    async executeTask(task: Task): Promise<TaskResult>
    async getAgents(): Promise<Agent[]>
    async getTopology(): Promise<TopologyInfo>
    async optimize(): Promise<void>
    destroy(): void
}
```

#### `Agent`

Individual agent within a swarm.

```javascript
class Agent {
    readonly id: string
    readonly type: AgentType
    readonly status: AgentStatus
    
    async executeTask(task: Task): Promise<TaskResult>
    async getMetrics(): Promise<AgentMetrics>
    async communicate(targetAgent: string, message: any): Promise<void>
    terminate(): void
}
```

### Configuration Interfaces

```typescript
interface SwarmConfig {
    topology: 'mesh' | 'hierarchical' | 'ring' | 'star';
    agents: number;
    agentTypes?: AgentType[];
    useWebWorkers?: boolean;
    enableSIMD?: boolean;
    enableNeuralNetworks?: boolean;
    memoryLimit?: string;
}

interface Task {
    id?: string;
    type: string;
    data?: any;
    options?: {
        timeout?: number;
        priority?: 'low' | 'medium' | 'high';
        neural?: boolean;
    };
}

interface TaskResult {
    id: string;
    status: 'completed' | 'failed' | 'timeout';
    result?: any;
    error?: string;
    metrics: {
        executionTime: number;
        memoryUsed: number;
        agentsUsed: number;
    };
}
```

## 🌐 Browser Features

### Web Workers Integration

```javascript
// Main thread
import { CodeMesh } from '@ruvnet/code-mesh';

const mesh = new CodeMesh({
    useWebWorkers: true,
    maxWorkers: navigator.hardwareConcurrency
});

// Automatic worker management
const swarm = await mesh.createSwarm({
    topology: 'mesh',
    agents: 4 // Each agent runs in separate worker
});
```

### Shared Array Buffer Support

```javascript
// Enable high-performance inter-agent communication
const mesh = new CodeMesh({
    useSharedArrayBuffer: true,
    sharedMemorySize: '64MB'
});

// Agents can now share data without serialization overhead
const result = await swarm.processLargeDataset(data);
```

### Service Worker Integration

```javascript
// service-worker.js
import { CodeMesh } from '@ruvnet/code-mesh';

let backgroundSwarm;

self.addEventListener('message', async (event) => {
    if (event.data.type === 'START_BACKGROUND_PROCESSING') {
        backgroundSwarm = new CodeMesh();
        await backgroundSwarm.init();
        
        // Process data in background
        const result = await backgroundSwarm.processInBackground(event.data.payload);
        
        // Send result back to main thread
        self.postMessage({ type: 'PROCESSING_COMPLETE', result });
    }
});
```

## 🏗️ Advanced Usage

### Custom Neural Networks

```javascript
import { NeuralNetwork, ActivationFunction } from '@ruvnet/code-mesh';

// Create custom neural network in WASM
const network = new NeuralNetwork({
    layers: [784, 128, 64, 10],
    activation: ActivationFunction.ReLU,
    optimizer: 'adam',
    learningRate: 0.001
});

// Train with WASM performance
await network.train(trainingData, {
    epochs: 100,
    batchSize: 32,
    useSIMD: true
});

// Deploy to agents
const swarm = await mesh.createSwarm({
    agents: 3,
    neuralNetwork: network
});
```

### Real-time Streaming

```javascript
// Process streaming data with WASM agents
const stream = new ReadableStream({
    start(controller) {
        // Stream data to WASM processors
    }
});

const processor = await mesh.createStreamProcessor({
    inputStream: stream,
    agents: 4,
    bufferSize: '16MB'
});

processor.on('data', (processedChunk) => {
    console.log('Processed:', processedChunk);
});
```

### Performance Monitoring

```javascript
import { PerformanceMonitor } from '@ruvnet/code-mesh';

const monitor = new PerformanceMonitor({
    enableCPUProfiling: true,
    enableMemoryProfiling: true,
    sampleRate: 1000 // 1 second
});

// Monitor WASM performance
monitor.startMonitoring();

const metrics = await monitor.getMetrics();
console.log('WASM Performance:', metrics);
```

## 🎯 Performance Optimizations

### Browser Optimizations

```javascript
// Optimize for different browser environments
const mesh = new CodeMesh({
    // Use WebAssembly SIMD if available
    autoDetectSIMD: true,
    
    // Optimize memory allocation
    memoryGrowthStrategy: 'dynamic',
    initialMemory: '32MB',
    maxMemory: '512MB',
    
    // Enable threading if available
    useSharedArrayBuffer: 'auto',
    
    // Optimize for mobile devices
    mobileOptimizations: true
});
```

### Node.js Optimizations

```javascript
// Server-side optimizations
const mesh = new CodeMesh({
    // Use all available CPU cores
    maxAgents: require('os').cpus().length,
    
    // Enable SIMD for math operations
    enableSIMD: true,
    
    // Optimize for server workloads
    serverMode: true,
    
    // Large memory allocation for big data
    memoryLimit: '2GB'
});
```

## 🔧 Build Configuration

### Webpack Integration

```javascript
// webpack.config.js
module.exports = {
    // ... other config
    experiments: {
        asyncWebAssembly: true,
        topLevelAwait: true
    },
    resolve: {
        fallback: {
            "crypto": require.resolve("crypto-browserify"),
            "stream": require.resolve("stream-browserify")
        }
    }
};
```

### Vite Configuration

```javascript
// vite.config.js
export default {
    optimizeDeps: {
        exclude: ['@ruvnet/code-mesh']
    },
    server: {
        headers: {
            'Cross-Origin-Embedder-Policy': 'require-corp',
            'Cross-Origin-Opener-Policy': 'same-origin'
        }
    }
};
```

## 🎨 Examples

### Real-time Data Processing

```javascript
// Process real-time sensor data
import { CodeMesh } from '@ruvnet/code-mesh';

const mesh = new CodeMesh();
await mesh.init();

const swarm = await mesh.createSwarm({
    topology: 'ring',
    agents: 6,
    enableNeuralNetworks: true
});

// Process sensor data stream
const sensorStream = new EventSource('/api/sensors');
sensorStream.onmessage = async (event) => {
    const sensorData = JSON.parse(event.data);
    
    const analysis = await swarm.executeTask({
        type: 'sensor-analysis',
        data: sensorData,
        options: { neural: true }
    });
    
    if (analysis.result.anomaly) {
        alert('Anomaly detected!');
    }
};
```

### Image Processing

```javascript
// Client-side image processing with WASM
const mesh = new CodeMesh();
await mesh.init();

const imageProcessor = await mesh.createImageProcessor({
    agents: 4,
    enableSIMD: true
});

document.getElementById('upload').addEventListener('change', async (e) => {
    const file = e.target.files[0];
    const imageData = await readImageData(file);
    
    const processed = await imageProcessor.process(imageData, {
        operations: ['resize', 'enhance', 'denoise'],
        parallel: true
    });
    
    displayProcessedImage(processed);
});
```

## 🐛 Troubleshooting

### Common Issues

**Issue**: WASM module fails to load
**Solution**: Ensure proper MIME type configuration and CORS headers

**Issue**: SharedArrayBuffer not available
**Solution**: Serve with proper headers for cross-origin isolation

**Issue**: Performance slower than expected
**Solution**: Enable SIMD and ensure proper memory allocation

**Issue**: TypeScript errors
**Solution**: Install `@types/node` and ensure proper tsconfig.json

### Debug Mode

```javascript
// Enable debug logging
const mesh = new CodeMesh({
    debug: true,
    logLevel: 'verbose'
});

// Monitor WASM memory usage
mesh.on('memory-warning', (usage) => {
    console.warn('High memory usage:', usage);
});

// Track performance
mesh.on('performance-update', (metrics) => {
    console.log('Performance:', metrics);
});
```

## 📚 Documentation

- [WASM API Reference](https://docs.rs/code-mesh-wasm)
- [JavaScript Guide](https://github.com/ruvnet/code-mesh/docs/javascript.md)
- [Browser Integration](https://github.com/ruvnet/code-mesh/docs/browser.md)
- [Performance Guide](https://github.com/ruvnet/code-mesh/docs/wasm-performance.md)

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

**Code-Mesh WASM - Rust Performance in Every Browser** 🌐⚡

*Bringing distributed swarm intelligence to the web*

</div>