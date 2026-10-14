#!/usr/bin/env node

/**
 * NioXS Companion Gateway Runner (bundled with NioAI)
 * Automatically detects platform (Linux, macOS, Windows, Termux),
 * locates existing binary or downloads native standalone nioxs binary,
 * and launches it directly.
 */

const { spawn, spawnSync } = require('child_process');
const fs = require('fs');
const path = require('path');
const os = require('os');
const https = require('https');

const REPO = 'nio-labs/nio-xs';
const DEFAULT_VERSION = 'v0.4.3';
const VERSION = process.env.NIOXS_VERSION || DEFAULT_VERSION;

function getPlatformInfo() {
  const nodePlatform = process.platform;
  const nodeArch = process.arch;

  let osType = '';
  let isTermux = false;
  let isWindows = false;

  if (process.env.TERMUX_VERSION || fs.existsSync('/data/data/com.termux')) {
    isTermux = true;
    osType = 'termux';
  } else if (nodePlatform === 'win32') {
    isWindows = true;
    osType = 'windows';
  } else if (nodePlatform === 'darwin') {
    osType = 'darwin';
  } else {
    osType = 'linux';
  }

  let arch = '';
  if (nodeArch === 'x64') {
    arch = 'amd64';
  } else if (nodeArch === 'arm64') {
    arch = 'arm64';
  } else {
    arch = nodeArch;
  }

  return { osType, arch, isWindows, isTermux };
}

function downloadBinary(url, dest) {
  return new Promise((resolve, reject) => {
    https.get(url, (res) => {
      if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
        return downloadBinary(res.headers.location, dest).then(resolve).catch(reject);
      }
      if (res.statusCode !== 200) {
        return reject(new Error(`Failed to download binary from ${url} (HTTP ${res.statusCode})`));
      }
      const fileStream = fs.createWriteStream(dest);
      res.pipe(fileStream);
      fileStream.on('finish', () => {
        fileStream.close();
        resolve();
      });
      fileStream.on('error', reject);
    }).on('error', reject);
  });
}

async function findOrDownloadBinary() {
  const { osType, arch, isWindows, isTermux } = getPlatformInfo();
  const binaryName = isWindows ? 'nioxs.exe' : 'nioxs';

  // Check 1: Explicit env override
  if (process.env.NIOXS_BIN && fs.existsSync(process.env.NIOXS_BIN)) {
    return process.env.NIOXS_BIN;
  }

  // Check 2: Adjacent to current nio binary or local target/release
  const adjacentPaths = [
    path.resolve(__dirname, '..', '..', 'nio-xs', 'target', 'release', binaryName),
    path.resolve(__dirname, '..', 'target', 'release', binaryName),
  ];
  for (const p of adjacentPaths) {
    if (fs.existsSync(p)) return p;
  }

  // Check 3: Global PATH
  const whichCmd = isWindows ? 'where' : 'which';
  const whichResult = spawnSync(whichCmd, ['nioxs'], { stdio: 'pipe' });
  if (whichResult.status === 0) {
    const foundPath = whichResult.stdout.toString().split(/\r?\n/)[0].trim();
    if (foundPath && fs.existsSync(foundPath)) {
      return foundPath;
    }
  }

  // Check 4: Termux $PREFIX/bin/nioxs
  if (isTermux) {
    const prefixBin = path.join(process.env.PREFIX || '/data/data/com.termux/files/usr', 'bin', binaryName);
    if (fs.existsSync(prefixBin)) {
      return prefixBin;
    }
  }

  // Check 5: User cache directory ~/.nioxs/bin/nioxs
  const userHome = os.homedir();
  const cacheDir = path.join(userHome, '.nioxs', 'bin');
  if (!fs.existsSync(cacheDir)) {
    fs.mkdirSync(cacheDir, { recursive: true });
  }

  const cachedBinary = path.join(cacheDir, binaryName);
  if (fs.existsSync(cachedBinary)) {
    return cachedBinary;
  }

  // Download binary from GitHub Releases
  console.error(`[nio-xs] Initializing NioXS companion gateway binary for ${osType}-${arch}...`);
  const assetName = isWindows
    ? `nioxs-${osType}-${arch}.exe`
    : `nioxs-${osType}-${arch}`;

  const downloadUrl = `https://github.com/${REPO}/releases/download/${VERSION}/${assetName}`;

  try {
    console.error(`[nio-xs] Fetching from ${downloadUrl}...`);
    await downloadBinary(downloadUrl, cachedBinary);
    if (!isWindows) {
      fs.chmodSync(cachedBinary, 0o755);
    }
    console.error(`[nio-xs] Successfully installed to ${cachedBinary}`);
    await ensureNioDbBinary(cacheDir, { osType, arch, isWindows });
    return cachedBinary;
  } catch (err) {
    console.error(`[nio-xs] Remote binary download failed: ${err.message}`);
    const cargoCheck = spawnSync('cargo', ['--version'], { stdio: 'pipe' });
    if (cargoCheck.status === 0) {
      console.error(`[nio-xs] Compiling locally with cargo install --locked --git https://github.com/${REPO}.git ...`);
      const installResult = spawnSync('cargo', ['install', '--locked', '--git', `https://github.com/${REPO}.git`, '--bin', 'nioxs', '--root', path.join(userHome, '.nioxs')], {
        stdio: 'inherit',
      });
      if (installResult.status === 0 && fs.existsSync(cachedBinary)) {
        await ensureNioDbBinary(cacheDir, { osType, arch, isWindows });
        return cachedBinary;
      }
    }
    throw new Error(`Could not find or download nioxs binary. Run 'cargo install --git https://github.com/${REPO}.git' or visit https://github.com/${REPO}/releases.`);
  }
}

async function ensureNioDbBinary(cacheDir, { osType, arch, isWindows }) {
  const niodbName = isWindows ? 'niodb.exe' : 'niodb';
  const targetPath = path.join(cacheDir, niodbName);
  if (fs.existsSync(targetPath)) {
    return targetPath;
  }

  const whichCmd = isWindows ? 'where' : 'which';
  const whichResult = spawnSync(whichCmd, ['niodb'], { stdio: 'pipe' });
  if (whichResult.status === 0) {
    return targetPath;
  }

  console.error(`[nio-xs] Bundling NioDB for companion memory persistence...`);
  const dbOs = osType === 'windows' ? 'win32' : osType;
  const dbArch = arch === 'amd64' ? 'x64' : arch;
  const dbAssetName = isWindows ? `niodb-${dbOs}-${dbArch}.exe` : `niodb-${dbOs}-${dbArch}`;
  const niodbUrl = `https://github.com/nio-labs/nio-db/releases/latest/download/${dbAssetName}`;

  try {
    await downloadBinary(niodbUrl, targetPath);
    if (!isWindows) {
      fs.chmodSync(targetPath, 0o755);
    }
    console.error(`[nio-xs] NioDB bundled successfully at ${targetPath}`);
  } catch (err) {
    console.error(`[nio-xs] Note: Automatic NioDB download deferred (${err.message}).`);
  }
  return targetPath;
}

async function main() {
  try {
    // If installed in node_modules, try delegating to @nio-labs/nio-xs if available
    try {
      const pkgPath = require.resolve('@nio-labs/nio-xs/bin/nioxs.js');
      if (pkgPath && pkgPath !== __filename && fs.existsSync(pkgPath)) {
        require(pkgPath);
        return;
      }
    } catch {}

    const binaryPath = await findOrDownloadBinary();
    const args = process.argv.slice(2);

    const child = spawn(binaryPath, args, {
      stdio: 'inherit',
      cwd: process.cwd(),
      env: process.env,
    });

    const forwardSignal = (sig) => {
      if (child.pid) {
        try { process.kill(child.pid, sig); } catch {}
      }
    };

    process.on('SIGINT', () => forwardSignal('SIGINT'));
    process.on('SIGTERM', () => forwardSignal('SIGTERM'));
    process.on('SIGHUP', () => forwardSignal('SIGHUP'));

    child.on('exit', (code, signal) => {
      process.exit(code ?? (signal ? 128 + (os.constants.signals[signal] || 1) : 1));
    });

    child.on('error', (err) => {
      console.error(`[nio-xs] Execution error: ${err.message}`);
      process.exit(1);
    });
  } catch (err) {
    console.error(`[nio-xs error] ${err.message}`);
    process.exit(1);
  }
}

if (require.main === module) main();
module.exports = { findOrDownloadBinary, getPlatformInfo, downloadBinary, ensureNioDbBinary };
