#!/bin/bash
set -e

# Compress election data from raw-data/ to archives/
# This allows us to:
# - Keep raw-data/ as uncompressed working directory (gitignored)
# - Commit archives/ to git with compressed tar.xz files
# Uses parallel compression with all CPU cores

cd "$(dirname "$0")"

SOURCE_DIR="raw-data"
ARCHIVE_DIR="archives"

# Determine number of parallel jobs (CPU cores)
if [[ $OSTYPE == "darwin"* ]]; then
	JOBS=$(sysctl -n hw.ncpu)
else
	JOBS=$(nproc)
fi

# Compress one election at a time and let that compressor use all cores. Running
# multiple all-core XZ processes in parallel can exhaust memory and leave
# incomplete archives if a process is killed.
COMPRESSION_JOBS=1

echo "=== Election Data Compression ==="
echo "Source: $SOURCE_DIR/ (working directory)"
echo "Target: $ARCHIVE_DIR/ (for git)"
echo "Compression threads: $JOBS"
echo ""

# Create archives directory structure
mkdir -p "$ARCHIVE_DIR"

# Function to compress an election directory using only files from metadata
compress_election() {
	local target_path="$1"
	set -o pipefail

	# Read file list from mapping file
	local file_list=""
	local archive_entire_dir=false

	while IFS='|' read -r dir filename; do
		if [ "$dir" = "$target_path" ]; then
			if [ "$filename" = "*" ]; then
				# Special marker: archive entire directory
				archive_entire_dir=true
				break
			else
				if [ -z "$file_list" ]; then
					file_list="$filename"
				else
					file_list="$file_list|$filename"
				fi
			fi
		fi
	done <"$TEMP_MAPPING"

	if [ "$archive_entire_dir" = false ] && [ -z "$file_list" ]; then
		echo "  [SKIP] $target_path (no files in metadata)"
		return 0
	fi

	local relative_path
	relative_path="${target_path#"$SOURCE_DIR"/}"
	local parent_dir
	parent_dir=$(dirname "$relative_path")
	local dir_name
	dir_name=$(basename "$target_path")

	# Create target directory
	mkdir -p "$ARCHIVE_DIR/$parent_dir"

	local archive_path="$ARCHIVE_DIR/$parent_dir/$dir_name.tar.xz"

	# Check if archive exists and if any source files are newer
	local needs_update=false
	local update_reason="source changed"
	if [ -f "$archive_path" ]; then
		if ! tar -tJf "$archive_path" >/dev/null 2>&1; then
			needs_update=true
			update_reason="existing archive is invalid"
		elif [ "$archive_entire_dir" = true ]; then
			# Check if any file in directory is newer than archive
			if find "$target_path" -type f ! -name "*.pdf" ! -name ".*" -newer "$archive_path" 2>/dev/null | head -1 | grep -q .; then
				needs_update=true
			fi
		else
			# Check if any source file is newer than archive
			IFS='|' read -ra FILES <<<"$file_list"
			for filename in "${FILES[@]}"; do
				file_path="$target_path/$filename"
				if [ -f "$file_path" ] && [ "$file_path" -nt "$archive_path" ]; then
					needs_update=true
					break
				fi
			done
		fi

		if [ "$needs_update" = false ]; then
			return 0
		fi
		echo "  [UPDATE] $relative_path ($update_reason)"
	fi

	# Calculate size before
	local size_before=0
	local file_count=0

	if [ "$archive_entire_dir" = true ]; then
		# Batch stat calls so large extracted CVR directories do not spawn one
		# process per file just to calculate progress information.
		if [[ $OSTYPE == "darwin"* ]]; then
			read -r size_before file_count < <(
				find "$target_path" -type f ! -name "*.pdf" ! -name ".*" -exec stat -f '%z' {} + 2>/dev/null |
					awk '{ total += $1; count++ } END { print total + 0, count + 0 }'
			)
		else
			read -r size_before file_count < <(
				find "$target_path" -type f ! -name "*.pdf" ! -name ".*" -exec stat -c '%s' {} + 2>/dev/null |
					awk '{ total += $1; count++ } END { print total + 0, count + 0 }'
			)
		fi
	else
		IFS='|' read -ra FILES <<<"$file_list"
		file_count=${#FILES[@]}
		for filename in "${FILES[@]}"; do
			file_path="$target_path/$filename"
			if [ -f "$file_path" ]; then
				size_before=$((size_before + $(stat -f%z "$file_path" 2>/dev/null || stat -c%s "$file_path" 2>/dev/null || echo 0)))
			fi
		done
	fi

	size_before_human=$(numfmt --to=iec-i --suffix=B "$size_before" 2>/dev/null || echo "${size_before}B")

	echo "  [START] $relative_path ($size_before_human, $file_count files)"
	local temp_archive
	temp_archive=$(mktemp "$ARCHIVE_DIR/$parent_dir/.${dir_name}.tar.xz.tmp.XXXXXX")
	trap 'rm -f "$temp_archive"' EXIT

	# Create tar.xz archive
	if [ "$archive_entire_dir" = true ]; then
		# Archive entire directory excluding PDFs
		if command -v pixz &>/dev/null; then
			if ! tar -cf - --exclude="*.pdf" -C "$SOURCE_DIR/$parent_dir" "$dir_name/" | pixz -p "$JOBS" -9 >"$temp_archive"; then
				rm -f "$temp_archive"
				echo "  [ERROR] Compression failed!"
				return 1
			fi
		else
			# macOS tar compresses -J in-process and ignores XZ_OPT threading.
			# Pipe to xz explicitly so -T0 actually uses all available cores.
			if ! tar -cf - --exclude="*.pdf" -C "$SOURCE_DIR/$parent_dir" "$dir_name/" | xz -9 -T0 >"$temp_archive"; then
				rm -f "$temp_archive"
				echo "  [ERROR] Compression failed!"
				return 1
			fi
		fi
	else
		# Archive only specific files
		local tar_files=()
		IFS='|' read -ra FILES <<<"$file_list"
		for filename in "${FILES[@]}"; do
			if [ -f "$target_path/$filename" ]; then
				tar_files+=("$dir_name/$filename")
			fi
		done

		if [ ${#tar_files[@]} -eq 0 ]; then
			rm -f "$temp_archive"
			echo "  [SKIP] $relative_path (no files found)"
			return 0
		fi

		if command -v pixz &>/dev/null; then
			if ! tar -cf - -C "$SOURCE_DIR/$parent_dir" "${tar_files[@]}" | pixz -p "$JOBS" -9 >"$temp_archive"; then
				rm -f "$temp_archive"
				echo "  [ERROR] Compression failed!"
				return 1
			fi
		else
			if ! tar -cf - -C "$SOURCE_DIR/$parent_dir" "${tar_files[@]}" | xz -9 -T0 >"$temp_archive"; then
				rm -f "$temp_archive"
				echo "  [ERROR] Compression failed!"
				return 1
			fi
		fi
	fi

	# Verify the new archive before atomically replacing the existing one.
	if ! tar -tJf "$temp_archive" >/dev/null 2>&1; then
		echo "  [ERROR] Verification failed!"
		rm -f "$temp_archive"
		return 1
	fi

	mv "$temp_archive" "$archive_path"
	local size_after
	size_after=$(du -sh "$archive_path" | cut -f1)
	echo "  [DONE] $archive_path ($size_after)"
	echo "  [OK] Verified"
}

export SOURCE_DIR
export ARCHIVE_DIR
export JOBS

echo "Step 1: Reading election metadata to determine files to archive..."
echo ""

# Use metadata files to determine which files actually need to be archived
# This ensures we only archive files that are actually used, excluding PDFs and other unnecessary files
META_DIR="election-metadata"

# Create a temporary file to store election directory -> files mapping
# (bash associative arrays can't be exported to subshells)
TEMP_MAPPING=$(mktemp)
VALIDATION_ERRORS=$(mktemp)
trap 'rm -f "$TEMP_MAPPING" "$VALIDATION_ERRORS"' EXIT

# Read all metadata files and extract file lists
while IFS= read -r meta_file; do
	# Extract jurisdiction path from metadata
	jurisdiction_path=$(jq -r '.path' "$meta_file" 2>/dev/null)
	if [ -z "$jurisdiction_path" ] || [ "$jurisdiction_path" = "null" ]; then
		continue
	fi

	# Process each election in the metadata
	jq -c '.elections | to_entries[]' "$meta_file" 2>/dev/null | while IFS= read -r election_entry; do
		election_key=$(echo "$election_entry" | jq -r '.key')
		election_data=$(echo "$election_entry" | jq -r '.value')

		# Construct the election directory path
		election_dir="$SOURCE_DIR/$jurisdiction_path/$election_key"

		# Check if election has explicit files list
		files_count=$(echo "$election_data" | jq '.files | length')
		data_format=$(echo "$election_data" | jq -r '.dataFormat // empty')

		if [ "$files_count" -gt 0 ]; then
			# Use explicit files list
			found_explicit_file=false
			while IFS= read -r filename; do
				file_path="$election_dir/$filename"
				if [ -f "$file_path" ]; then
					echo "$election_dir|$filename" >>"$TEMP_MAPPING"
					found_explicit_file=true
				fi
			done < <(echo "$election_data" | jq -r '.files | keys[]')

			# Some NIST ZIP exports are stored extracted so report generation can
			# batch-process them. Archive the extracted election directory when the
			# metadata-named ZIP is absent but the required NIST files are present.
			if [ "$found_explicit_file" = false ] &&
				[ "$data_format" = "nist_sp_1500" ] &&
				[ -f "$election_dir/CandidateManifest.json" ] &&
				find "$election_dir" -maxdepth 1 -type f -name 'CvrExport*.json' -print -quit | grep -q .; then
				echo "$election_dir|*" >>"$TEMP_MAPPING"
			elif [ "$found_explicit_file" = false ] &&
				[ "$data_format" = "nist_sp_1500" ] &&
				[ -d "$election_dir" ]; then
				echo "Missing metadata-named ZIP and complete extracted NIST data: $election_dir" >>"$VALIDATION_ERRORS"
			fi
		else
			# No explicit files - check loaderParams for directory references
			# For NIST format, look for "cvr" parameter
			cvr_dir=$(echo "$election_data" | jq -r '.contests[0].loaderParams.cvr // empty' 2>/dev/null)
			if [ -n "$cvr_dir" ] && [ "$cvr_dir" != "null" ]; then
				# Archive the entire CVR directory (excluding PDFs)
				# Handle "." as current directory
				if [[ ${cvr_dir} == "." ]]; then
					cvr_path="${election_dir}"
				else
					cvr_path="$election_dir/$cvr_dir"
				fi
				if [ -d "$cvr_path" ]; then
					if [ "$data_format" != "nist_sp_1500" ] ||
						{ [ -f "$cvr_path/CandidateManifest.json" ] &&
							find "$cvr_path" -maxdepth 1 -type f -name 'CvrExport*.json' -print -quit | grep -q .; }; then
						# Mark this directory for archiving.
						echo "$cvr_path|*" >>"$TEMP_MAPPING"
					else
						echo "Incomplete extracted NIST data: $cvr_path" >>"$VALIDATION_ERRORS"
					fi
				fi
			else
				# Check for "file" parameter (like Minneapolis)
				file_param=$(echo "$election_data" | jq -r '.contests[0].loaderParams.file // empty' 2>/dev/null)
				if [ -n "$file_param" ] && [ "$file_param" != "null" ]; then
					file_path="$election_dir/$file_param"
					if [ -f "$file_path" ]; then
						echo "$election_dir|$file_param" >>"$TEMP_MAPPING"
					fi
				fi
				# Note: No fallback - only archive what's explicitly in metadata
			fi
		fi
	done
done < <(find "$META_DIR" -name "*.json" -type f 2>/dev/null | sort)

if [ -s "$VALIDATION_ERRORS" ]; then
	echo "Error: Refusing to compress incomplete election data:" >&2
	sed 's/^/  - /' "$VALIDATION_ERRORS" >&2
	exit 1
fi

# Extract unique election directories from mapping file
ELECTION_DIRS=()
while IFS= read -r election_dir; do
	ELECTION_DIRS+=("$election_dir")
done < <(cut -d'|' -f1 "$TEMP_MAPPING" | sort -u)

# Export the mapping file path for use in compress_election function
export TEMP_MAPPING

echo "Found ${#ELECTION_DIRS[@]} elections to process"
echo ""
echo "Step 2: Compressing one election at a time (using $JOBS threads)..."
echo ""

# Export compress_election function for parallel execution
export -f compress_election

# Run each compression in an isolated shell. The compressor itself uses all
# available cores, so multiple elections would compete for the same resources.
printf '%s\n' "${ELECTION_DIRS[@]}" | xargs -P "$COMPRESSION_JOBS" -I {} bash -c 'compress_election "$@"' _ {}

echo ""

echo "=== Compression Complete ==="
echo ""
echo "Archives directory structure:"
tree -h "$ARCHIVE_DIR" -L 4
echo ""
echo "Archive summary:"
find "$ARCHIVE_DIR" -name "*.tar.xz" -exec du -h {} \; | sort -h | tail -20
echo ""
echo "Total compressed size:"
du -sh "$ARCHIVE_DIR"
echo ""
echo "Original size (raw-data):"
du -sh "$SOURCE_DIR"
echo ""
echo "Next steps:"
echo "  1. Add archives/ to git: git add archives/"
echo "  2. Keep raw-data/ in .gitignore"
echo "  3. To extract: tar -xJf archives/path/to/election.tar.xz -C raw-data/path/to/"
