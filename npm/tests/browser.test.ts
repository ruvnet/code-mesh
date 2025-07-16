/**
 * Browser-specific tests for Code Mesh
 */

import { CodeMeshBrowser, createCodeMesh } from '../src/browser';

// Mock environment to be browser-like
Object.defineProperty(window, 'isSecureContext', {
  value: true,
  writable: true
});

describe('CodeMeshBrowser', () => {
  let codeMesh: CodeMeshBrowser;

  beforeEach(() => {
    codeMesh = new CodeMeshBrowser({ debug: true });
  });

  afterEach(() => {
    jest.clearAllMocks();
  });

  describe('initialization', () => {
    it('should create a CodeMeshBrowser instance', () => {
      expect(codeMesh).toBeInstanceOf(CodeMeshBrowser);
    });

    it('should initialize successfully', async () => {
      await expect(codeMesh.initialize()).resolves.not.toThrow();
    });

    it('should check browser compatibility', async () => {
      // This should not throw with our mocked environment
      await expect(codeMesh.initialize()).resolves.not.toThrow();
    });
  });

  describe('session management', () => {
    beforeEach(async () => {
      await codeMesh.initialize();
    });

    it('should create a new session', async () => {
      const sessionId = await codeMesh.createSession();
      expect(typeof sessionId).toBe('string');
      expect(sessionId.length).toBeGreaterThan(0);
    });

    it('should send and receive messages', async () => {
      const response = await codeMesh.sendMessage('Hello, world!');
      expect(typeof response).toBe('string');
      expect(response).toBe('Test response');
    });

    it('should get message history', async () => {
      await codeMesh.sendMessage('Test message');
      const messages = await codeMesh.getMessages();
      expect(Array.isArray(messages)).toBe(true);
    });

    it('should save and load sessions', async () => {
      await codeMesh.sendMessage('Test message');
      await codeMesh.saveSession();
      
      const sessions = await codeMesh.listSessions();
      expect(Array.isArray(sessions)).toBe(true);
    });

    it('should delete sessions', async () => {
      const sessionId = await codeMesh.createSession();
      await codeMesh.deleteSession(sessionId);
      // Should not throw
    });
  });

  describe('clipboard operations', () => {
    beforeEach(async () => {
      await codeMesh.initialize();
    });

    it('should get clipboard content', async () => {
      const content = await codeMesh.getClipboard();
      expect(typeof content).toBe('string');
    });

    it('should set clipboard content', async () => {
      await expect(codeMesh.setClipboard('test content')).resolves.not.toThrow();
    });
  });

  describe('performance and monitoring', () => {
    beforeEach(async () => {
      await codeMesh.initialize();
    });

    it('should get performance metrics', () => {
      const metrics = codeMesh.getPerformanceMetrics();
      expect(metrics).toBeDefined();
    });

    it('should get memory usage', () => {
      const memoryUsage = codeMesh.getMemoryUsage();
      expect(memoryUsage).toBeDefined();
    });
  });

  describe('web worker support', () => {
    beforeEach(async () => {
      await codeMesh.initialize();
    });

    it('should create web workers', async () => {
      const worker = await codeMesh.createWorker('/worker.js');
      expect(worker).toBeDefined();
      expect(worker.post_message).toBeDefined();
      expect(worker.terminate).toBeDefined();
    });
  });

  describe('data import/export', () => {
    beforeEach(async () => {
      await codeMesh.initialize();
    });

    it('should export session as JSON', async () => {
      await codeMesh.sendMessage('Test message');
      const exported = await codeMesh.exportSession('json');
      expect(typeof exported).toBe('string');
      
      // Should be valid JSON
      expect(() => JSON.parse(exported)).not.toThrow();
    });

    it('should export session as Markdown', async () => {
      await codeMesh.sendMessage('Test message');
      const exported = await codeMesh.exportSession('markdown');
      expect(typeof exported).toBe('string');
      expect(exported).toContain('##');
    });

    it('should import session data', async () => {
      const sessionData = JSON.stringify([
        { role: 'user', content: 'Hello', timestamp: new Date().toISOString(), id: '1' }
      ]);
      
      await expect(codeMesh.importSession(sessionData)).resolves.not.toThrow();
    });
  });

  describe('provider and model queries', () => {
    beforeEach(async () => {
      await codeMesh.initialize();
    });

    it('should get available providers', async () => {
      const providers = await codeMesh.getProviders();
      expect(Array.isArray(providers)).toBe(true);
      expect(providers).toContain('anthropic');
    });

    it('should get models for providers', async () => {
      const models = await codeMesh.getModels('anthropic');
      expect(Array.isArray(models)).toBe(true);
      expect(models.length).toBeGreaterThan(0);
    });
  });

  describe('PWA functionality', () => {
    beforeEach(async () => {
      await codeMesh.initialize();
    });

    it('should handle PWA installation', async () => {
      // Mock the deferred prompt
      (window as any).deferredPrompt = {
        prompt: jest.fn(),
        userChoice: Promise.resolve({ outcome: 'accepted' })
      };

      await expect(codeMesh.installPWA()).resolves.not.toThrow();
    });

    it('should handle missing PWA prompt', async () => {
      (window as any).deferredPrompt = null;
      await expect(codeMesh.installPWA()).resolves.not.toThrow();
    });
  });

  describe('factory functions', () => {
    it('should create CodeMesh instance via factory', () => {
      const instance = createCodeMesh();
      expect(instance).toBeInstanceOf(CodeMeshBrowser);
    });

    it('should create instance with custom config', () => {
      const config = { maxMemoryMB: 1024 };
      const instance = createCodeMesh(config);
      expect(instance).toBeInstanceOf(CodeMeshBrowser);
    });
  });
});