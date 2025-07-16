/**
 * Tests for WASM runner functionality
 */

import { WasmRunner } from '../src/wasm-runner';

describe('WasmRunner', () => {
  let wasmRunner: WasmRunner;

  beforeEach(() => {
    wasmRunner = new WasmRunner({ debug: true });
  });

  afterEach(() => {
    jest.clearAllMocks();
  });

  describe('initialization', () => {
    it('should create a WasmRunner instance', () => {
      expect(wasmRunner).toBeInstanceOf(WasmRunner);
    });

    it('should load WASM module', async () => {
      const wasmModule = await wasmRunner.loadWasm();
      expect(wasmModule).toBeDefined();
      expect(wasmModule.init).toHaveBeenCalled();
    });

    it('should return the same WASM module on subsequent calls', async () => {
      const module1 = await wasmRunner.loadWasm();
      const module2 = await wasmRunner.loadWasm();
      expect(module1).toBe(module2);
    });
  });

  describe('Code Mesh creation', () => {
    beforeEach(async () => {
      await wasmRunner.loadWasm();
    });

    it('should create a Code Mesh instance', async () => {
      const codeMesh = await wasmRunner.createCodeMesh();
      expect(codeMesh).toBeDefined();
      expect(codeMesh.session_id).toBeDefined();
    });

    it('should create a Code Mesh instance with custom config', async () => {
      const config = {
        useBrowserStorage: false,
        enableOffline: true,
        maxMemoryMB: 1024
      };

      const codeMesh = await wasmRunner.createCodeMesh(config);
      expect(codeMesh).toBeDefined();
    });
  });

  describe('provider and model queries', () => {
    beforeEach(async () => {
      await wasmRunner.loadWasm();
    });

    it('should get available providers', async () => {
      const providers = await wasmRunner.getProviders();
      expect(Array.isArray(providers)).toBe(true);
      expect(providers.length).toBeGreaterThan(0);
    });

    it('should get models for a provider', async () => {
      const models = await wasmRunner.getModels('anthropic');
      expect(Array.isArray(models)).toBe(true);
      expect(models.length).toBeGreaterThan(0);
    });
  });

  describe('platform detection', () => {
    beforeEach(async () => {
      await wasmRunner.loadWasm();
    });

    it('should get platform information', async () => {
      const platformInfo = await wasmRunner.getPlatformInfo();
      expect(platformInfo).toBeDefined();
      expect(typeof platformInfo.is_browser).toBe('boolean');
      expect(typeof platformInfo.is_node).toBe('boolean');
    });

    it('should check feature support', async () => {
      const features = await wasmRunner.checkFeatureSupport();
      expect(features).toBeDefined();
      expect(typeof features.simd).toBe('boolean');
      expect(typeof features.threads).toBe('boolean');
    });
  });

  describe('static methods', () => {
    it('should check WASM support', () => {
      const isSupported = WasmRunner.isWasmSupported();
      expect(typeof isSupported).toBe('boolean');
    });

    it('should provide recommended config', () => {
      const config = WasmRunner.getRecommendedConfig();
      expect(config).toBeDefined();
      expect(typeof config.maxMemoryMB).toBe('number');
      expect(typeof config.enablePerformanceMonitoring).toBe('boolean');
    });
  });

  describe('performance optimization', () => {
    beforeEach(async () => {
      await wasmRunner.loadWasm();
    });

    it('should optimize performance without errors', async () => {
      await expect(wasmRunner.optimizePerformance()).resolves.not.toThrow();
    });
  });
});