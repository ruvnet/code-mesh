module.exports = {
  preset: 'ts-jest',
  testEnvironment: 'jsdom',
  roots: ['<rootDir>/src', '<rootDir>/tests'],
  testMatch: [
    '**/__tests__/**/*.browser.ts',
    '**/?(*.)+(browser.spec|browser.test).ts'
  ],
  transform: {
    '^.+\.ts$': 'ts-loader'
  },
  moduleNameMapping: {
    '^@/(.*)$': '<rootDir>/src/$1'
  },
  setupFilesAfterEnv: ['<rootDir>/tests/browser.setup.ts'],
  collectCoverageFrom: [
    'src/**/*.ts',
    '!src/**/*.d.ts',
    '!src/**/*.test.ts',
    '!src/**/*.spec.ts'
  ],
  coverageDirectory: 'coverage-browser',
  coverageReporters: ['text', 'lcov', 'html'],
  verbose: true,
  testEnvironmentOptions: {
    url: 'https://localhost'
  },
  globals: {
    'ts-jest': {
      useESM: true
    }
  },
  // Mock WebAssembly for tests
  setupFiles: ['<rootDir>/tests/wasm.mock.ts']
};