#!/bin/bash
# Build script to copy frontend assets to dist directory

echo "Building frontend assets..."

# Create dist directory if it doesn't exist
mkdir -p dist

# Copy menu files
echo "Copying menu files..."
cp src/menu.html dist/
cp src/menu.css dist/
cp src/menu.js dist/

# Copy preferences files
echo "Copying preferences files..."
cp src/preferences.html dist/
cp src/preferences.css dist/
cp src/preferences.js dist/

# Copy other assets if needed
if [ -f src/index.html ]; then
  cp src/index.html dist/
fi

if [ -f src/main.js ]; then
  cp src/main.js dist/
fi

if [ -f src/styles.css ]; then
  cp src/styles.css dist/
fi

# Copy assets directory if it exists
if [ -d src/assets ]; then
  cp -r src/assets dist/
fi

echo "Build complete! dist/ directory contents:"
ls -la dist/
