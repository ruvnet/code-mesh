/**
 * Browser-specific entry point for Code Mesh
 * Optimized for web environments with PWA support
 */

import { WasmRunner } from './wasm-runner';

// Browser-specific configuration
const BROWSER_CONFIG = {
    enablePerformanceMonitoring: true,
    maxMemoryMB: 256,
    useWebWorkers: true,
    enableOffline: true,
    debug: false
};

/**
 * Code Mesh browser interface
 */
export class CodeMeshBrowser {
    private wasmRunner: WasmRunner;
    private codeMesh: any = null;
    private serviceWorker: ServiceWorkerRegistration | null = null;

    constructor(config: any = {}) {
        this.wasmRunner = new WasmRunner({
            ...BROWSER_CONFIG,
            ...config
        });
    }

    /**
     * Initialize Code Mesh for browser usage
     */
    async initialize(): Promise<void> {
        try {
            // Check browser compatibility
            this.checkBrowserCompatibility();
            
            // Load WASM module
            await this.wasmRunner.loadWasm();
            
            // Create Code Mesh instance with browser-optimized config
            this.codeMesh = await this.wasmRunner.createCodeMesh({
                useBrowserStorage: true,
                enableOffline: true,
                useWebWorkers: this.supportsWebWorkers(),
                authProvider: 'browser'
            });
            
            await this.codeMesh.initialize();
            
            // Register service worker for offline support
            if (this.supportsServiceWorkers()) {
                await this.registerServiceWorker();
            }
            
            console.log('✅ Code Mesh browser initialized successfully!');
        } catch (error) {
            console.error('❌ Failed to initialize Code Mesh in browser:', error);
            throw error;
        }
    }

    /**
     * Check if the browser supports required features
     */
    private checkBrowserCompatibility(): void {
        const errors: string[] = [];

        if (!WasmRunner.isWasmSupported()) {
            errors.push('WebAssembly not supported');
        }

        if (typeof indexedDB === 'undefined') {
            errors.push('IndexedDB not supported');
        }

        if (typeof fetch === 'undefined') {
            errors.push('Fetch API not supported');
        }

        if (!window.isSecureContext) {
            console.warn('⚠️  Not running in secure context - some features may be limited');
        }

        if (errors.length > 0) {
            throw new Error(`Browser compatibility issues: ${errors.join(', ')}`);
        }
    }

    /**
     * Check if web workers are supported
     */
    private supportsWebWorkers(): boolean {
        return typeof Worker !== 'undefined';
    }

    /**
     * Check if service workers are supported
     */
    private supportsServiceWorkers(): boolean {
        return 'serviceWorker' in navigator;
    }

    /**
     * Register service worker for offline support
     */
    private async registerServiceWorker(): Promise<void> {
        try {
            this.serviceWorker = await navigator.serviceWorker.register('/sw.js');
            console.log('✅ Service worker registered for offline support');
        } catch (error) {
            console.warn('⚠️  Failed to register service worker:', error);
        }
    }

    /**
     * Create a new chat session
     */
    async createSession(): Promise<string> {
        if (!this.codeMesh) {
            throw new Error('Code Mesh not initialized');
        }
        
        this.codeMesh.clear_session();
        return this.codeMesh.session_id();
    }

    /**
     * Send a message and get AI response
     */
    async sendMessage(message: string, model: string = 'claude-3-sonnet-20240229', apiKey?: string): Promise<string> {
        if (!this.codeMesh) {
            throw new Error('Code Mesh not initialized');
        }

        try {
            await this.codeMesh.add_user_message(message);
            const response = await this.codeMesh.generate_response(model, apiKey);
            return response;
        } catch (error) {
            console.error('Failed to send message:', error);
            throw error;
        }
    }

    /**
     * Get chat history
     */
    async getMessages(): Promise<any[]> {
        if (!this.codeMesh) {
            throw new Error('Code Mesh not initialized');
        }
        
        return this.codeMesh.get_messages();
    }

    /**
     * Save current session
     */
    async saveSession(): Promise<void> {
        if (!this.codeMesh) {
            throw new Error('Code Mesh not initialized');
        }
        
        await this.codeMesh.save_session();
    }

    /**
     * Load a saved session
     */
    async loadSession(sessionId: string): Promise<void> {
        if (!this.codeMesh) {
            throw new Error('Code Mesh not initialized');
        }
        
        await this.codeMesh.load_session(sessionId);
    }

    /**
     * List all saved sessions
     */
    async listSessions(): Promise<any[]> {
        if (!this.codeMesh) {
            throw new Error('Code Mesh not initialized');
        }
        
        return this.codeMesh.list_sessions();
    }

    /**
     * Delete a session
     */
    async deleteSession(sessionId: string): Promise<void> {
        if (!this.codeMesh) {
            throw new Error('Code Mesh not initialized');
        }
        
        await this.codeMesh.delete_session(sessionId);
    }

    /**
     * Get clipboard content
     */
    async getClipboard(): Promise<string> {
        if (!this.codeMesh) {
            throw new Error('Code Mesh not initialized');
        }
        
        return this.codeMesh.get_clipboard();
    }

    /**
     * Set clipboard content
     */
    async setClipboard(text: string): Promise<void> {
        if (!this.codeMesh) {
            throw new Error('Code Mesh not initialized');
        }
        
        await this.codeMesh.set_clipboard(text);
    }

    /**
     * Get performance metrics
     */
    getPerformanceMetrics(): any {
        if (!this.codeMesh) {
            throw new Error('Code Mesh not initialized');
        }
        
        return this.codeMesh.get_performance_metrics();
    }

    /**
     * Get memory usage
     */
    getMemoryUsage(): any {
        if (!this.codeMesh) {
            throw new Error('Code Mesh not initialized');
        }
        
        return this.codeMesh.get_memory_usage();
    }

    /**
     * Create a web worker for background processing
     */
    async createWorker(scriptUrl: string): Promise<any> {
        return this.wasmRunner.createWorker(scriptUrl);
    }

    /**
     * Install as PWA
     */
    async installPWA(): Promise<void> {
        // @ts-ignore
        const deferredPrompt = window.deferredPrompt;
        
        if (deferredPrompt) {
            deferredPrompt.prompt();
            const { outcome } = await deferredPrompt.userChoice;
            console.log(`PWA install outcome: ${outcome}`);
            // @ts-ignore
            window.deferredPrompt = null;
        } else {
            console.log('PWA installation not available');
        }
    }

    /**
     * Export session data
     */
    async exportSession(format: 'json' | 'markdown' = 'json'): Promise<string> {
        const messages = await this.getMessages();
        
        if (format === 'markdown') {
            return messages.map(msg => `## ${msg.role}\n\n${msg.content}\n`).join('\n');
        } else {
            return JSON.stringify(messages, null, 2);
        }
    }

    /**
     * Import session data
     */
    async importSession(data: string): Promise<void> {
        if (!this.codeMesh) {
            throw new Error('Code Mesh not initialized');
        }
        
        try {
            const messages = JSON.parse(data);
            this.codeMesh.clear_session();
            
            for (const message of messages) {
                if (message.role === 'user') {
                    await this.codeMesh.add_user_message(message.content);
                }
                // Assistant messages are handled differently in the core
            }
        } catch (error) {
            throw new Error(`Failed to import session: ${error}`);
        }
    }

    /**
     * Get available providers
     */
    async getProviders(): Promise<string[]> {
        return this.wasmRunner.getProviders();
    }

    /**
     * Get available models for a provider
     */
    async getModels(provider: string): Promise<string[]> {
        return this.wasmRunner.getModels(provider);
    }
}

/**
 * Create global Code Mesh instance for browser usage
 */
export function createCodeMesh(config?: any): CodeMeshBrowser {
    return new CodeMeshBrowser(config);
}

/**
 * Initialize Code Mesh and attach to window for global access
 */
export async function initializeGlobal(config?: any): Promise<CodeMeshBrowser> {
    const codeMesh = new CodeMeshBrowser(config);
    await codeMesh.initialize();
    
    // Attach to window for global access
    (window as any).CodeMesh = codeMesh;
    
    // Dispatch ready event
    window.dispatchEvent(new CustomEvent('codemesh-ready', { detail: codeMesh }));
    
    return codeMesh;
}

// Auto-initialize if in browser and not in a module context
if (typeof window !== 'undefined' && !window.location.search.includes('no-auto-init')) {
    // Initialize on DOM content loaded
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', () => {
            initializeGlobal().catch(console.error);
        });
    } else {
        initializeGlobal().catch(console.error);
    }
}

// PWA install prompt handling
if (typeof window !== 'undefined') {
    window.addEventListener('beforeinstallprompt', (e) => {
        e.preventDefault();
        // @ts-ignore
        window.deferredPrompt = e;
        
        // Show install button or notification
        const installButton = document.getElementById('install-pwa');
        if (installButton) {
            installButton.style.display = 'block';
        }
    });
}