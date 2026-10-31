#!/usr/bin/env node

/**
 * Valheim Modpack Generator for Thunderstore / r2modman
 * 
 * Extracts installed mods and configs from any r2modman Valheim profile,
 * provides granular selection of mods, versions, and configs, and packages
 * everything into a Thunderstore-ready ZIP archive.
 */

const fs = require('fs');
const path = require('path');
const os = require('os');
const zlib = require('zlib');
const readline = require('readline');
const { execFileSync } = require('child_process');

// ANSI color codes
const c = {
  reset: '\x1b[0m',
  bold: '\x1b[1m',
  dim: '\x1b[2m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  blue: '\x1b[34m',
  cyan: '\x1b[36m',
  red: '\x1b[31m',
  magenta: '\x1b[35m',
};

// Default search paths for r2modman Valheim profiles
function getDefaultProfilesDirs() {
  const home = os.homedir();
  return [
    path.join(home, '.config', 'r2modmanPlus-local', 'Valheim', 'profiles'),
    path.join(home, '.config', 'r2modman', 'Valheim', 'profiles'),
    path.join(home, '.var', 'app', 'com.github.ebkr.r2modman', 'config', 'r2modmanPlus-local', 'Valheim', 'profiles'),
    path.join(home, 'AppData', 'Roaming', 'r2modmanPlus-local', 'Valheim', 'profiles'),
  ].filter(p => fs.existsSync(p));
}

// Find profiles in a profiles directory
function getAvailableProfiles(baseDir) {
  if (!fs.existsSync(baseDir)) return [];
  const entries = fs.readdirSync(baseDir, { withFileTypes: true });
  return entries
    .filter(e => e.isDirectory() && fs.existsSync(path.join(baseDir, e.name, 'mods.yml')))
    .map(e => e.name);
}

// Parse mods.yml from an r2modman profile
function parseModsYml(profilePath) {
  const modsFile = path.join(profilePath, 'mods.yml');
  if (!fs.existsSync(modsFile)) {
    throw new Error(`mods.yml not found at: ${modsFile}`);
  }

  const content = fs.readFileSync(modsFile, 'utf8');
  const blocks = content.split(/(?:^|\n)- manifestVersion:\s*\d+/);
  const mods = [];

  for (const block of blocks.slice(1)) {
    const getVal = (key) => {
      const match = block.match(new RegExp(`(?:^|\\n)\\s*${key}:\\s*([^\\n]+)`));
      return match ? match[1].trim().replace(/^['"]|['"]$/g, '') : '';
    };

    const name = getVal('name');
    const authorName = getVal('authorName');
    const displayName = getVal('displayName') || name;
    const description = getVal('description');
    const websiteUrl = getVal('websiteUrl');
    const enabled = getVal('enabled') === 'true';

    const major = getVal('major') || '0';
    const minor = getVal('minor') || '0';
    const patch = getVal('patch') || '0';
    const version = `${major}.${minor}.${patch}`;

    // Thunderstore dependency string is Author-ModName-Major.Minor.Patch
    const depString = `${name}-${version}`;

    if (name) {
      mods.push({
        name,
        authorName,
        displayName,
        description,
        websiteUrl,
        enabled,
        version,
        depString,
      });
    }
  }

  return mods;
}

// Check if a path is purely runtime cache or report
function isCacheOrReport(name, fullPath) {
  if (/MultiplayerClientCache|Reports|ClusterCache|\.records$|\.log$/i.test(name)) return true;
  if (fs.existsSync(fullPath) && fs.statSync(fullPath).isDirectory()) {
    try {
      const children = fs.readdirSync(fullPath);
      if (children.length === 0) return false;
      // If all subchildren are cache or reports
      return children.every(c => /MultiplayerClientCache|Reports|ClusterCache|\.records$|\.log$/i.test(c));
    } catch {
      return false;
    }
  }
  return false;
}

// Scan configs in BepInEx/config
function scanProfileConfigs(profilePath) {
  const configDir = path.join(profilePath, 'BepInEx', 'config');
  if (!fs.existsSync(configDir)) return [];

  const items = fs.readdirSync(configDir, { withFileTypes: true });
  const configs = [];

  for (const item of items) {
    const fullPath = path.join(configDir, item.name);
    const isDir = item.isDirectory();
    const isDat = item.name.endsWith('.dat');
    const isSensitive = isDat || /quickconnect|player_\d+|credential|secret|token/i.test(item.name);
    const isCache = isCacheOrReport(item.name, fullPath);

    let category = 'SAFE';
    if (isSensitive) category = 'SENSITIVE';
    else if (isCache) category = 'CACHE';

    configs.push({
      name: item.name,
      fullPath,
      isDir,
      isSensitive,
      isCache,
      category,
      size: isDir ? 0 : fs.statSync(fullPath).size,
    });
  }

  return configs;
}

// Generate valid 256x256 PNG in memory with pure Node.js
function generateDefaultIcon(title = 'Valheim') {
  function crc32(buf) {
    let crc = 0xffffffff;
    for (let i = 0; i < buf.length; i++) {
      crc ^= buf[i];
      for (let j = 0; j < 8; j++) {
        crc = (crc >>> 1) ^ (-(crc & 1) & 0xedb88320);
      }
    }
    return (crc ^ 0xffffffff) >>> 0;
  }

  function makeChunk(type, data) {
    const len = Buffer.alloc(4);
    len.writeUInt32BE(data.length, 0);
    const typeBuf = Buffer.from(type, 'ascii');
    const crcBuf = Buffer.alloc(4);
    crcBuf.writeUInt32BE(crc32(Buffer.concat([typeBuf, data])), 0);
    return Buffer.concat([len, typeBuf, data, crcBuf]);
  }

  const width = 256, height = 256;
  const header = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8; // 8 bit depth
  ihdr[9] = 6; // RGBA
  ihdr[10] = 0; ihdr[11] = 0; ihdr[12] = 0;

  const raw = Buffer.alloc(height * (1 + width * 4));
  let offset = 0;
  for (let y = 0; y < height; y++) {
    raw[offset++] = 0; // PNG filter byte (None)
    for (let x = 0; x < width; x++) {
      // Norse gold border (width 6px) + corner ornaments
      const isBorder = (x < 6 || x >= width - 6 || y < 6 || y >= height - 6);
      const isInnerBorder = (x === 12 || x === width - 13 || y === 12 || y === height - 13) && (x >= 12 && x <= width - 13 && y >= 12 && y <= height - 13);
      
      // Central diamond rune emblem
      const dx = Math.abs(x - 128);
      const dy = Math.abs(y - 128);
      const isRune = Math.abs(dx + dy - 60) <= 3 || (dx <= 3 && dy <= 70) || (Math.abs(dx - dy) <= 3 && dx < 40 && dy < 40);

      if (isBorder || isInnerBorder || isRune) {
        raw[offset++] = 212; // R: Norse bronze/gold
        raw[offset++] = 162; // G
        raw[offset++] = 58;  // B
        raw[offset++] = 255; // A
      } else {
        // Deep charcoal/slate gradient
        const grad = Math.floor(22 + (y / height) * 28 + (Math.sin(x / 20) * 4));
        raw[offset++] = Math.max(0, Math.min(255, grad - 6));
        raw[offset++] = Math.max(0, Math.min(255, grad));
        raw[offset++] = Math.max(0, Math.min(255, grad + 8));
        raw[offset++] = 255;
      }
    }
  }

  const idat = zlib.deflateSync(raw);
  return Buffer.concat([
    header,
    makeChunk('IHDR', ihdr),
    makeChunk('IDAT', idat),
    makeChunk('IEND', Buffer.alloc(0))
  ]);
}

// Copy file or directory recursively with optional filter
function copyRecursive(src, dest, filterFn = null) {
  if (filterFn && !filterFn(src)) return;
  const stat = fs.lstatSync(src);
  if (stat.isSymbolicLink()) {
    throw new Error(`Refusing to package symlink: ${src}`);
  }
  if (stat.isDirectory()) {
    fs.mkdirSync(dest, { recursive: true });
    for (const item of fs.readdirSync(src)) {
      copyRecursive(path.join(src, item), path.join(dest, item), filterFn);
    }
  } else {
    fs.mkdirSync(path.dirname(dest), { recursive: true });
    fs.copyFileSync(src, dest);
  }
}

// Create ZIP file using nix `zip` or fallback to node zip
function createZipArchive(sourceDir, zipFilePath) {
  const finalPath = path.resolve(zipFilePath);
  fs.mkdirSync(path.dirname(finalPath), { recursive: true });
  const tempDir = fs.mkdtempSync(path.join(path.dirname(finalPath), '.valheim-pack-'));
  const tempZip = path.join(tempDir, path.basename(finalPath));
  let preserveTempDir = false;
  try {
    try {
      execFileSync('zip', ['-r', '-q', tempZip, '.'], {
        cwd: sourceDir,
        stdio: 'pipe',
      });
    } catch (err) {
      // Keep the dependency-free Node writer as a fallback when `zip` is unavailable.
      writeStandaloneZip(sourceDir, tempZip);
    }
    if (!fs.existsSync(tempZip) || fs.statSync(tempZip).size === 0) {
      throw new Error('ZIP writer did not produce an archive');
    }

    let existing;
    try {
      existing = fs.lstatSync(finalPath);
    } catch (err) {
      if (err.code !== 'ENOENT') throw err;
    }
    if (existing && (!existing.isFile() || existing.isSymbolicLink())) {
      throw new Error(`Refusing to replace non-regular archive path: ${finalPath}`);
    }

    if (existing && process.platform === 'win32') {
      const backupPath = path.join(tempDir, `${path.basename(finalPath)}.backup`);
      fs.renameSync(finalPath, backupPath);
      try {
        fs.renameSync(tempZip, finalPath);
      } catch (err) {
        try {
          fs.renameSync(backupPath, finalPath);
        } catch (restoreErr) {
          preserveTempDir = true;
          throw new Error(`Could not publish archive (${err.message}) or restore previous archive (${restoreErr.message}); previous archive is preserved at ${backupPath}`);
        }
        throw err;
      }
      fs.rmSync(backupPath);
    } else {
      fs.renameSync(tempZip, finalPath);
    }
    return true;
  } finally {
    if (!preserveTempDir) fs.rmSync(tempDir, { recursive: true, force: true });
  }
}

// Minimal pure Node.js ZIP archive generator (deflate)
function writeStandaloneZip(sourceDir, outZipPath) {
  function crc32(buf) {
    let crc = 0xffffffff;
    for (let i = 0; i < buf.length; i++) {
      crc ^= buf[i];
      for (let j = 0; j < 8; j++) {
        crc = (crc >>> 1) ^ (-(crc & 1) & 0xedb88320);
      }
    }
    return (crc ^ 0xffffffff) >>> 0;
  }

  function getFiles(dir, base = '') {
    const results = [];
    for (const item of fs.readdirSync(dir, { withFileTypes: true })) {
      const rel = base ? `${base}/${item.name}` : item.name;
      const full = path.join(dir, item.name);
      if (item.isDirectory()) {
        results.push(...getFiles(full, rel));
      } else {
        results.push({ relPath: rel, fullPath: full });
      }
    }
    return results;
  }

  const files = getFiles(sourceDir);
  const localHeaders = [];
  const centralHeaders = [];
  let offset = 0;

  for (const f of files) {
    const rawData = fs.readFileSync(f.fullPath);
    const compressedData = zlib.deflateRawSync(rawData);
    const fileCrc = crc32(rawData);
    const nameBuf = Buffer.from(f.relPath.replace(/\\/g, '/'), 'utf8');

    // Local file header (30 bytes + nameBuf.length)
    const lh = Buffer.alloc(30);
    lh.writeUInt32LE(0x04034b50, 0); // Signature
    lh.writeUInt16LE(20, 4);         // Version needed (2.0)
    lh.writeUInt16LE(0, 6);          // Flags
    lh.writeUInt16LE(8, 8);          // Compression (Deflate)
    lh.writeUInt16LE(0, 10);         // Mod time
    lh.writeUInt16LE(0, 12);         // Mod date
    lh.writeUInt32LE(fileCrc, 14);   // CRC32
    lh.writeUInt32LE(compressedData.length, 18); // Compressed size
    lh.writeUInt32LE(rawData.length, 22);        // Uncompressed size
    lh.writeUInt16LE(nameBuf.length, 26);        // Filename len
    lh.writeUInt16LE(0, 28);                     // Extra len

    const localRecord = Buffer.concat([lh, nameBuf, compressedData]);
    localHeaders.push(localRecord);

    // Central directory header (46 bytes + nameBuf.length)
    const cd = Buffer.alloc(46);
    cd.writeUInt32LE(0x02014b50, 0); // Signature
    cd.writeUInt16LE(20, 4);         // Version made by
    cd.writeUInt16LE(20, 6);         // Version needed
    cd.writeUInt16LE(0, 8);          // Flags
    cd.writeUInt16LE(8, 10);         // Compression (Deflate)
    cd.writeUInt16LE(0, 12);         // Mod time
    cd.writeUInt16LE(0, 14);         // Mod date
    cd.writeUInt32LE(fileCrc, 16);   // CRC32
    cd.writeUInt32LE(compressedData.length, 20); // Compressed size
    cd.writeUInt32LE(rawData.length, 24);        // Uncompressed size
    cd.writeUInt16LE(nameBuf.length, 28);        // Filename len
    cd.writeUInt16LE(0, 30);                     // Extra len
    cd.writeUInt16LE(0, 32);                     // Comment len
    cd.writeUInt16LE(0, 34);                     // Disk num
    cd.writeUInt16LE(0, 36);                     // Internal attrs
    cd.writeUInt32LE(0, 38);                     // External attrs
    cd.writeUInt32LE(offset, 42);                // Local header offset

    centralHeaders.push(Buffer.concat([cd, nameBuf]));
    offset += localRecord.length;
  }

  const cdBuffer = Buffer.concat(centralHeaders);
  const cdOffset = offset;
  const cdSize = cdBuffer.length;

  // End of central directory record (22 bytes)
  const eocd = Buffer.alloc(22);
  eocd.writeUInt32LE(0x06054b50, 0); // Signature
  eocd.writeUInt16LE(0, 4);          // Disk number
  eocd.writeUInt16LE(0, 6);          // Start disk
  eocd.writeUInt16LE(files.length, 8);  // Entries on disk
  eocd.writeUInt16LE(files.length, 10); // Total entries
  eocd.writeUInt32LE(cdSize, 12);       // Size of CD
  eocd.writeUInt32LE(cdOffset, 16);     // Offset of CD
  eocd.writeUInt16LE(0, 20);            // Comment length

  const finalZip = Buffer.concat([...localHeaders, cdBuffer, eocd]);
  fs.writeFileSync(outZipPath, finalZip);
}

// Interactive prompt helper
function createPrompter() {
  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
  });

  const question = (query) => new Promise(resolve => rl.question(query, resolve));
  const close = () => rl.close();

  return { question, close };
}

// Format markdown README
function generateReadme(name, version, description, mods, includedConfigs) {
  return `# ${name}

> ${description}

**Version:** \`${version}\`

---

## 📦 Included Mods (${mods.length})

| Mod | Author | Version | Package |
| --- | --- | --- | --- |
${mods.map(m => `| **${m.displayName}** | ${m.authorName || '-'} | \`${m.version}\` | [\`${m.name}\`](${m.websiteUrl || `https://thunderstore.io/c/valheim/p/${m.name}/`}) |`).join('\n')}

---

## ⚙️ Included Configurations (${includedConfigs.length})

${includedConfigs.map(c => `- \`${c.name}\``).join('\n')}
`;
}

// Build Modpack
function buildModpack({
  profilePath,
  name,
  version,
  description,
  websiteUrl,
  selectedMods,
  selectedConfigs,
  customIconPath,
  outDir = './build',
  distDir = './dist',
}) {
  if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-(?:(?:0|[1-9]\d*|[0-9A-Za-z-]*[A-Za-z-][0-9A-Za-z-]*)(?:\.(?:0|[1-9]\d*|[0-9A-Za-z-]*[A-Za-z-][0-9A-Za-z-]*))*))?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$/.test(version)) {
    throw new Error(`Invalid modpack version '${version}'; use semantic versioning such as 1.2.3`);
  }
  if (!/^[A-Za-z0-9_]+$/.test(name)) {
    throw new Error('Invalid modpack name; use letters, numbers, and underscores only');
  }
  const buildFolder = path.join(outDir, name);
  const zipFile = path.join(distDir, `${name}-${version}.zip`);

  // Clean build folder
  if (fs.existsSync(buildFolder)) {
    fs.rmSync(buildFolder, { recursive: true, force: true });
  }
  fs.mkdirSync(buildFolder, { recursive: true });

  // 1. Generate manifest.json
  const manifest = {
    name,
    version_number: version,
    website_url: websiteUrl || '',
    description: description || 'Curated Valheim Modpack',
    dependencies: selectedMods.map(m => `${m.name}-${m.version}`),
  };
  fs.writeFileSync(
    path.join(buildFolder, 'manifest.json'),
    JSON.stringify(manifest, null, 2)
  );

  // 2. Generate README.md
  const readmeContent = generateReadme(name, version, description, selectedMods, selectedConfigs);
  fs.writeFileSync(path.join(buildFolder, 'README.md'), readmeContent);

  // 3. Handle icon.png
  const destIcon = path.join(buildFolder, 'icon.png');
  if (customIconPath && fs.existsSync(customIconPath)) {
    fs.copyFileSync(customIconPath, destIcon);
  } else {
    fs.writeFileSync(destIcon, generateDefaultIcon(name));
  }

  // 4. Copy config files to build/config
  const configDest = path.join(buildFolder, 'config');
  fs.mkdirSync(configDest, { recursive: true });
  for (const cfg of selectedConfigs) {
    const targetPath = path.join(configDest, cfg.name);
    copyRecursive(cfg.fullPath, targetPath, (p) => !/MultiplayerClientCache|Reports|ClusterCache|\.records$|\.log$/i.test(p));
  }

  // Clean up any empty folders inside build/config
  function cleanEmptyDirs(dir) {
    if (!fs.existsSync(dir)) return;
    for (const item of fs.readdirSync(dir, { withFileTypes: true })) {
      const full = path.join(dir, item.name);
      if (item.isDirectory()) {
        cleanEmptyDirs(full);
        if (fs.readdirSync(full).length === 0) {
          fs.rmdirSync(full);
        }
      }
    }
  }
  cleanEmptyDirs(configDest);

  // 5. Create ZIP package
  createZipArchive(buildFolder, zipFile);

  return {
    buildFolder,
    zipFile,
    manifest,
    modCount: selectedMods.length,
    configCount: selectedConfigs.length,
    zipSize: fs.statSync(zipFile).size,
  };
}

// Parse command line arguments
function parseArgs() {
  const args = process.argv.slice(2);
  const options = {
    help: false,
    interactive: true,
    listProfiles: false,
    listMods: false,
    listConfigs: false,
    profile: '',
    profilesDir: '',
    name: '',
    version: '1.0.0',
    desc: 'Valheim modpack with custom configurations',
    website: '',
    allMods: false,
    allConfigs: false,
    includeSensitive: false,
    includeCaches: false,
    icon: '',
    out: './build',
    dist: './dist',
  };

  for (let i = 0; i < args.length; i++) {
    const a = args[i];
    if (a === '--help' || a === '-h') options.help = true;
    else if (a === '--list-profiles') options.listProfiles = true;
    else if (a === '--list-mods') options.listMods = true;
    else if (a === '--list-configs') options.listConfigs = true;
    else if (a === '--profile' || a === '-p') options.profile = args[++i];
    else if (a === '--profiles-dir') options.profilesDir = args[++i];
    else if (a === '--name' || a === '-n') options.name = args[++i];
    else if (a === '--version' || a === '-v') options.version = args[++i];
    else if (a === '--desc' || a === '-d') options.desc = args[++i];
    else if (a === '--website' || a === '-w') options.website = args[++i];
    else if (a === '--all-mods') { options.allMods = true; options.interactive = false; }
    else if (a === '--all-configs') { options.allConfigs = true; options.interactive = false; }
    else if (a === '--include-sensitive') options.includeSensitive = true;
    else if (a === '--include-caches') options.includeCaches = true;
    else if (a === '--icon') options.icon = args[++i];
    else if (a === '--yes' || a === '-y' || a === '--non-interactive') options.interactive = false;
  }

  return options;
}

// Print Help
function printHelp() {
  console.log(`
${c.bold}${c.cyan}Valheim Modpack Generator for Thunderstore / r2modman${c.reset}

${c.bold}USAGE:${c.reset}
  node valheim-pack.js [options]

${c.bold}MODES:${c.reset}
  ${c.green}Interactive Mode (default):${c.reset}
    Run with no arguments to step through an interactive wizard for profile selection,
    granular mod toggling, version overrides, and config filtering.

  ${c.green}Automated / Scriptable Mode:${c.reset}
    Pass CLI flags to generate a modpack directly without prompts.

${c.bold}OPTIONS:${c.reset}
  ${c.yellow}-p, --profile <name>${c.reset}         Target r2modman profile (e.g. Default, Music, Test)
  ${c.yellow}--profiles-dir <dir>${c.reset}        Custom profiles directory path
  ${c.yellow}-n, --name <name>${c.reset}           Modpack package name (letters, numbers, underscores)
  ${c.yellow}-v, --version <semver>${c.reset}       Modpack version (e.g. 1.0.0)
  ${c.yellow}-d, --desc <description>${c.reset}    Modpack description (up to 250 chars)
  ${c.yellow}-w, --website <url>${c.reset}         Website / GitHub repository URL
  ${c.yellow}--icon <path>${c.reset}               Custom 256x256 PNG icon
  ${c.yellow}--all-mods${c.reset}                  Include all enabled mods without prompting
  ${c.yellow}--all-configs${c.reset}               Include all safe configs without prompting
  ${c.yellow}--include-sensitive${c.reset}         Include sensitive files (.dat, passwords) in configs
  ${c.yellow}--include-caches${c.reset}            Include world cache files and profiling reports
  ${c.yellow}-y, --yes${c.reset}                   Non-interactive, use defaults for unspecified options
  ${c.yellow}--list-profiles${c.reset}             List detected r2modman profiles
  ${c.yellow}--list-mods${c.reset}                 List mods in target profile
  ${c.yellow}--list-configs${c.reset}              List configs in target profile
  ${c.yellow}-h, --help${c.reset}                  Show this help message

${c.bold}EXAMPLES:${c.reset}
  ${c.dim}# Run interactive wizard:${c.reset}
  node valheim-pack.js

  ${c.dim}# Quick export Default profile:${c.reset}
  node valheim-pack.js --profile Default --name My_Viking_Pack --version 1.0.0 --all-mods --all-configs -y

  ${c.dim}# List available profiles:${c.reset}
  node valheim-pack.js --list-profiles
`);
}

// Main execution
async function main() {
  const opts = parseArgs();

  if (opts.help) {
    printHelp();
    return;
  }

  // Resolve profiles directory
  const availableDirs = getDefaultProfilesDirs();
  const profilesDir = opts.profilesDir || (availableDirs.length > 0 ? availableDirs[0] : null);

  if (!profilesDir || !fs.existsSync(profilesDir)) {
    console.error(`${c.red}Error: Could not locate r2modman Valheim profiles directory.${c.reset}`);
    console.error(`Please provide --profiles-dir <path>`);
    process.exit(1);
  }

  const profiles = getAvailableProfiles(profilesDir);

  if (profiles.length === 0) {
    throw new Error(`No Valheim profiles with mods.yml found in ${profilesDir}`);
  }

  if (opts.listProfiles) {
    console.log(`${c.bold}${c.cyan}Detected Valheim profiles at:${c.reset} ${profilesDir}\n`);
    for (const p of profiles) {
      const mods = parseModsYml(path.join(profilesDir, p));
      console.log(` • ${c.bold}${p}${c.reset} (${mods.length} mods, ${mods.filter(m => m.enabled).length} enabled)`);
    }
    return;
  }

  let selectedProfile = opts.profile || (profiles.includes('Default') ? 'Default' : profiles[0]);
  if (!profiles.includes(selectedProfile)) {
    throw new Error(`Profile '${selectedProfile}' was not found in ${profilesDir}`);
  }

  if (opts.listMods) {
    const profilePath = path.join(profilesDir, selectedProfile);
    const mods = parseModsYml(profilePath);
    console.log(`\n${c.bold}${c.cyan}Mods in profile [${selectedProfile}] (${mods.length}):${c.reset}\n`);
    mods.forEach((m, idx) => {
      const status = m.enabled ? `${c.green}[enabled]${c.reset}` : `${c.dim}[disabled]${c.reset}`;
      console.log(` ${String(idx + 1).padStart(2)}. ${status} ${c.bold}${m.displayName}${c.reset} (${m.name}) v${m.version}`);
    });
    return;
  }

  if (opts.listConfigs) {
    const profilePath = path.join(profilesDir, selectedProfile);
    const configs = scanProfileConfigs(profilePath);
    console.log(`\n${c.bold}${c.cyan}Configs in profile [${selectedProfile}] (${configs.length}):${c.reset}\n`);
    configs.forEach((cfg, idx) => {
      let tag = `${c.green}[SAFE]${c.reset}`;
      if (cfg.category === 'SENSITIVE') tag = `${c.red}[SENSITIVE / DAT]${c.reset}`;
      else if (cfg.category === 'CACHE') tag = `${c.yellow}[CACHE / REPORT]${c.reset}`;
      const type = cfg.isDir ? `[DIR]` : `[FILE]`;
      console.log(` ${String(idx + 1).padStart(2)}. ${tag} ${type} ${cfg.name}`);
    });
    return;
  }

  // INTERACTIVE MODE
  if (opts.interactive) {
    const prompter = createPrompter();
    console.log(`\n${c.bold}${c.cyan}╔══════════════════════════════════════════════════════════╗${c.reset}`);
    console.log(`${c.bold}${c.cyan}║      Valheim Modpack Generator for Thunderstore / r2     ║${c.reset}`);
    console.log(`${c.bold}${c.cyan}╚══════════════════════════════════════════════════════════╝${c.reset}\n`);

    // 1. Profile selection
    console.log(`${c.bold}Step 1: Select Profile${c.reset}`);
    profiles.forEach((p, idx) => {
      const mods = parseModsYml(path.join(profilesDir, p));
      const isDef = p === selectedProfile ? ` ${c.green}(default)${c.reset}` : '';
      console.log(`  [${idx + 1}] ${c.bold}${p}${c.reset}${isDef} - ${mods.length} mods (${mods.filter(m => m.enabled).length} enabled)`);
    });
    const pAns = await prompter.question(`\nSelect profile [1-${profiles.length}] (default: 1): `);
    const pIdx = parseInt(pAns.trim(), 10) - 1;
    if (!isNaN(pIdx) && profiles[pIdx]) {
      selectedProfile = profiles[pIdx];
    }
    console.log(`Selected: ${c.bold}${c.green}${selectedProfile}${c.reset}\n`);

    const profilePath = path.join(profilesDir, selectedProfile);
    const allMods = parseModsYml(profilePath);
    const allConfigs = scanProfileConfigs(profilePath);

    // 2. Modpack Metadata
    console.log(`${c.bold}Step 2: Modpack Metadata${c.reset}`);
    let defaultPackName = `${selectedProfile}_Modpack`.replace(/[^a-zA-Z0-9_]/g, '_');
    let packName = await prompter.question(`Modpack Name (letters, numbers, underscores only) [${defaultPackName}]: `);
    packName = packName.trim() || defaultPackName;
    packName = packName.replace(/[^a-zA-Z0-9_]/g, '_');

    let packVer = await prompter.question(`Modpack Version [1.0.0]: `);
    packVer = packVer.trim() || '1.0.0';
    if (!/^\d+\.\d+\.\d+$/.test(packVer)) {
      console.log(`${c.yellow}Warning: Thunderstore requires SemVer X.Y.Z format. Defaulting to 1.0.0${c.reset}`);
      packVer = '1.0.0';
    }

    let packDesc = await prompter.question(`Short Description [Curated Valheim modpack with custom configurations]: `);
    packDesc = packDesc.trim() || 'Curated Valheim modpack with custom configurations';

    let packWeb = await prompter.question(`Website / GitHub URL (optional) []: `);
    packWeb = packWeb.trim();
    console.log('');

    // 3. Granular Mod Selection
    console.log(`${c.bold}Step 3: Mod Selection${c.reset}`);
    console.log(`  [1] Include all enabled mods (${allMods.filter(m => m.enabled).length} mods) ${c.green}[Recommended]${c.reset}`);
    console.log(`  [2] Granular selection: toggle individual mods`);
    console.log(`  [3] Include all mods (both enabled and disabled, ${allMods.length} mods)`);

    const modChoice = (await prompter.question(`Choose mod selection mode [1-3] (default: 1): `)).trim();
    let selectedMods = [];

    if (modChoice === '2') {
      const activeState = allMods.map(m => m.enabled);
      let selecting = true;
      while (selecting) {
        console.log(`\n${c.bold}Current Mod Selection:${c.reset}`);
        allMods.forEach((m, idx) => {
          const mark = activeState[idx] ? `${c.green}[✔]${c.reset}` : `${c.dim}[ ]${c.reset}`;
          console.log(`  ${mark} ${String(idx + 1).padStart(2)}. ${m.displayName} (v${m.version})`);
        });
        console.log(`\nOptions: enter numbers to toggle (e.g. '3', '1,4,7', '1-5'), 'a' for all, 'n' for none, 'done' to finish.`);
        const toggleAns = (await prompter.question(`Toggle items: `)).trim().toLowerCase();

        if (toggleAns === 'done' || toggleAns === '') {
          selecting = false;
        } else if (toggleAns === 'a') {
          activeState.fill(true);
        } else if (toggleAns === 'n') {
          activeState.fill(false);
        } else {
          const parts = toggleAns.split(/[\s,]+/);
          for (const part of parts) {
            if (part.includes('-')) {
              const [s, e] = part.split('-').map(Number);
              if (!isNaN(s) && !isNaN(e)) {
                for (let i = s; i <= e; i++) {
                  if (i >= 1 && i <= allMods.length) activeState[i - 1] = !activeState[i - 1];
                }
              }
            } else {
              const num = parseInt(part, 10);
              if (!isNaN(num) && num >= 1 && num <= allMods.length) {
                activeState[num - 1] = !activeState[num - 1];
              }
            }
          }
        }
      }
      selectedMods = allMods.filter((_, idx) => activeState[idx]);
    } else if (modChoice === '3') {
      selectedMods = [...allMods];
    } else {
      selectedMods = allMods.filter(m => m.enabled);
    }
    console.log(`Selected ${c.bold}${c.green}${selectedMods.length}${c.reset} mods.\n`);

    // 4. Mod Version Overrides
    const verOverrideAns = (await prompter.question(`Do you want to override/customize any mod version strings? (y/N): `)).trim().toLowerCase();
    if (verOverrideAns === 'y' || verOverrideAns === 'yes') {
      let customizing = true;
      while (customizing) {
        console.log(`\n${c.bold}Mods in Pack:${c.reset}`);
        selectedMods.forEach((m, idx) => {
          console.log(`  [${idx + 1}] ${m.displayName}: ${c.cyan}${m.version}${c.reset}`);
        });
        const pickMod = (await prompter.question(`Select mod number to change version (or press Enter to continue): `)).trim();
        const mIdx = parseInt(pickMod, 10) - 1;
        if (!isNaN(mIdx) && selectedMods[mIdx]) {
          const newV = (await prompter.question(`Enter new version for ${selectedMods[mIdx].displayName} [${selectedMods[mIdx].version}]: `)).trim();
          if (newV) {
            selectedMods[mIdx].version = newV;
            selectedMods[mIdx].depString = `${selectedMods[mIdx].name}-${newV}`;
            console.log(`${c.green}Updated ${selectedMods[mIdx].displayName} -> v${newV}${c.reset}`);
          }
        } else {
          customizing = false;
        }
      }
    }
    console.log('');

    // 5. Config Selection
    console.log(`${c.bold}Step 4: Configuration Files${c.reset}`);
    const safeConfigs = allConfigs.filter(c => c.category === 'SAFE');
    const sensitiveConfigs = allConfigs.filter(c => c.category === 'SENSITIVE');
    const cacheConfigs = allConfigs.filter(c => c.category === 'CACHE');

    console.log(`Found ${allConfigs.length} configuration items in profile:`);
    console.log(`  • ${safeConfigs.length} mod configuration files/directories`);
    if (sensitiveConfigs.length > 0) {
      console.log(`  • ${c.red}${sensitiveConfigs.length} sensitive / player state files:${c.reset} ${sensitiveConfigs.map(s => s.name).join(', ')}`);
      console.log(`    ${c.dim}(Auto-excluded by default to prevent leaking server passwords and player IDs)${c.reset}`);
    }
    if (cacheConfigs.length > 0) {
      console.log(`  • ${c.yellow}${cacheConfigs.length} runtime cache/report items:${c.reset} ${cacheConfigs.map(s => s.name).join(', ')}`);
      console.log(`    ${c.dim}(Auto-excluded by default to keep package size minimal)${c.reset}`);
    }

    console.log(`\n  [1] Include all safe configs (${safeConfigs.length} items) ${c.green}[Recommended]${c.reset}`);
    console.log(`  [2] Granular selection: pick specific config files/folders`);
    console.log(`  [3] Skip configs (generate mod list only)`);

    const cfgChoice = (await prompter.question(`Choose config selection mode [1-3] (default: 1): `)).trim();
    let selectedConfigs = [];

    if (cfgChoice === '2') {
      const activeState = allConfigs.map(c => c.category === 'SAFE');
      let selecting = true;
      while (selecting) {
        console.log(`\n${c.bold}Current Config Selection:${c.reset}`);
        allConfigs.forEach((cfg, idx) => {
          const mark = activeState[idx] ? `${c.green}[✔]${c.reset}` : `${c.dim}[ ]${c.reset}`;
          let tag = '';
          if (cfg.category === 'SENSITIVE') tag = ` ${c.red}(SENSITIVE / DAT)${c.reset}`;
          else if (cfg.category === 'CACHE') tag = ` ${c.yellow}(CACHE / REPORT)${c.reset}`;
          console.log(`  ${mark} ${String(idx + 1).padStart(2)}. ${cfg.name}${tag}`);
        });
        console.log(`\nOptions: enter numbers to toggle (e.g. '1,3-5'), 'a' for all, 'n' for none, 'done' to finish.`);
        const toggleAns = (await prompter.question(`Toggle configs: `)).trim().toLowerCase();

        if (toggleAns === 'done' || toggleAns === '') {
          selecting = false;
        } else if (toggleAns === 'a') {
          activeState.fill(true);
        } else if (toggleAns === 'n') {
          activeState.fill(false);
        } else {
          const parts = toggleAns.split(/[\s,]+/);
          for (const part of parts) {
            if (part.includes('-')) {
              const [s, e] = part.split('-').map(Number);
              if (!isNaN(s) && !isNaN(e)) {
                for (let i = s; i <= e; i++) {
                  if (i >= 1 && i <= allConfigs.length) activeState[i - 1] = !activeState[i - 1];
                }
              }
            } else {
              const num = parseInt(part, 10);
              if (!isNaN(num) && num >= 1 && num <= allConfigs.length) {
                activeState[num - 1] = !activeState[num - 1];
              }
            }
          }
        }
      }
      selectedConfigs = allConfigs.filter((_, idx) => activeState[idx]);
    } else if (cfgChoice === '3') {
      selectedConfigs = [];
    } else {
      selectedConfigs = safeConfigs;
    }
    console.log(`Selected ${c.bold}${c.green}${selectedConfigs.length}${c.reset} config items.\n`);

    // 6. Icon
    console.log(`${c.bold}Step 5: Icon${c.reset}`);
    console.log(`  [1] Auto-generate sleek 256x256 Valheim rune icon ${c.green}[Default]${c.reset}`);
    console.log(`  [2] Specify path to custom 256x256 PNG file`);
    const iconChoice = (await prompter.question(`Choose icon option [1-2] (default: 1): `)).trim();
    let customIcon = '';
    if (iconChoice === '2') {
      const iconPath = (await prompter.question(`Enter path to 256x256 icon.png: `)).trim();
      if (fs.existsSync(iconPath)) {
        customIcon = iconPath;
      } else {
        console.log(`${c.yellow}File not found. Generating default icon instead.${c.reset}`);
      }
    }
    console.log('');

    // Summary & confirmation
    console.log(`${c.bold}${c.cyan}═══ Build Summary ═══${c.reset}`);
    console.log(` Profile:         ${c.bold}${selectedProfile}${c.reset}`);
    console.log(` Modpack:         ${c.bold}${packName} v${packVer}${c.reset}`);
    console.log(` Mods:            ${c.green}${selectedMods.length} dependencies${c.reset}`);
    console.log(` Configs:         ${c.green}${selectedConfigs.length} items${c.reset}`);
    console.log(` Target output:   ${c.bold}dist/${packName}-${packVer}.zip${c.reset}\n`);

    const confirm = (await prompter.question(`Generate modpack files and ZIP now? (Y/n): `)).trim().toLowerCase();
    prompter.close();

    if (confirm === 'n' || confirm === 'no') {
      console.log(`${c.yellow}Build canceled.${c.reset}`);
      return;
    }

    console.log(`\n${c.bold}Building modpack...${c.reset}`);
    const result = buildModpack({
      profilePath,
      name: packName,
      version: packVer,
      description: packDesc,
      websiteUrl: packWeb,
      selectedMods,
      selectedConfigs,
      customIconPath: customIcon,
      outDir: opts.out,
      distDir: opts.dist,
    });

    console.log(`\n${c.bold}${c.green}✔ Modpack successfully generated!${c.reset}`);
    console.log(`  📁 Unpacked Folder: ${c.bold}${result.buildFolder}${c.reset}`);
    console.log(`  📦 Thunderstore ZIP: ${c.bold}${result.zipFile}${c.reset} (${(result.zipSize / 1024).toFixed(1)} KB)`);
    console.log(`\n${c.bold}Next steps to upload to Thunderstore:${c.reset}`);
    console.log(`  1. Log into ${c.cyan}https://thunderstore.io/c/valheim/${c.reset}`);
    console.log(`  2. Click ${c.bold}"Upload"${c.reset} in the top-right header.`);
    console.log(`  3. Drag and drop ${c.bold}${result.zipFile}${c.reset}`);
    console.log(`  4. Select your community team, add the "Modpacks" category, and submit!`);

  } else {
    // AUTOMATED / NON-INTERACTIVE MODE
    const profilePath = path.join(profilesDir, selectedProfile);
    const allMods = parseModsYml(profilePath);
    const allConfigs = scanProfileConfigs(profilePath);

    const name = (opts.name || `${selectedProfile}_Modpack`).replace(/[^a-zA-Z0-9_]/g, '_');
    const version = opts.version || '1.0.0';
    const description = opts.desc || 'Curated Valheim modpack';
    const websiteUrl = opts.website || '';

    const selectedMods = allMods.filter(m => m.enabled);
    let selectedConfigs = allConfigs.filter(c => c.category === 'SAFE');
    if (opts.includeSensitive) {
      selectedConfigs = selectedConfigs.concat(allConfigs.filter(c => c.category === 'SENSITIVE'));
    }
    if (opts.includeCaches) {
      selectedConfigs = selectedConfigs.concat(allConfigs.filter(c => c.category === 'CACHE'));
    }

    console.log(`Generating modpack ${name} v${version} from profile [${selectedProfile}]...`);
    const result = buildModpack({
      profilePath,
      name,
      version,
      description,
      websiteUrl,
      selectedMods,
      selectedConfigs,
      customIconPath: opts.icon,
      outDir: opts.out,
      distDir: opts.dist,
    });

    console.log(`✔ Modpack created: ${result.zipFile} (${(result.zipSize / 1024).toFixed(1)} KB)`);
    console.log(`  Mods: ${result.modCount}, Configs: ${result.configCount}`);
  }
}

main().catch(err => {
  console.error(`${c.red}Fatal Error:${c.reset}`, err.message);
  process.exit(1);
});
