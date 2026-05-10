const fs = require('fs');
const path = require('path');
const https = require('https');
const { execSync } = require('child_process');

const VERSION = '0.1.0';
const REPO = 'AlphaGlider25/TyCode';

function getTarget() {
  const platform = process.platform;
  const arch = process.arch;

  if (platform === 'linux' && arch === 'x64') return 'x86_64-unknown-linux-gnu';
  if (platform === 'darwin' && arch === 'x64') return 'x86_64-apple-darwin';
  if (platform === 'darwin' && arch === 'arm64') return 'aarch64-apple-darwin';
  if (platform === 'win32' && arch === 'x64') return 'x86_64-pc-windows-msvc';

  console.error(`Unsupported platform: ${platform} ${arch}`);
  process.exit(1);
}

function getBinaryName(target) {
  return target.includes('windows') ? 'tycode.exe' : 'tycode';
}

async function downloadBinary(target) {
  const binaryName = getBinaryName(target);
  const url = `https://github.com/${REPO}/releases/download/${VERSION}/tycode-${target}`;
  const binDir = path.join(__dirname, '..', 'bin');
  const binPath = path.join(binDir, binaryName);

  if (!fs.existsSync(binDir)) {
    fs.mkdirSync(binDir, { recursive: true });
  }

  return new Promise((resolve, reject) => {
    https.get(url, (res) => {
      if (res.statusCode !== 200) {
        reject(new Error(`Failed to download binary: ${res.statusCode}`));
        return;
      }

      const file = fs.createWriteStream(binPath);
      res.pipe(file);

      file.on('finish', () => {
        file.close();
        fs.chmodSync(binPath, 0o755);
        resolve();
      });

      file.on('error', reject);
    }).on('error', reject);
  });
}

async function main() {
  try {
    const target = getTarget();
    console.log(`Installing tycode ${VERSION} for ${target}...`);
    await downloadBinary(target);
    console.log('✓ Installation complete');
  } catch (err) {
    console.error('✗ Installation failed:', err.message);
    process.exit(1);
  }
}

main();
