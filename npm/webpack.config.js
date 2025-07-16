const path = require('path');
const WasmPackPlugin = require('@wasm-tool/wasm-pack-plugin');

module.exports = (env, argv) => {
  const isProduction = argv.mode === 'production';
  
  return {
    mode: isProduction ? 'production' : 'development',
    entry: {
      index: './src/index.ts',
      browser: './src/browser.ts',
      'wasm-runner': './src/wasm-runner.ts'
    },
    output: {
      path: path.resolve(__dirname, 'dist'),
      filename: '[name].js',
      library: {
        name: 'CodeMesh',
        type: 'umd'
      },
      globalObject: 'this',
      clean: true
    },
    resolve: {
      extensions: ['.ts', '.js', '.wasm'],
      fallback: {
        "fs": false,
        "path": false,
        "os": false,
        "crypto": false,
        "stream": false,
        "buffer": false
      }
    },
    module: {
      rules: [
        {
          test: /\.ts$/,
          use: 'ts-loader',
          exclude: /node_modules/
        },
        {
          test: /\.wasm$/,
          type: 'webassembly/async'
        }
      ]
    },
    plugins: [
      new WasmPackPlugin({
        crateDirectory: path.resolve(__dirname, '../crates/code-mesh-wasm'),
        outDir: path.resolve(__dirname, 'wasm/bundler'),
        args: '--log-level warn',
        extraArgs: '--target bundler'
      })
    ],
    experiments: {
      asyncWebAssembly: true
    },
    devtool: isProduction ? 'source-map' : 'eval-source-map',
    optimization: {
      minimize: isProduction,
      splitChunks: {
        chunks: 'all',
        cacheGroups: {
          wasm: {
            test: /\.wasm$/,
            name: 'wasm',
            chunks: 'all'
          }
        }
      }
    }
  };
};