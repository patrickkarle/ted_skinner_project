// Script to generate icon files from the square logo
// Cross-platform support: Windows, macOS, Linux
// Windows ICO requires: 16, 24, 32, 48, 64, and 256 pixels
// macOS requires: icon.icns and retina @2x variants
// Linux requires: icon.png (512x512)
// IMPORTANT: For optimal dev display, 32px should be first layer in ICO
import sharp from 'sharp';
import pngToIco from 'png-to-ico';
import { writeFileSync, mkdirSync, existsSync } from 'fs';
import { dirname, join } from 'path';
import { fileURLToPath } from 'url';
import { execSync } from 'child_process';

const __dirname = dirname(fileURLToPath(import.meta.url));
const projectRoot = join(__dirname, '..');
const sourceImage = join(projectRoot, 'image-resources', 'fullintel_logo.jpg');
const iconsDir = join(projectRoot, 'src-tauri', 'icons');

async function generateIcons() {
  console.log('Generating icons from:', sourceImage);
  console.log('Output directory:', iconsDir);

  // Ensure icons directory exists
  mkdirSync(iconsDir, { recursive: true });

  // Generate PNG icons at all required sizes for cross-platform
  // 16x16 - Small icons, system tray
  // 24x24 - Some toolbar contexts
  // 32x32 - Standard taskbar, title bar (MOST IMPORTANT for dev)
  // 48x48 - Medium icons
  // 64x64 - Large icons
  // 128x128 - Tauri default
  // 256x256 - High-DPI, large icon view, and 128x128@2x for macOS retina
  // 512x512 - Linux/macOS master icon
  const sizes = [16, 24, 32, 48, 64, 128, 256, 512];

  for (const size of sizes) {
    const outputPath = join(iconsDir, `${size}x${size}.png`);
    await sharp(sourceImage)
      .resize(size, size, { fit: 'contain', background: { r: 255, g: 255, b: 255, alpha: 1 } })
      .ensureAlpha()  // Ensure RGBA format (required by Tauri)
      .png()
      .toFile(outputPath);
    console.log(`Generated: ${size}x${size}.png (RGBA)`);
  }

  // Generate macOS retina icon (128x128@2x = 256x256 pixels)
  const retinaPath = join(iconsDir, '128x128@2x.png');
  await sharp(sourceImage)
    .resize(256, 256, { fit: 'contain', background: { r: 255, g: 255, b: 255, alpha: 1 } })
    .ensureAlpha()  // Ensure RGBA format (required by Tauri)
    .png()
    .toFile(retinaPath);
  console.log('Generated: 128x128@2x.png (macOS retina, RGBA)');

  // Generate master icon.png (512x512 for Linux/cross-platform)
  const masterIconPath = join(iconsDir, 'icon.png');
  await sharp(sourceImage)
    .resize(512, 512, { fit: 'contain', background: { r: 255, g: 255, b: 255, alpha: 1 } })
    .ensureAlpha()  // Ensure RGBA format (required by Tauri)
    .png()
    .toFile(masterIconPath);
  console.log('Generated: icon.png (512x512 master, RGBA)');

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

  // Note: icon.icns generation requires macOS or special tools
  // The existing icon.icns can be regenerated on macOS using:
  // iconutil -c icns icon.iconset
  // Or use a tool like png2icns on Linux
  console.log('\nNote: icon.icns requires macOS iconutil or png2icns to regenerate');
  console.log('Existing icon.icns will be used if present');

  console.log('\n✅ All icons generated successfully!');
  console.log('Icon includes layers: 16, 24, 32, 48, 64, 128, 256, 512 pixels');
  console.log('Cross-platform ready: Windows (ico), macOS (icns, @2x), Linux (png)');
}

generateIcons().catch(console.error);
