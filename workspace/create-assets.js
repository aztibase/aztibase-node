const sharp = require('sharp');
const path = require('path');

async function createAssets() {
  const dir = path.join(__dirname, 'slides');

  // Title slide gradient - pearl with lavender hint
  await sharp(Buffer.from(`<svg xmlns="http://www.w3.org/2000/svg" width="1440" height="810">
    <defs>
      <linearGradient id="g" x1="0%" y1="0%" x2="100%" y2="100%">
        <stop offset="0%" style="stop-color:#F8F6F9"/>
        <stop offset="40%" style="stop-color:#FDFBFD"/>
        <stop offset="100%" style="stop-color:#F4F0F7"/>
      </linearGradient>
    </defs>
    <rect width="100%" height="100%" fill="url(#g)"/>
  </svg>`)).png().toFile(path.join(dir, 'bg-title.png'));

  // Content slide gradient - warm pearl
  await sharp(Buffer.from(`<svg xmlns="http://www.w3.org/2000/svg" width="1440" height="810">
    <defs>
      <linearGradient id="g" x1="0%" y1="0%" x2="0%" y2="100%">
        <stop offset="0%" style="stop-color:#FDFCFD"/>
        <stop offset="100%" style="stop-color:#F9F7FA"/>
      </linearGradient>
    </defs>
    <rect width="100%" height="100%" fill="url(#g)"/>
  </svg>`)).png().toFile(path.join(dir, 'bg-content.png'));

  // Accent bar - deep violet-blue
  await sharp(Buffer.from(`<svg xmlns="http://www.w3.org/2000/svg" width="12" height="810">
    <defs>
      <linearGradient id="g" x1="0%" y1="0%" x2="0%" y2="100%">
        <stop offset="0%" style="stop-color:#4A3B8F"/>
        <stop offset="100%" style="stop-color:#6B5CA5"/>
      </linearGradient>
    </defs>
    <rect width="100%" height="100%" fill="url(#g)"/>
  </svg>`)).png().toFile(path.join(dir, 'accent-bar.png'));

  // Closing slide gradient - deeper pearl with violet
  await sharp(Buffer.from(`<svg xmlns="http://www.w3.org/2000/svg" width="1440" height="810">
    <defs>
      <linearGradient id="g" x1="0%" y1="0%" x2="100%" y2="100%">
        <stop offset="0%" style="stop-color:#4A3B8F"/>
        <stop offset="50%" style="stop-color:#5D4E9F"/>
        <stop offset="100%" style="stop-color:#6B5CA5"/>
      </linearGradient>
    </defs>
    <rect width="100%" height="100%" fill="url(#g)"/>
  </svg>`)).png().toFile(path.join(dir, 'bg-closing.png'));

  // Header bar for content slides
  await sharp(Buffer.from(`<svg xmlns="http://www.w3.org/2000/svg" width="1440" height="8">
    <defs>
      <linearGradient id="g" x1="0%" y1="0%" x2="100%" y2="0%">
        <stop offset="0%" style="stop-color:#4A3B8F"/>
        <stop offset="50%" style="stop-color:#6B5CA5"/>
        <stop offset="100%" style="stop-color:#D4A574"/>
      </linearGradient>
    </defs>
    <rect width="100%" height="100%" fill="url(#g)"/>
  </svg>`)).png().toFile(path.join(dir, 'header-bar.png'));

  console.log('Assets created');
}

createAssets().catch(console.error);
