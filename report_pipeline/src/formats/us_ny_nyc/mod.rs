use crate::formats::common::CandidateMap;
use crate::model::election::{Ballot, Candidate, CandidateType, Choice, Election};
use calamine::{open_workbook_auto, Reader, Sheets};
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::{BTreeMap, HashMap};
use std::fs::read_dir;
use std::path::Path;

struct ReaderOptions {
    office_name: String,
    jurisdiction_name: String,
    candidates_file: String,
    cvr_pattern: String,
}

impl ReaderOptions {
    pub fn from_params(params: BTreeMap<String, String>) -> ReaderOptions {
        let office_name: String = params.get("officeName").unwrap().clone();

        let jurisdiction_name: String = params.get("jurisdictionName").unwrap().clone();

        let candidates_file: String = params.get("candidatesFile").unwrap().clone();

        let cvr_pattern: String = params.get("cvrPattern").unwrap().clone();

        ReaderOptions {
            office_name,
            candidates_file,
            jurisdiction_name,
            cvr_pattern,
        }
    }
}

pub fn read_candidate_ids(workbook: &mut Sheets) -> HashMap<u32, String> {
    let mut candidates = HashMap::new();
    let first_sheet = workbook.sheet_names().first().unwrap().clone();
    let sheet = workbook.worksheet_range(&first_sheet).unwrap().unwrap();

    let mut rows = sheet.rows();
    rows.next(); // Skip header row

    for row in rows {
        let id_cell = row.get(0);
        let name_cell = row.get(1);

        if let (Some(id_cell), Some(name_cell)) = (id_cell, name_cell) {
            // Try to parse ID as float first, then as string if that fails
            let id = if let Some(float_val) = id_cell.get_float() {
                float_val as u32
            } else if let Some(string_val) = id_cell.get_string() {
                match string_val.parse::<u32>() {
                    Ok(parsed_id) => parsed_id,
                    Err(_) => continue, // Skip rows with non-numeric IDs
                }
            } else {
                continue; // Skip rows where ID can't be read
            };

            let name = if let Some(name_str) = name_cell.get_string() {
                name_str.to_string()
            } else {
                continue; // Skip rows where name can't be read
            };
            candidates.insert(id, name);
        }
    }

    candidates
}

pub fn nyc_ballot_reader(path: &Path, params: BTreeMap<String, String>) -> Election {
    let options = ReaderOptions::from_params(params);
    let mut ballots: Vec<Ballot> = Vec::new();
    let mut candidate_ids: CandidateMap<u32> = CandidateMap::new();
    let mut candidates_workbook = open_workbook_auto(path.join(options.candidates_file)).unwrap();

    let candidates = read_candidate_ids(&mut candidates_workbook);

    lazy_static! {
        static ref COLUMN_RX: Regex =
            Regex::new(r#"(.+) Choice ([1-5]) of ([1-5]) (.+) \((\d+)\)"#).unwrap();
    }

    let file_rx = Regex::new(&format!("^{}$", options.cvr_pattern)).unwrap();

    for file in read_dir(path).unwrap() {
        if !file_rx.is_match(file.as_ref().unwrap().file_name().to_str().unwrap()) {
            eprintln!("Skipping: {:?}", file);
            continue;
        }

        eprintln!("Reading: {:?}", file);
        let mut workbook = open_workbook_auto(file.unwrap().path()).unwrap();
        let first_sheet = workbook.sheet_names().first().unwrap().clone();
        let sheet = workbook.worksheet_range(&first_sheet).unwrap().unwrap();

        let mut rows = sheet.rows();
        let first_row = rows.next().unwrap();

        let mut rank_to_col: BTreeMap<u32, usize> = BTreeMap::new();
        let mut cvr_id_col: Option<usize> = None;

        for (i, col) in first_row.iter().enumerate() {
            let colname = col.get_string().unwrap();
            if colname == "Cast Vote Record" {
                cvr_id_col = Some(i)
            } else if let Some(caps) = COLUMN_RX.captures(colname) {
                if caps.get(1).unwrap().as_str() != options.office_name {
                    continue;
                }
                if caps.get(4).unwrap().as_str() != options.jurisdiction_name {
                    continue;
                }
                let rank: u32 = caps.get(2).unwrap().as_str().parse().unwrap();
                assert!((1..=5).contains(&rank));
                rank_to_col.insert(rank, i);
            }
        }

        for row in rows {
            let mut votes: Vec<Choice> = Vec::new();
            let ballot_id = row
                .get(cvr_id_col.unwrap())
                .expect("Getting column")
                .get_string()
                .unwrap();
            for col in rank_to_col.values() {
                let value = row.get(*col).unwrap().get_string().unwrap();
                let choice = if value == "undervote" {
                    Choice::Undervote
                } else if value == "overvote" {
                    Choice::Overvote
                } else if value == "Write-in" {
                    candidate_ids.add_id_to_choice(
                        0,
                        Candidate::new("Write-in".to_string(), CandidateType::WriteIn),
                    )
                } else {
                    let ext_id: u32 = value.parse().unwrap();
                    let candidate_name = candidates.get(&ext_id).unwrap();
                    candidate_ids.add_id_to_choice(
                        ext_id,
                        Candidate::new(candidate_name.clone(), CandidateType::Regular),
                    )
                };

                votes.push(choice);
            }

            let ballot = Ballot::new(ballot_id.to_owned(), votes);
            ballots.push(ballot);
        }
    }

    Election::new(candidate_ids.into_vec(), ballots)
}
