/**
 * WASM mocks for testing
 */

// Mock WASM module
const mockWasmModule = {
  CodeMesh: jest.fn().mockImplementation(() => ({
    session_id: jest.fn().mockReturnValue('test-session-123'),
    add_user_message: jest.fn().mockResolvedValue(undefined),
    generate_response: jest.fn().mockResolvedValue('Test response'),
    get_messages: jest.fn().mockReturnValue([]),
    clear_session: jest.fn(),
    save_session: jest.fn().mockResolvedValue(undefined),
    load_session: jest.fn().mockResolvedValue(undefined),
    list_sessions: jest.fn().mockResolvedValue([]),
    delete_session: jest.fn().mockResolvedValue(undefined),
    get_performance_metrics: jest.fn().mockReturnValue({}),
    get_memory_usage: jest.fn().mockReturnValue({}),
    get_clipboard: jest.fn().mockResolvedValue(''),
    set_clipboard: jest.fn().mockResolvedValue(undefined),
    initialize: jest.fn().mockResolvedValue(undefined)
  })),
  
  CodeMeshConfig: jest.fn().mockImplementation(() => ({
    use_browser_storage: true,
    enable_offline: false,
    use_web_workers: true,
    max_memory_mb: 512,
    enable_performance_monitoring: true,
    api_endpoint: null,
    auth_provider: null
  })),
  
  init: jest.fn(),
  list_providers: jest.fn().mockResolvedValue(['anthropic', 'openai']),
  list_models: jest.fn().mockResolvedValue(['claude-3-sonnet', 'gpt-4']),
  get_platform_info: jest.fn().mockReturnValue({
    is_browser: true,
    is_node: false,
    user_agent: 'test-agent',
    platform: 'test-platform'
  }),
  check_feature_support: jest.fn().mockReturnValue({
    simd: false,
    threads: false,
    bulk_memory: true,
    reference_types: true
  }),
  create_worker: jest.fn().mockReturnValue({
    post_message: jest.fn(),
    terminate: jest.fn()
  })
};

// Mock dynamic imports
jest.mock('../wasm/web/code_mesh_wasm.js', () => ({
  __esModule: true,
  default: jest.fn().mockResolvedValue(undefined),
  ...mockWasmModule
}), { virtual: true });

jest.mock('../wasm/node/code_mesh_wasm.js', () => ({
  __esModule: true,
  ...mockWasmModule
}), { virtual: true });

jest.mock('../wasm/bundler/code_mesh_wasm.js', () => ({
  __esModule: true,
  default: jest.fn().mockResolvedValue(undefined),
  ...mockWasmModule
}), { virtual: true });

export { mockWasmModule };