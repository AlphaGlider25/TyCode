const fs = require('fs');
const path = require('path');

try {
  const binDir = path.join(__dirname, '..', 'bin');
  if (fs.existsSync(binDir)) {
    fs.rmSync(binDir, { recursive: true, force: true });
    console.log('✓ Binary removed');
  }
} catch (err) {
  console.error('✗ Uninstall failed:', err.message);
  process.exit(1);
}
