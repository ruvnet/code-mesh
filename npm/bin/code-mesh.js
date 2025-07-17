#!/usr/bin/env node

const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');
const os = require('os');
const https = require('https');
const crypto = require('crypto');
const chalk = require('chalk').default || require('chalk');
const ora = require('ora').default || require('ora');

// Binary names for different platforms
const BINARY_NAME = {
  'darwin-x64': 'code-mesh-macos-x64',
  'darwin-arm64': 'code-mesh-macos-arm64',
  'linux-x64': 'code-mesh-linux-x64',
  'linux-arm64': 'code-mesh-linux-arm64',
  'win32-x64': 'code-mesh-windows-x64.exe',
};

// Get platform-specific binary name
function getBinaryName() {
  const platform = os.platform();
  const arch = os.arch();
  const key = `${platform}-${arch}`;
  return BINARY_NAME[key];
}

// Get binary path
function getBinaryPath() {
  const binaryName = getBinaryName();
  if (!binaryName) {
    console.error(chalk.red(`Unsupported platform: ${os.platform()} ${os.arch()}`));
    console.error(chalk.yellow('Code Mesh native binary is not available for your platform.'));
    console.error(chalk.yellow('Falling back to WASM version...'));
    return null;
  }
  
  return path.join(__dirname, '..', 'bin', binaryName);
}

// Download binary if not exists
async function ensureBinary() {
  const binaryPath = getBinaryPath();
  if (!binaryPath) return false;
  
  if (fs.existsSync(binaryPath)) {
    return true;
  }
  
  const spinner = ora('Downloading Code Mesh binary...').start();
  
  try {
    // TODO: Implement actual binary download from GitHub releases
    spinner.fail('Binary download not yet implemented');
    return false;
  } catch (error) {
    spinner.fail(`Failed to download binary: ${error.message}`);
    return false;
  }
}

// Run native binary
function runNativeBinary() {
  const binaryPath = getBinaryPath();
  const args = process.argv.slice(2);
  
  const child = spawn(binaryPath, args, {
    stdio: 'inherit',
    env: {
      ...process.env,
      RUST_LOG: 'error', // Suppress warnings, only show errors
    },
  });
  
  child.on('exit', (code) => {
    process.exit(code);
  });
  
  child.on('error', (err) => {
    console.error(chalk.red(`Failed to run Code Mesh: ${err.message}`));
    process.exit(1);
  });
}

// Run WASM version
async function runWasmVersion() {
  console.log(chalk.yellow('Running Code Mesh in WASM mode...'));
  
  try {
    // Dynamic import for ESM support
    const { runWasm } = await import('../dist/wasm-runner.js');
    await runWasm(process.argv.slice(2));
  } catch (error) {
    console.error(chalk.red(`Failed to run WASM version: ${error.message}`));
    process.exit(1);
  }
}

// Main execution
async function main() {
  const binaryAvailable = await ensureBinary();
  
  if (binaryAvailable) {
    runNativeBinary();
  } else {
    await runWasmVersion();
  }
}

// Run
main().catch((err) => {
  console.error(chalk.red(`Unexpected error: ${err.message}`));
  process.exit(1);
});