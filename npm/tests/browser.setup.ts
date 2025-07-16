/**
 * Jest setup for browser tests
 */

import 'jest-environment-jsdom';

// Mock WebAssembly for browser tests
Object.defineProperty(window, 'WebAssembly', {
  value: {
    instantiate: jest.fn().mockResolvedValue({
      instance: {
        exports: {}
      }
    }),
    compile: jest.fn().mockResolvedValue({}),
    validate: jest.fn().mockReturnValue(true),
    Module: jest.fn(),
    Instance: jest.fn(),
    Memory: jest.fn(),
    Table: jest.fn(),
    Global: jest.fn(),
    CompileError: Error,
    LinkError: Error,
    RuntimeError: Error
  },
  writable: true
});

// Mock IndexedDB
const mockIDB = {
  open: jest.fn().mockImplementation(() => {
    const request = {
      onsuccess: null,
      onerror: null,
      onupgradeneeded: null,
      result: {
        transaction: jest.fn().mockReturnValue({
          objectStore: jest.fn().mockReturnValue({
            add: jest.fn(),
            put: jest.fn(),
            get: jest.fn(),
            delete: jest.fn(),
            clear: jest.fn(),
            count: jest.fn(),
            getAll: jest.fn(),
            createIndex: jest.fn(),
            index: jest.fn()
          })
        }),
        createObjectStore: jest.fn().mockReturnValue({
          createIndex: jest.fn()
        }),
        objectStoreNames: {
          contains: jest.fn().mockReturnValue(false)
        }
      }
    };
    setTimeout(() => {
      if (request.onsuccess) request.onsuccess({ target: request });
    }, 0);
    return request;
  })
};

Object.defineProperty(window, 'indexedDB', {
  value: mockIDB,
  writable: true
});

// Mock localStorage
const mockStorage = {
  getItem: jest.fn(),
  setItem: jest.fn(),
  removeItem: jest.fn(),
  clear: jest.fn(),
  length: 0,
  key: jest.fn()
};

Object.defineProperty(window, 'localStorage', {
  value: mockStorage,
  writable: true
});

// Mock navigator
Object.defineProperty(window.navigator, 'serviceWorker', {
  value: {
    register: jest.fn().mockResolvedValue({
      active: null,
      installing: null,
      waiting: null,
      update: jest.fn(),
      unregister: jest.fn().mockResolvedValue(true)
    })
  },
  writable: true
});

Object.defineProperty(window.navigator, 'clipboard', {
  value: {
    readText: jest.fn().mockResolvedValue(''),
    writeText: jest.fn().mockResolvedValue(undefined)
  },
  writable: true
});

Object.defineProperty(window.navigator, 'permissions', {
  value: {
    query: jest.fn().mockResolvedValue({ state: 'granted' })
  },
  writable: true
});

// Mock crypto
Object.defineProperty(window, 'crypto', {
  value: {
    getRandomValues: jest.fn((arr) => {
      for (let i = 0; i < arr.length; i++) {
        arr[i] = Math.floor(Math.random() * 256);
      }
      return arr;
    }),
    subtle: {
      generateKey: jest.fn().mockResolvedValue({}),
      encrypt: jest.fn().mockResolvedValue(new ArrayBuffer(16)),
      decrypt: jest.fn().mockResolvedValue(new ArrayBuffer(16)),
      digest: jest.fn().mockResolvedValue(new ArrayBuffer(32))
    }
  },
  writable: true
});

// Mock Worker
Object.defineProperty(window, 'Worker', {
  value: jest.fn().mockImplementation((scriptURL) => ({
    postMessage: jest.fn(),
    terminate: jest.fn(),
    onmessage: null,
    onerror: null,
    addEventListener: jest.fn(),
    removeEventListener: jest.fn(),
    dispatchEvent: jest.fn()
  })),
  writable: true
});

// Mock performance
Object.defineProperty(window, 'performance', {
  value: {
    now: jest.fn(() => Date.now()),
    mark: jest.fn(),
    measure: jest.fn(),
    clearMarks: jest.fn(),
    clearMeasures: jest.fn(),
    getEntries: jest.fn(() => []),
    memory: {
      usedJSHeapSize: 1000000,
      totalJSHeapSize: 2000000,
      jsHeapSizeLimit: 4000000
    }
  },
  writable: true
});

// Mock fetch
global.fetch = jest.fn().mockImplementation((url) => {
  return Promise.resolve({
    ok: true,
    status: 200,
    json: () => Promise.resolve({}),
    text: () => Promise.resolve(''),
    arrayBuffer: () => Promise.resolve(new ArrayBuffer(0)),
    blob: () => Promise.resolve(new Blob())
  });
});

// Mock URL
Object.defineProperty(window, 'URL', {
  value: class MockURL {
    constructor(url: string) {
      this.href = url;
      this.protocol = 'https:';
      this.hostname = 'localhost';
      this.port = '';
      this.pathname = '/';
      this.search = '';
      this.hash = '';
      this.searchParams = {
        set: jest.fn(),
        get: jest.fn(),
        append: jest.fn(),
        delete: jest.fn(),
        has: jest.fn()
      };
    }
    href: string;
    protocol: string;
    hostname: string;
    port: string;
    pathname: string;
    search: string;
    hash: string;
    searchParams: any;
  },
  writable: true
});

// Suppress console output during tests
global.console = {
  ...console,
  debug: jest.fn(),
  log: jest.fn(),
  warn: jest.fn(),
  error: jest.fn()
};