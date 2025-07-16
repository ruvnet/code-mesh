/**
 * WASM runner for Code Mesh - handles loading and execution of WebAssembly modules
 * Supports both Node.js and browser environments with appropriate fallbacks
 */

import { platform, arch } from 'os';
import { promises as fs } from 'fs';
import path from 'path';

// Platform detection
const isBrowser = typeof window !== 'undefined';
const isNode = typeof process !== 'undefined' && process.versions && process.versions.node;
const isWebWorker = typeof WorkerGlobalScope !== 'undefined' && typeof importScripts === 'function';

// WASM module interface
interface CodeMeshWasm {
    CodeMesh: any;
    greet(name: string): void;
    process_data(data: string): string;
    benchmark_performance(iterations: number): any;
    main(): void;
    default?: any;
}

// Configuration for WASM loading
interface WasmConfig {
    enablePerformanceMonitoring?: boolean;
    maxMemoryMB?: number;
    useWebWorkers?: boolean;
    enableOffline?: boolean;
    debug?: boolean;
}

/**
 * WASM loader and manager
 */
export class WasmRunner {
    private wasmModule: CodeMeshWasm | null = null;
    private config: WasmConfig;
    private loadPromise: Promise<CodeMeshWasm> | null = null;

    constructor(config: WasmConfig = {}) {
        this.config = {
            enablePerformanceMonitoring: true,
            maxMemoryMB: 512,
            useWebWorkers: true,
            enableOffline: false,
            debug: false,
            ...config
        };
    }

    /**
     * Load the appropriate WASM module for the current environment
     */
    async loadWasm(): Promise<CodeMeshWasm> {
        if (this.wasmModule) {
            return this.wasmModule;
        }

        if (this.loadPromise) {
            return this.loadPromise;
        }

        this.loadPromise = this._loadWasmInternal();
        return this.loadPromise;
    }

    private async _loadWasmInternal(): Promise<CodeMeshWasm> {
        try {
            if (isBrowser) {
                return await this.loadWasmBrowser();
            } else if (isNode) {
                return await this.loadWasmNode();
            } else if (isWebWorker) {
                return await this.loadWasmWorker();
            } else {
                throw new Error('Unsupported environment for WASM loading');
            }
        } catch (error) {
            this.logError('Failed to load WASM module:', error);
            throw error;
        }
    }

    /**
     * Load WASM module in browser environment
     */
    private async loadWasmBrowser(): Promise<CodeMeshWasm> {
        this.logDebug('Loading WASM for browser environment');

        try {
            // Try to load the web target build
            const wasmModule = await import('../wasm/web/code_mesh_wasm.js');
            await wasmModule.default(); // Initialize WASM
            
            this.wasmModule = wasmModule as unknown as CodeMeshWasm;
            this.logDebug('WASM module loaded successfully in browser');
            return this.wasmModule;
        } catch (error) {
            this.logError('Failed to load web WASM build:', error);
            
            // Fallback to bundler target
            try {
                const wasmModule = await import('../wasm/web/code_mesh_wasm.js');
                await wasmModule.default();
                
                this.wasmModule = wasmModule as unknown as CodeMeshWasm;
                this.logDebug('WASM module loaded successfully using bundler fallback');
                return this.wasmModule;
            } catch (fallbackError) {
                this.logError('Failed to load bundler WASM build:', fallbackError);
                throw new Error('Could not load WASM module in browser environment');
            }
        }
    }

    /**
     * Load WASM module in Node.js environment
     */
    private async loadWasmNode(): Promise<CodeMeshWasm> {
        this.logDebug('Loading WASM for Node.js environment');

        try {
            // Check if the Node.js WASM build exists
            const wasmPath = path.join(__dirname, '../wasm/node/code_mesh_wasm.js');
            
            try {
                await fs.access(wasmPath);
            } catch {
                throw new Error(`WASM module not found at ${wasmPath}. Run 'npm run build:wasm:node' first.`);
            }

            const wasmModule = await import(wasmPath);
            wasmModule.init();
            
            this.wasmModule = wasmModule as CodeMeshWasm;
            this.logDebug('WASM module loaded successfully in Node.js');
            return this.wasmModule;
        } catch (error) {
            this.logError('Failed to load Node.js WASM build:', error);
            throw error;
        }
    }

    /**
     * Load WASM module in web worker environment
     */
    private async loadWasmWorker(): Promise<CodeMeshWasm> {
        this.logDebug('Loading WASM for web worker environment');

        try {
            // Import scripts for web worker
            if (typeof importScripts === 'function') {
                importScripts('../wasm/web/code_mesh_wasm.js');
                
                // @ts-ignore - Global scope in web worker
                const wasmModule = globalThis.wasm_bindgen || self.wasm_bindgen;
                
                if (wasmModule) {
                    await wasmModule('../wasm/web/code_mesh_wasm_bg.wasm');
                    wasmModule.init();
                    
                    this.wasmModule = wasmModule as CodeMeshWasm;
                    this.logDebug('WASM module loaded successfully in web worker');
                    return this.wasmModule;
                }
            }
            
            throw new Error('Could not load WASM in web worker environment');
        } catch (error) {
            this.logError('Failed to load WASM in web worker:', error);
            throw error;
        }
    }

    /**
     * Create a Code Mesh instance
     */
    async createCodeMesh(config?: any): Promise<any> {
        const wasm = await this.loadWasm();
        
        // Create simplified Code Mesh instance
        return new wasm.CodeMesh();
    }

    /**
     * Get available providers
     */
    async getProviders(): Promise<string[]> {
        // Mock implementation since the actual WASM module is simplified
        return ['openai', 'anthropic', 'local'];
    }

    /**
     * Get available models for a provider
     */
    async getModels(provider: string): Promise<string[]> {
        // Mock implementation since the actual WASM module is simplified
        const models: Record<string, string[]> = {
            'openai': ['gpt-4', 'gpt-3.5-turbo'],
            'anthropic': ['claude-3-opus', 'claude-3-sonnet'],
            'local': ['llama', 'mistral']
        };
        return models[provider] || [];
    }

    /**
     * Get platform information
     */
    async getPlatformInfo(): Promise<any> {
        // Mock implementation since the actual WASM module is simplified
        return {
            platform: isBrowser ? 'browser' : 'node',
            arch: isNode ? arch() : 'unknown',
            os: isNode ? platform() : 'unknown'
        };
    }

    /**
     * Check WASM feature support
     */
    async checkFeatureSupport(): Promise<any> {
        // Mock implementation since the actual WASM module is simplified
        return {
            wasm: WasmRunner.isWasmSupported(),
            webWorkers: isBrowser && typeof Worker !== 'undefined',
            serviceWorkers: isBrowser && typeof navigator !== 'undefined' && 'serviceWorker' in navigator
        };
    }

    /**
     * Create a web worker (browser only)
     */
    async createWorker(scriptUrl: string): Promise<any> {
        if (!isBrowser) {
            throw new Error('Web workers are only available in browser environment');
        }
        
        // Mock implementation since the actual WASM module is simplified
        return new Worker(scriptUrl);
    }

    /**
     * Check if WASM is supported in current environment
     */
    static isWasmSupported(): boolean {
        if (isBrowser) {
            return typeof WebAssembly !== 'undefined' && typeof WebAssembly.instantiate === 'function';
        } else if (isNode) {
            try {
                return typeof WebAssembly !== 'undefined';
            } catch {
                return false;
            }
        }
        return false;
    }

    /**
     * Get recommended configuration for current environment
     */
    static getRecommendedConfig(): WasmConfig {
        if (isBrowser) {
            return {
                enablePerformanceMonitoring: true,
                maxMemoryMB: 256,
                useWebWorkers: true,
                enableOffline: true,
                debug: false
            };
        } else if (isNode) {
            return {
                enablePerformanceMonitoring: false,
                maxMemoryMB: 1024,
                useWebWorkers: false,
                enableOffline: false,
                debug: false
            };
        } else {
            return {
                enablePerformanceMonitoring: false,
                maxMemoryMB: 512,
                useWebWorkers: false,
                enableOffline: false,
                debug: true
            };
        }
    }

    /**
     * Optimize WASM performance
     */
    async optimizePerformance(): Promise<void> {
        if (isBrowser && this.wasmModule) {
            try {
                // Force garbage collection if available
                if (typeof (globalThis as any).gc === 'function') {
                    (globalThis as any).gc();
                }
                
                // Optimize memory usage
                const optimizeMemory = (this.wasmModule as any).optimize_memory;
                if (optimizeMemory) {
                    await optimizeMemory();
                }
            } catch (error) {
                this.logDebug('Performance optimization failed:', error);
            }
        }
    }

    private logDebug(message: string, ...args: any[]): void {
        if (this.config.debug) {
            console.debug(`[WasmRunner] ${message}`, ...args);
        }
    }

    private logError(message: string, ...args: any[]): void {
        console.error(`[WasmRunner] ${message}`, ...args);
    }
}

/**
 * Run Code Mesh with CLI arguments (for NPX usage)
 */
export async function runWasm(args: string[]): Promise<void> {
    try {
        console.log('🦀 Starting Code Mesh in WASM mode...');
        
        const config = WasmRunner.getRecommendedConfig();
        config.debug = args.includes('--debug') || args.includes('-d');
        
        const runner = new WasmRunner(config);
        
        // Check WASM support
        if (!WasmRunner.isWasmSupported()) {
            throw new Error('WebAssembly is not supported in this environment');
        }
        
        // Load WASM module
        console.log('📦 Loading WebAssembly module...');
        await runner.loadWasm();
        
        // Get platform info
        const platformInfo = await runner.getPlatformInfo();
        console.log('🔍 Platform:', platformInfo);
        
        // Check feature support
        const features = await runner.checkFeatureSupport();
        console.log('⚡ WASM Features:', features);
        
        // Create Code Mesh instance
        const codeMesh = await runner.createCodeMesh({
            useBrowserStorage: platformInfo.is_browser,
            enableOffline: platformInfo.is_browser,
            useWebWorkers: platformInfo.is_browser && features.threads,
        });
        
        await codeMesh.initialize();
        
        console.log('✅ Code Mesh WASM initialized successfully!');
        console.log(`📋 Session ID: ${codeMesh.session_id()}`);
        
        // Handle CLI commands
        if (args.length === 0) {
            console.log('💡 Try: npx code-mesh --help');
            return;
        }
        
        const command = args[0];
        
        switch (command) {
            case 'providers':
                const providers = await runner.getProviders();
                console.log('🤖 Available providers:', providers.join(', '));
                break;
                
            case 'models':
                if (args.length < 2) {
                    console.error('❌ Usage: npx code-mesh models <provider>');
                    process.exit(1);
                }
                const models = await runner.getModels(args[1]);
                console.log(`🧠 Models for ${args[1]}:`, models.join(', '));
                break;
                
            case 'chat':
                console.log('💬 Interactive chat mode not yet implemented in WASM');
                break;
                
            case 'info':
                console.log('ℹ️  Code Mesh WASM Information:');
                console.log('   Platform:', platformInfo);
                console.log('   Features:', features);
                console.log('   Memory:', await codeMesh.get_memory_usage());
                break;
                
            default:
                console.log(`❓ Unknown command: ${command}`);
                console.log('📖 Available commands: providers, models, chat, info');
        }
        
    } catch (error) {
        console.error('❌ Failed to run Code Mesh in WASM mode:', error);
        process.exit(1);
    }
}

// Export default instance for convenience
export const defaultWasmRunner = new WasmRunner();