#!/bin/bash

# Script to unzip all zip files in report_pipeline/raw-data/ directories
# Extracts each zip file in its own directory to avoid conflicts

set -e  # Exit on any error

echo "🚀 Starting to unzip all zip files in report_pipeline/raw-data/"
echo ""

# Counter for tracking progress
total_files=$(find report_pipeline/raw-data/ -name "*.zip" -type f | wc -l | tr -d ' ')
current_file=0

# Find all zip files and process them
find report_pipeline/raw-data/ -name "*.zip" -type f | while read -r zipfile; do
    current_file=$((current_file + 1))
    
    # Get the directory containing the zip file
    zip_dir=$(dirname "$zipfile")
    zip_basename=$(basename "$zipfile" .zip)
    
    echo "[$current_file/$total_files] Processing: $zipfile"
    
    # Create extraction directory based on zip filename
    extract_dir="$zip_dir/${zip_basename}_extracted"
    
    # Check if extraction directory already exists
    if [ -d "$extract_dir" ]; then
        echo "  ⚠️  Directory already exists: $extract_dir"
        echo "  ℹ️  Skipping (remove directory to force re-extraction)"
        echo ""
        continue
    fi
    
    # Create the extraction directory
    mkdir -p "$extract_dir"
    
    # Extract the zip file
    if unzip -q "$zipfile" -d "$extract_dir"; then
        echo "  ✅ Successfully extracted to: $extract_dir"
        
        # List what was extracted (first few items)
        echo "  📁 Contents:"
        ls "$extract_dir" | head -5 | sed 's/^/     - /'
        if [ $(ls "$extract_dir" | wc -l) -gt 5 ]; then
            echo "     ... and $(($(ls "$extract_dir" | wc -l) - 5)) more items"
        fi
    else
        echo "  ❌ Failed to extract: $zipfile"
        # Remove the failed extraction directory
        rmdir "$extract_dir" 2>/dev/null || true
    fi
    
    echo ""
done

echo "🎉 Finished processing all zip files!"
echo ""
echo "📊 Summary:"
echo "  Total zip files found: $total_files"
echo ""
echo "💡 To view extracted contents:"
echo "  find report_pipeline/raw-data/ -name '*_extracted' -type d"
