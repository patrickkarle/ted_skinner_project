// Script to generate icon files from the square logo
// Windows ICO requires: 16, 24, 32, 48, 64, and 256 pixels
// IMPORTANT: For optimal dev display, 32px should be first layer in ICO
import sharp from 'sharp';
import pngToIco from 'png-to-ico';
import { writeFileSync, mkdirSync } from 'fs';
import { dirname, join } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const projectRoot = join(__dirname, '..');
const sourceImage = join(projectRoot, 'image-resources', 'fullintel_logo.jpg');
const iconsDir = join(projectRoot, 'src-tauri', 'icons');

async function generateIcons() {
  console.log('Generating icons from:', sourceImage);
  console.log('Output directory:', iconsDir);

  // Ensure icons directory exists
  mkdirSync(iconsDir, { recursive: true });

  // Generate PNG icons at all Windows recommended sizes
  // 16x16 - Small icons, system tray
  // 24x24 - Some toolbar contexts
  // 32x32 - Standard taskbar, title bar (MOST IMPORTANT for dev)
  // 48x48 - Medium icons
  // 64x64 - Large icons
  // 128x128 - Tauri default
  // 256x256 - High-DPI, large icon view
  const sizes = [16, 24, 32, 48, 64, 128, 256];

  for (const size of sizes) {
    const outputPath = join(iconsDir, `${size}x${size}.png`);
    await sharp(sourceImage)
      .resize(size, size, { fit: 'contain', background: { r: 255, g: 255, b: 255, alpha: 1 } })
      .png()
      .toFile(outputPath);
    console.log(`Generated: ${size}x${size}.png`);
  }

  // Generate ICO file with 32px FIRST for optimal dev display
  // Order matters! First layer is shown in development mode
  const icoBuffer = await pngToIco([
    join(iconsDir, '32x32.png'),    // First for dev mode display
    join(iconsDir, '16x16.png'),
    join(iconsDir, '24x24.png'),
    join(iconsDir, '48x48.png'),
    join(iconsDir, '64x64.png'),
    join(iconsDir, '128x128.png'),
    join(iconsDir, '256x256.png')
  ]);

  const icoPath = join(iconsDir, 'icon.ico');
  writeFileSync(icoPath, icoBuffer);
  console.log('Generated: icon.ico (with 32px as first layer for dev mode)');

  console.log('\nAll icons generated successfully!');
  console.log('Icon includes layers: 16, 24, 32, 48, 64, 128, 256 pixels');
}

generateIcons().catch(console.error);
