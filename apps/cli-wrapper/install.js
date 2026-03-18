const https = require('https');
const fs = require('fs');
const path = require('path');
const os = require('os');
const { execSync } = require('child_process');

const VERSION = require('./package.json').version;
const REPO = 'svetozar-technologies/localcode';

function getPlatformKey() {
  const platform = os.platform();
  const arch = os.arch();
  return `${platform}-${arch}`;
}

function getBinaryName() {
  const ext = os.platform() === 'win32' ? '.exe' : '';
  return `localcode-${getPlatformKey()}${ext}`;
}

async function install() {
  const binaryName = getBinaryName();
  const binDir = path.join(__dirname, 'bin');
  const binaryPath = path.join(binDir, binaryName);

  // Skip if already exists
  if (fs.existsSync(binaryPath)) {
    console.log('LocalCode binary already installed.');
    return;
  }

  // Try cargo install as fallback
  console.log(`Attempting cargo install localcode-cli...`);
  try {
    execSync('cargo install localcode-cli', { stdio: 'inherit' });
    console.log('LocalCode installed via cargo.');
    return;
  } catch {
    console.log('cargo install failed. Please install Rust or download the binary manually.');
    console.log(`Visit: https://github.com/${REPO}/releases`);
  }
}

install().catch(err => {
  console.error('Installation failed:', err.message);
  process.exit(1);
});
