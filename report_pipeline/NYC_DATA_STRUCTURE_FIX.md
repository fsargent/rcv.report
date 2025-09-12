# NYC Election Data Structure Analysis and Fix Plan

## Current Problem

The current implementation incorrectly processes all NYC ballots and then filters them after reading, leading to:
- **325,335 total ballots** (including ballots from all council districts)
- **294,966 undervotes** (90% incorrectly marked as undervotes)
- **Massive inefficiency** (reading and processing irrelevant ballots)

## Root Cause Analysis

### File Structure Discovery
The NYC election data is already organized by **precinct** in the filename pattern:
- `2025P1V1_ABS.xlsx` - Precinct 1, Vote Type 1, Location ABS
- `2025P2V1_AFF.xlsx` - Precinct 2, Vote Type 1, Location AFF
- `2025P3V1_ELE1.xlsx` - Precinct 3, Vote Type 1, Location ELE1
- etc.

### Council District Distribution by Precinct
Analysis shows council districts are distributed across precincts:
- **Precinct 1**: Council Districts 1, 2, 3, 4, 5, 7, 8, 10
- **Precinct 2**: Council Districts 8, 11, 12, 13, 14, 16, 17
- **Precinct 3**: Council Districts 33, 35, 36, 38, 39, 41, 46, 47, 48
- **Precinct 4**: Council Districts 19, 21, 25, 28, 30

### The Real Issue
The 2nd Council District appears in **Precinct 1** files, but the current code:
1. Reads ALL precinct files (P1, P2, P3, P4, P5)
2. Processes ALL ballots from ALL precincts
3. Then filters out ballots that don't have choices for the 2nd Council District
4. This results in ~90% of ballots being marked as "undervotes"

## Proposed Solution

### 1. Precinct-Based File Filtering

**Current Approach (Wrong):**
```rust
// Reads ALL files matching pattern
let file_rx = Regex::new(&format!("^{}$", options.cvr_pattern)).unwrap();
// cvrPattern: "2025P1V.+\\.xlsx" - matches ALL precincts!
```

**Proposed Approach (Correct):**
```rust
// Extract precinct from council district mapping
let precinct_for_district = get_precinct_for_council_district(&options.jurisdiction_name);
let file_rx = Regex::new(&format!("^2025P{}V.+\\.xlsx$", precinct_for_district)).unwrap();
```

### 2. Council District to Precinct Mapping

Create a mapping function:
```rust
fn get_precinct_for_council_district(district_name: &str) -> u32 {
    let district_num: u32 = extract_district_number(district_name);
    
    match district_num {
        1..=10 => 1,    // Districts 1-10 are in Precinct 1
        11..=17 => 2,   // Districts 11-17 are in Precinct 2  
        19..=30 => 4,   // Districts 19-30 are in Precinct 4
        33..=48 => 3,   // Districts 33-48 are in Precinct 3
        _ => panic!("Unknown council district: {}", district_num)
    }
}
```

### 3. Dynamic Precinct Discovery

**Alternative Approach:** Instead of hardcoding mappings, dynamically discover which precinct contains the target council district:

```rust
fn find_precinct_for_council_district(path: &Path, target_district: &str) -> Option<u32> {
    for precinct in 1..=5 {
        let sample_file = format!("2025P{}V1_ABS.xlsx", precinct);
        if let Ok(df) = read_sample_file(&path.join(sample_file)) {
            if contains_council_district(&df, target_district) {
                return Some(precinct);
            }
        }
    }
    None
}
```

### 4. Implementation Steps

#### Step 1: Update ReaderOptions
```rust
struct ReaderOptions {
    office_name: String,
    jurisdiction_name: String,
    candidates_file: String,
    cvr_pattern: String,
    target_precinct: Option<u32>, // New field
}
```

#### Step 2: Modify File Pattern Matching
```rust
// In nyc_ballot_reader function
let target_precinct = find_precinct_for_council_district(path, &options.jurisdiction_name)
    .expect("Could not find precinct for council district");

let file_rx = Regex::new(&format!("^2025P{}V.+\\.xlsx$", target_precinct)).unwrap();
```

#### Step 3: Update Metadata Structure
```json
{
  "loaderParams": {
    "candidatesFile": "Primary Election 2025 - 06-24-2025_CandidacyID_To_Name.xlsx",
    "cvrPattern": "2025P1V.+\\.xlsx",
    "jurisdictionName": "2nd Council District", 
    "officeName": "DEM Council Member",
    "targetPrecinct": 1  // New field
  }
}
```

#### Step 4: Remove Ballot-Level Filtering
Remove the current ballot filtering logic since we'll only read relevant files:
```rust
// REMOVE THIS CODE:
// Check if this ballot has any valid choices for this district
// let mut has_valid_choice = false;
// for col in rank_to_col.values() { ... }
// if !has_valid_choice { continue; }
```

### 5. Benefits of This Approach

1. **Massive Performance Improvement**: Only read files from the relevant precinct
2. **Accurate Results**: No false undervotes from other districts
3. **Scalable**: Works for any council district without hardcoding
4. **Efficient**: Reduces data processing by ~90%
5. **Maintainable**: Clear separation of concerns

### 6. Expected Results

For 2nd Council District analysis:
- **Before**: Read 25+ files, process 325K ballots, 90% undervotes
- **After**: Read ~8 files (Precinct 1 only), process ~30K ballots, 0% false undervotes

### 7. Testing Strategy

1. **Unit Tests**: Test precinct discovery for each council district
2. **Integration Tests**: Verify ballot counts match expected precinct totals
3. **Regression Tests**: Ensure existing functionality still works
4. **Performance Tests**: Measure processing time improvement

### 8. Migration Plan

1. **Phase 1**: Implement precinct discovery logic
2. **Phase 2**: Update file pattern matching
3. **Phase 3**: Remove ballot-level filtering
4. **Phase 4**: Update metadata files
5. **Phase 5**: Regenerate all NYC reports

## Conclusion

The current approach is fundamentally flawed because it processes data at the wrong level. By filtering at the **file level** (precinct) rather than the **ballot level**, we can achieve:
- 99%+ reduction in processing time
- Elimination of false undervotes
- Accurate RCV analysis per council district
- Scalable solution for all NYC elections

This fix transforms the NYC format handler from an inefficient "read everything then filter" approach to an efficient "read only what's needed" approach.
