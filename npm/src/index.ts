/**
 * Code Mesh - Universal TypeScript/JavaScript API
 * 
 * This module provides a unified interface for Code Mesh that works
 * across Node.js, browsers, and web workers with automatic WASM loading.
 */

import { WasmRunner } from './wasm-runner';

// Re-export browser-specific functionality
export { CodeMeshBrowser } from './browser';

// Re-export WASM runner
export { WasmRunner } from './wasm-runner';

// Environment detection
export const Environment = {
    isBrowser: typeof window !== 'undefined',
    isNode: typeof process !== 'undefined' && process.versions && process.versions.node,
    isWebWorker: typeof WorkerGlobalScope !== 'undefined' && typeof importScripts === 'function',
    isServiceWorker: typeof ServiceWorkerGlobalScope !== 'undefined'
};

/**
 * Universal Code Mesh interface that adapts to the current environment
 */
export class CodeMesh {
    private wasmRunner: WasmRunner;
    private instance: any = null;
    private initialized: boolean = false;

    constructor(config: any = {}) {
        this.wasmRunner = new WasmRunner(config);
    }

    /**
     * Initialize Code Mesh for the current environment
     */
    async initialize(config?: any): Promise<void> {
        if (this.initialized) {
            return;
        }

        try {
            // Load WASM module
            await this.wasmRunner.loadWasm();
            
            // Create instance with environment-specific configuration
            const instanceConfig = {
                useBrowserStorage: Environment.isBrowser,
                enableOffline: Environment.isBrowser,
                useWebWorkers: Environment.isBrowser,
                ...config
            };
            
            this.instance = await this.wasmRunner.createCodeMesh(instanceConfig);
            await this.instance.initialize();
            
            this.initialized = true;
        } catch (error) {
            throw new Error(`Failed to initialize Code Mesh: ${error}`);
        }
    }

    /**
     * Ensure the instance is initialized
     */
    private ensureInitialized(): void {
        if (!this.initialized || !this.instance) {
            throw new Error('Code Mesh not initialized. Call initialize() first.');
        }
    }

    /**
     * Get current session ID
     */
    getSessionId(): string {
        this.ensureInitialized();
        return this.instance.session_id();
    }

    /**
     * Add a user message to the current session
     */
    async addMessage(content: string): Promise<void> {
        this.ensureInitialized();
        await this.instance.add_user_message(content);
    }

    /**
     * Generate AI response
     */
    async generateResponse(model: string, apiKey?: string): Promise<string> {
        this.ensureInitialized();
        return await this.instance.generate_response(model, apiKey);
    }

    /**
     * Send message and get response in one call
     */
    async chat(message: string, model: string = 'claude-3-sonnet-20240229', apiKey?: string): Promise<string> {
        await this.addMessage(message);
        return await this.generateResponse(model, apiKey);
    }

    /**
     * Get all messages in the current session
     */
    getMessages(): any[] {
        this.ensureInitialized();
        return this.instance.get_messages();
    }

    /**
     * Clear the current session
     */
    clearSession(): void {
        this.ensureInitialized();
        this.instance.clear_session();
    }

    /**
     * Save current session (browser only)
     */
    async saveSession(): Promise<void> {
        this.ensureInitialized();
        if (!Environment.isBrowser) {
            throw new Error('Session saving only available in browser environment');
        }
        await this.instance.save_session();
    }

    /**
     * Load a saved session (browser only)
     */
    async loadSession(sessionId: string): Promise<void> {
        this.ensureInitialized();
        if (!Environment.isBrowser) {
            throw new Error('Session loading only available in browser environment');
        }
        await this.instance.load_session(sessionId);
    }

    /**
     * List saved sessions (browser only)
     */
    async listSessions(): Promise<any[]> {
        this.ensureInitialized();
        if (!Environment.isBrowser) {
            throw new Error('Session listing only available in browser environment');
        }
        return await this.instance.list_sessions();
    }

    /**
     * Delete a session (browser only)
     */
    async deleteSession(sessionId: string): Promise<void> {
        this.ensureInitialized();
        if (!Environment.isBrowser) {
            throw new Error('Session deletion only available in browser environment');
        }
        await this.instance.delete_session(sessionId);
    }

    /**
     * Get performance metrics
     */
    getPerformanceMetrics(): any {
        this.ensureInitialized();
        return this.instance.get_performance_metrics();
    }

    /**
     * Get memory usage statistics
     */
    getMemoryUsage(): any {
        this.ensureInitialized();
        return this.instance.get_memory_usage();
    }

    /**
     * Get clipboard content (browser only)
     */
    async getClipboard(): Promise<string> {
        this.ensureInitialized();
        if (!Environment.isBrowser) {
            throw new Error('Clipboard access only available in browser environment');
        }
        return await this.instance.get_clipboard();
    }

    /**
     * Set clipboard content (browser only)
     */
    async setClipboard(text: string): Promise<void> {
        this.ensureInitialized();
        if (!Environment.isBrowser) {
            throw new Error('Clipboard access only available in browser environment');
        }
        await this.instance.set_clipboard(text);
    }

    /**
     * Get available providers
     */
    async getProviders(): Promise<string[]> {
        return await this.wasmRunner.getProviders();
    }

    /**
     * Get available models for a provider
     */
    async getModels(provider: string): Promise<string[]> {
        return await this.wasmRunner.getModels(provider);
    }

    /**
     * Get platform information
     */
    async getPlatformInfo(): Promise<any> {
        return await this.wasmRunner.getPlatformInfo();
    }

    /**
     * Check WASM feature support
     */
    async checkFeatureSupport(): Promise<any> {
        return await this.wasmRunner.checkFeatureSupport();
    }

    /**
     * Create a web worker (browser only)
     */
    async createWorker(scriptUrl: string): Promise<any> {
        if (!Environment.isBrowser) {
            throw new Error('Web workers only available in browser environment');
        }
        return await this.wasmRunner.createWorker(scriptUrl);
    }
}

/**
 * Create a new Code Mesh instance
 */
export function createInstance(config?: any): CodeMesh {
    return new CodeMesh(config);
}

/**
 * Quick start function - creates and initializes Code Mesh in one call
 */
export async function quickStart(config?: any): Promise<CodeMesh> {
    const instance = new CodeMesh(config);
    await instance.initialize();
    return instance;
}

/**
 * Utility functions
 */
export const Utils = {
    /**
     * Check if WASM is supported in current environment
     */
    isWasmSupported(): boolean {
        return WasmRunner.isWasmSupported();
    },

    /**
     * Get recommended configuration for current environment
     */
    getRecommendedConfig(): any {
        return WasmRunner.getRecommendedConfig();
    },

    /**
     * Format bytes into human-readable format
     */
    formatBytes(bytes: number): string {
        const units = ['B', 'KB', 'MB', 'GB', 'TB'];
        const threshold = 1024;
        
        if (bytes === 0) return '0 B';
        
        let size = bytes;
        let unitIndex = 0;
        
        while (size >= threshold && unitIndex < units.length - 1) {
            size /= threshold;
            unitIndex++;
        }
        
        return `${size.toFixed(1)} ${units[unitIndex]}`;
    },

    /**
     * Validate API key format
     */
    validateApiKey(apiKey: string, provider: string): boolean {
        switch (provider) {
            case 'anthropic':
                return apiKey.startsWith('sk-ant-');
            case 'openai':
                return apiKey.startsWith('sk-');
            case 'mistral':
                return apiKey.length > 20; // Basic length check
            default:
                return apiKey.length > 10; // Generic validation
        }
    }
};

/**
 * Type definitions for better TypeScript support
 */
export interface CodeMeshMessage {
    role: 'user' | 'assistant' | 'system';
    content: string;
    timestamp: string;
    id: string;
}

export interface CodeMeshSession {
    id: string;
    messages: CodeMeshMessage[];
    created_at: string;
    updated_at: string;
}

export interface PerformanceMetrics {
    marks: Record<string, number>;
    measures: Record<string, number>;
    memory_usage?: {
        used: number;
        total: number;
    };
}

export interface PlatformInfo {
    is_browser: boolean;
    is_node: boolean;
    user_agent?: string;
    platform?: string;
    language?: string;
}

export interface WasmFeatures {
    simd: boolean;
    threads: boolean;
    bulk_memory: boolean;
    reference_types: boolean;
}

// Export types (remove duplicate export)
export type {
    CodeMeshMessage as CodeMeshMessageType,
    CodeMeshSession as CodeMeshSessionType,
    PerformanceMetrics as PerformanceMetricsType,
    PlatformInfo as PlatformInfoType,
    WasmFeatures as WasmFeaturesType
};

// Default export
export default CodeMesh;