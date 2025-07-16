/**
 * Jest setup for Node.js tests
 */

// Mock WASM module for testing
global.WebAssembly = {
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
} as any;

// Mock console methods for cleaner test output
global.console = {
  ...console,
  debug: jest.fn(),
  log: jest.fn(),
  warn: jest.fn(),
  error: jest.fn()
};