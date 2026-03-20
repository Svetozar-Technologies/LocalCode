const https = require("https");
const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");

const VERSION = require("./package.json").version;
const REPO = "Svetozar-Technologies/LocalCode";

const PLATFORM_MAP = {
  darwin: "darwin",
  linux: "linux",
};

const ARCH_MAP = {
  arm64: "arm64",
  x64: "x64",
};

function getTarballName() {
  const platform = PLATFORM_MAP[process.platform];
  const arch = ARCH_MAP[process.arch];

  if (!platform || !arch) {
    throw new Error(
      `Unsupported platform: ${process.platform}-${process.arch}`
    );
  }

  return `localcode-${platform}-${arch}.tar.gz`;
}

function download(url) {
  return new Promise((resolve, reject) => {
    https
      .get(url, (res) => {
        if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
          return download(res.headers.location).then(resolve, reject);
        }
        if (res.statusCode !== 200) {
          return reject(new Error(`HTTP ${res.statusCode} for ${url}`));
        }
        const chunks = [];
        res.on("data", (chunk) => chunks.push(chunk));
        res.on("end", () => resolve(Buffer.concat(chunks)));
        res.on("error", reject);
      })
      .on("error", reject);
  });
}

async function install() {
  const binDir = path.join(__dirname, "bin");
  const binaryPath = path.join(binDir, "localcode-binary");

  if (fs.existsSync(binaryPath)) {
    console.log("LocalCode binary already installed.");
    return;
  }

  const tarball = getTarballName();
  const url = `https://github.com/${REPO}/releases/download/v${VERSION}/${tarball}`;

  console.log(`Downloading LocalCode v${VERSION} for ${process.platform}-${process.arch}...`);

  try {
    const data = await download(url);

    // Extract to a temp directory to avoid overwriting the shell wrapper
    const tmpDir = path.join(__dirname, ".tmp-install");
    fs.mkdirSync(tmpDir, { recursive: true });

    const tarPath = path.join(tmpDir, tarball);
    fs.writeFileSync(tarPath, data);
    execSync(`tar xzf "${tarPath}" -C "${tmpDir}"`, { stdio: "inherit" });

    // The tarball contains either "localcode" (new format) or "localcode-platform-arch" (old format)
    const legacyName = tarball.replace(".tar.gz", "");
    const newFormatPath = path.join(tmpDir, "localcode");
    const legacyPath = path.join(tmpDir, legacyName);

    const extractedBinary = fs.existsSync(legacyPath) ? legacyPath : newFormatPath;

    fs.mkdirSync(binDir, { recursive: true });
    fs.copyFileSync(extractedBinary, binaryPath);
    fs.chmodSync(binaryPath, 0o755);

    // Clean up temp directory
    fs.rmSync(tmpDir, { recursive: true });

    console.log("LocalCode installed successfully.");
  } catch (err) {
    console.error(`Failed to download from GitHub: ${err.message}`);
    console.log(`Visit: https://github.com/${REPO}/releases`);
    process.exit(1);
  }
}

install().catch((err) => {
  console.error("Installation failed:", err.message);
  process.exit(1);
});
