#!/bin/bash
set -e

# Extract election data from archives/ to raw-data/
# This allows contributors to decompress the git-committed archives
# into their local working directory

cd "$(dirname "$0")"

ARCHIVE_DIR="archives"
TARGET_DIR="raw-data"

# Determine number of parallel jobs (CPU cores)
if [[ "$OSTYPE" == "darwin"* ]]; then
    JOBS=$(sysctl -n hw.ncpu)
else
    JOBS=$(nproc)
fi

echo "=== Election Data Extraction ==="
echo "Source: $ARCHIVE_DIR/ (git-committed archives)"
echo "Target: $TARGET_DIR/ (working directory)"
echo "Parallel jobs: $JOBS"
echo ""

# Check if archives directory exists
if [ ! -d "$ARCHIVE_DIR" ]; then
    echo "Error: $ARCHIVE_DIR/ directory not found!"
    echo "Make sure you're in the report_pipeline directory."
    exit 1
fi

# Create target directory
mkdir -p "$TARGET_DIR"

# Function to extract an election archive
extract_election() {
    local archive_path="$1"
    local relative_path
    relative_path="${archive_path#"$ARCHIVE_DIR"/}"
    local parent_dir
    parent_dir=$(dirname "$relative_path")
    local archive_name
    archive_name=$(basename "$archive_path" .tar.xz)

    # Determine target directory
    local target_parent="$TARGET_DIR/$parent_dir"
    local target_path="$target_parent/$archive_name"

    # Validate even archives whose destination already exists. Otherwise a
    # corrupt archive can be hidden indefinitely by a partial prior extraction.
    if ! tar -tJf "$archive_path" >/dev/null 2>&1; then
        echo "  [ERROR] Invalid archive: $archive_path"
        return 1
    fi

    # Skip if already extracted and up-to-date
    if [ -d "$target_path" ]; then
        # Check if archive is newer than extracted directory
        if [ "$archive_path" -nt "$target_path" ]; then
            echo "  [UPDATE] $relative_path (archive changed)"
        else
            return 0
        fi
    fi

    # Get archive size
    local archive_size
    archive_size=$(du -sh "$archive_path" | cut -f1)

    echo "  [START] Extracting $relative_path ($archive_size)"

    # Create target directory
    mkdir -p "$target_parent"
	local temp_parent
	temp_parent=$(mktemp -d "$target_parent/.${archive_name}.extract.XXXXXX")

    # Extract to a temporary sibling, then replace the destination only after
    # extraction has completed successfully.
    if tar -xJf "$archive_path" -C "$temp_parent" 2>/dev/null && [ -d "$temp_parent/$archive_name" ]; then
		rm -rf "$target_path"
		mv "$temp_parent/$archive_name" "$target_path"
		rm -rf "$temp_parent"
        local extracted_size
        extracted_size=$(du -sh "$target_path" | cut -f1)
        echo "  [DONE] $target_path ($extracted_size)"
    else
		rm -rf "$temp_parent"
        echo "  [ERROR] Failed to extract $archive_path"
        return 1
    fi
}

export -f extract_election
export ARCHIVE_DIR
export TARGET_DIR

echo "Step 1: Finding all archives..."
echo ""

# Find all tar.xz archives
ARCHIVES=()
while IFS= read -r archive_path; do
    ARCHIVES+=("$archive_path")
done < <(find "$ARCHIVE_DIR" -name "*.tar.xz" -type f | sort)

if [ ${#ARCHIVES[@]} -eq 0 ]; then
    echo "No archives found in $ARCHIVE_DIR/"
    exit 0
fi

echo "Found ${#ARCHIVES[@]} archives to extract"
echo ""
echo "Step 2: Extracting in parallel (using $JOBS cores)..."
echo ""

# Use parallel extraction with xargs
printf '%s\n' "${ARCHIVES[@]}" | xargs -P "$JOBS" -I {} bash -c 'extract_election "$@"' _ {}

echo ""
echo "=== Extraction Complete ==="
echo ""
echo "Extracted data location:"
du -sh "$TARGET_DIR"
echo ""
echo "Directory structure:"
tree -L 4 -d "$TARGET_DIR" 2>/dev/null || find "$TARGET_DIR" -type d | head -20
echo ""
echo "Summary:"
echo "  Archives: ${#ARCHIVES[@]}"
echo "  Target: $TARGET_DIR/"
echo ""
echo "Note: $TARGET_DIR/ is gitignored and safe to modify"
