#!/usr/bin/env node

const { execFileSync } = require('child_process');
const path = require('path');
const os = require('os');
const fs = require('fs');

function getBinaryName() {
  const platform = os.platform();
  const arch = os.arch();
  const ext = platform === 'win32' ? '.exe' : '';
  return `localcode-${platform}-${arch}${ext}`;
}

function findBinary() {
  // Check local install
  const localBin = path.join(__dirname, '..', 'bin', getBinaryName());
  if (fs.existsSync(localBin)) return localBin;

  // Check global paths
  const globalBin = path.join(__dirname, getBinaryName());
  if (fs.existsSync(globalBin)) return globalBin;

  // Fallback: try PATH
  return 'localcode';
}

try {
  const binary = findBinary();
  const result = execFileSync(binary, process.argv.slice(2), {
    stdio: 'inherit',
    env: process.env,
  });
} catch (error) {
  if (error.status !== undefined) {
    process.exit(error.status);
  }
  console.error('Failed to run localcode binary. Try reinstalling: npm install -g localcode');
  process.exit(1);
}
