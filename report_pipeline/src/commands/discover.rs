use crate::util::write_serialized;
use calamine::{open_workbook, Reader, Xlsx};
use colored::Colorize;
use serde_json::{json, Map, Value};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

pub fn discover(raw_data_dir: &Path, meta_dir: &Path, jurisdiction: &str, election: &str) {
    println!(
        "🔍 Discovering contests for {} {}",
        jurisdiction.cyan(),
        election.cyan()
    );

    // Build the path to the raw data
    let raw_path = raw_data_dir.join(jurisdiction).join(election);

    if !raw_path.exists() {
        eprintln!("❌ Raw data path does not exist: {}", raw_path.display());
        return;
    }

    // For NYC format, discover contests from CVR files
    if jurisdiction == "us/ny/nyc" {
        discover_nyc_contests(&raw_path, meta_dir, jurisdiction, election);
    } else {
        eprintln!(
            "❌ Discovery not yet implemented for jurisdiction: {}",
            jurisdiction
        );
    }
}

fn discover_nyc_contests(raw_path: &Path, meta_dir: &Path, jurisdiction: &str, election: &str) {
    println!("📋 Analyzing NYC CVR files...");

    // Find all P group files (P1, P2, P3, P4, P5)
    let mut p_groups = Vec::new();

    for entry in fs::read_dir(raw_path).expect("Failed to read raw data directory") {
        let entry = entry.expect("Failed to read directory entry");
        let filename = entry.file_name().to_string_lossy().to_string();

        // Look for files like 2025P1V1_ELE1.xlsx, 2025P2V1_ELE1.xlsx, etc.
        if filename.contains("V1_ELE1.xlsx") && filename.contains("2025P") {
            if let Some(p_num) = extract_p_number(&filename) {
                p_groups.push((p_num, filename));
            }
        }
    }

    p_groups.sort_by_key(|(p_num, _)| *p_num);
    println!(
        "📁 Found {} P groups: {:?}",
        p_groups.len(),
        p_groups.iter().map(|(p, _)| p).collect::<Vec<_>>()
    );

    // Find candidate mapping file
    let candidate_file = find_candidate_file(raw_path);
    if candidate_file.is_none() {
        eprintln!("❌ Could not find candidate mapping file");
        return;
    }
    let candidate_file = candidate_file.unwrap();
    println!("👥 Found candidate file: {}", candidate_file);

    // Analyze each P group to extract contests
    let mut all_contests = Vec::new();
    let mut offices = Map::new();

    for (p_num, filename) in p_groups {
        println!("🔍 Analyzing P{} group: {}", p_num, filename);

        let file_path = raw_path.join(&filename);
        let contests = analyze_p_group(&file_path, p_num, &candidate_file);

        for contest in contests {
            println!(
                "  📊 Found contest: {} ({})",
                contest.office_name.green(),
                contest.office_id
            );

            // Add to offices map
            offices.insert(
                contest.office_id.clone(),
                json!({
                    "name": contest.office_name
                }),
            );

            all_contests.push(contest);
        }
    }

    // Generate file hashes for all files in the directory
    let mut files = Map::new();
    for entry in fs::read_dir(raw_path).expect("Failed to read raw data directory") {
        let entry = entry.expect("Failed to read directory entry");
        let filename = entry.file_name().to_string_lossy().to_string();

        if filename.ends_with(".xlsx") {
            // For now, use placeholder hash - sync command will fill these in
            files.insert(filename, Value::String("placeholder".to_string()));
        }
    }

    // Generate metadata JSON
    let metadata = json!({
        "name": get_jurisdiction_name(jurisdiction),
        "path": jurisdiction,
        "kind": get_jurisdiction_kind(jurisdiction),
        "offices": offices,
        "elections": {
            election: {
                "name": "Primary Election",
                "date": "2025-06-24", // TODO: extract from data
                "dataFormat": "us_ny_nyc",
                "tabulationOptions": null,
                "normalization": "simple",
                "contests": all_contests.clone().into_iter().map(|c| json!({
                    "office": c.office_id,
                    "loaderParams": {
                        "candidatesFile": candidate_file,
                        "cvrPattern": format!("2025P{}V.+\\.xlsx", c.p_group),
                        "jurisdictionName": c.jurisdiction_name,
                        "officeName": c.office_name_pattern
                    }
                })).collect::<Vec<_>>(),
                "files": files
            }
        }
    });

    // Write metadata file
    let meta_path = meta_dir.join(jurisdiction);
    fs::create_dir_all(&meta_path).expect("Failed to create metadata directory");

    let meta_file = meta_path.join("nyc.json");
    write_serialized(&meta_file, &metadata);

    println!(
        "✅ Generated metadata with {} contests: {}",
        all_contests.len(),
        meta_file.display()
    );
}

#[derive(Debug, Clone)]
struct Contest {
    office_id: String,
    office_name: String,
    office_name_pattern: String,
    jurisdiction_name: String,
    p_group: u32,
}

fn extract_p_number(filename: &str) -> Option<u32> {
    // Extract P number from filename like "2025P1V1_ELE1.xlsx"
    if let Some(start) = filename.find("2025P") {
        let p_part = &filename[start + 5..];
        if let Some(end) = p_part.find("V") {
            let p_num_str = &p_part[..end];
            return p_num_str.parse().ok();
        }
    }
    None
}

fn find_candidate_file(raw_path: &Path) -> Option<String> {
    for entry in fs::read_dir(raw_path).expect("Failed to read raw data directory") {
        let entry = entry.expect("Failed to read directory entry");
        let filename = entry.file_name().to_string_lossy().to_string();

        if filename.contains("CandidacyID_To_Name") && filename.ends_with(".xlsx") {
            return Some(filename);
        }
    }
    None
}

fn analyze_p_group(file_path: &Path, p_num: u32, candidate_file: &str) -> Vec<Contest> {
    let mut contests = Vec::new();

    // Open the Excel file
    let mut workbook: Xlsx<_> = match open_workbook(file_path) {
        Ok(wb) => wb,
        Err(e) => {
            eprintln!("❌ Failed to open {}: {}", file_path.display(), e);
            return contests;
        }
    };

    let first_sheet = workbook.sheet_names().first().unwrap().clone();
    let sheet = match workbook.worksheet_range(&first_sheet) {
        Some(Ok(sheet)) => sheet,
        Some(Err(e)) => {
            eprintln!("❌ Failed to read sheet in {}: {}", file_path.display(), e);
            return contests;
        }
        None => {
            eprintln!("❌ Empty sheet in {}", file_path.display());
            return contests;
        }
    };

    // Extract unique contests from headers
    let mut seen_contests = HashSet::new();
    let mut rows = sheet.rows();

    if let Some(header_row) = rows.next() {
        for cell in header_row {
            if let Some(header) = cell.get_string() {
                if header.contains("DEM ") && header.contains("Choice 1 of") {
                    // Parse contest info from header like "DEM Borough President Choice 1 of 4 New York (026918)"
                    if let Some(contest) = parse_contest_header(header, p_num) {
                        if seen_contests.insert(contest.office_id.clone()) {
                            contests.push(contest);
                        }
                    }
                }
            }
        }
    }

    contests
}

fn parse_contest_header(header: &str, p_num: u32) -> Option<Contest> {
    // Parse header like "DEM Borough President Choice 1 of 4 New York (026918)"

    // Extract the jurisdiction code in parentheses
    let jurisdiction_code = if let Some(start) = header.rfind('(') {
        if let Some(end) = header.rfind(')') {
            &header[start + 1..end]
        } else {
            return None;
        }
    } else {
        return None;
    };

    // Extract the part before "Choice 1 of"
    let office_part = if let Some(choice_pos) = header.find(" Choice 1 of") {
        &header[..choice_pos]
    } else {
        return None;
    };

    // Extract jurisdiction name (part between last number and opening parenthesis)
    let jurisdiction_name = if let Some(paren_pos) = header.rfind(" (") {
        let before_paren = &header[..paren_pos];
        if let Some(last_space) = before_paren.rfind(' ') {
            &before_paren[last_space + 1..]
        } else {
            "Unknown"
        }
    } else {
        "Unknown"
    };

    // Generate office ID and name
    let office_name = office_part.to_string();
    let office_id = generate_office_id(&office_name, jurisdiction_name, jurisdiction_code);

    Some(Contest {
        office_id,
        office_name: office_name.clone(),
        office_name_pattern: office_part.to_string(),
        jurisdiction_name: jurisdiction_name.to_string(),
        p_group: p_num,
    })
}

fn generate_office_id(
    office_name: &str,
    jurisdiction_name: &str,
    jurisdiction_code: &str,
) -> String {
    // Generate a clean office ID
    let mut id = office_name
        .to_lowercase()
        .replace("dem ", "")
        .replace(" ", "-");

    // Add jurisdiction suffix for non-citywide races
    if jurisdiction_name != "Citywide" {
        id = format!("{}-{}", id, jurisdiction_name.to_lowercase());
    }

    // Add jurisdiction code to make it unique
    format!("{}-{}", id, jurisdiction_code)
}

fn get_jurisdiction_name(jurisdiction: &str) -> &str {
    match jurisdiction {
        "us/ny/nyc" => "New York City",
        _ => "Unknown Jurisdiction",
    }
}

fn get_jurisdiction_kind(jurisdiction: &str) -> &str {
    match jurisdiction {
        "us/ny/nyc" => "city",
        _ => "unknown",
    }
}
