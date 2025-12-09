# LaReview Brand Assets

This directory contains the official brand assets for LaReview.

## Logo Files

- `lareview-icon.svg` - Main app icon source file
- `favicon.svg` - Favicon version for web integration
- `icons/` - Directory containing PNG versions in various sizes

## PNG Versions

The `icons/` directory contains the following PNG versions of the logo:
- `icon-16.png` - 16x16 pixels for small interface elements
- `icon-32.png` - 32x32 pixels for standard UI elements and window icons
- `icon-64.png` - 64x64 pixels for larger UI elements
- `icon-128.png` - 128x128 pixels for high-resolution displays
- `icon-256.png` - 256x256 pixels for best quality displays
- `icon-512.png` - 512x512 pixels for the highest quality displays

## Logo Concept

The LaReview logo represents the core concept of the application: structured code review organized around **Intents** and **Sub-flows**. The design uses a circle divided with vertical lines on one half, symbolizing the structured organization of review tasks. The purple color (#7C3AED) represents the intelligent, AI-powered approach to code review.

## Generating PNG Versions

The PNG versions were generated using ImageMagick with high-quality parameters:

```bash
# Create high quality PNGs with proper color space
magick -density 300 assets/lareview-icon.svg -background none -define png:color-type=2 -depth 8 -colorspace sRGB assets/icons/icon-256.png

# Create other sizes from the high-quality source
magick assets/icons/icon-256.png -resize 512x512 assets/icons/icon-512.png
magick assets/icons/icon-256.png -resize 128x128 assets/icons/icon-128.png
magick assets/icons/icon-256.png -resize 64x64 assets/icons/icon-64.png
magick assets/icons/icon-256.png -resize 32x32 assets/icons/icon-32.png
magick assets/icons/icon-256.png -resize 16x16 assets/icons/icon-16.png
```