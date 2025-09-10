use super::{CandidateInfo, ContestInfo, ContestReport, ReportResult, ResultSummary, RoundResult};
use crate::database::{BallotsDatabase, ContestInfo as DbContestInfo};
use crate::model::election::CandidateType;
use std::collections::{HashMap, HashSet};

/// Generate a complete contest report with RCV tabulation
pub async fn generate_contest_report(
    ballots_db: &BallotsDatabase,
    contest: &DbContestInfo,
) -> ReportResult<ContestReport> {
    // Get contest metadata
    let election = ballots_db.get_election_by_id(contest.election_id).await?;
    let jurisdiction = ballots_db
        .get_jurisdiction_by_id(election.jurisdiction_id)
        .await?;

    // Get candidates
    let candidates = ballots_db.get_candidates_for_contest(contest.id).await?;
    let candidate_map: HashMap<i64, String> =
        candidates.iter().map(|c| (c.id, c.name.clone())).collect();

    // Get all ballots for this contest
    let ballots = ballots_db.get_ballots_for_contest(contest.id).await?;

    println!(
        "    📊 Tabulating {} ballots for {}",
        ballots.len(),
        contest.office_name
    );

    // Convert to internal ballot format for tabulation
    let mut rcv_ballots = Vec::new();
    for ballot in ballots {
        let choices = ballots_db.get_choices_for_ballot(ballot.id).await?;

        // Sort choices by rank and convert to candidate names
        let mut ranked_choices: Vec<_> = choices
            .into_iter()
            .filter_map(|choice| {
                if choice.choice_type == "candidate" {
                    candidate_map
                        .get(&choice.candidate_id?)
                        .map(|name| (choice.rank_position, name.clone()))
                } else {
                    None // Skip non-candidate choices for now
                }
            })
            .collect();

        ranked_choices.sort_by_key(|(rank, _)| *rank);
        let ballot_ranking: Vec<String> =
            ranked_choices.into_iter().map(|(_, name)| name).collect();

        if !ballot_ranking.is_empty() {
            rcv_ballots.push(ballot_ranking);
        }
    }

    // Perform RCV tabulation
    let candidate_names: Vec<String> = candidate_map.values().cloned().collect();
    let tabulation_results = tabulate_rcv(&rcv_ballots, &candidate_names);

    // Build contest info
    let info = ContestInfo {
        name: contest.office_name.clone(),
        date: election.date.format("%Y-%m-%d").to_string(),
        data_format: "us_ny_nyc".to_string(), // TODO: Get from database
        jurisdiction_path: jurisdiction.path.clone(),
        election_path: election.election_path.clone(),
        office: contest.office.clone(),
        office_name: contest.office_name.clone(),
        jurisdiction_name: jurisdiction.name.clone(),
        election_name: election.name.clone(),
    };

    // Build candidate info
    let candidate_info: Vec<CandidateInfo> = candidates
        .into_iter()
        .map(|c| CandidateInfo {
            name: c.name,
            candidate_type: match c.candidate_type {
                CandidateType::Regular => "Regular".to_string(),
                CandidateType::WriteIn => "WriteIn".to_string(),
                CandidateType::QualifiedWriteIn => "QualifiedWriteIn".to_string(),
            },
        })
        .collect();

    // Build summary
    let summary = ResultSummary {
        winner: tabulation_results.winner.clone(),
        total_rounds: tabulation_results.rounds.len() as i64,
        total_ballots: rcv_ballots.len() as i64,
    };

    Ok(ContestReport {
        info,
        ballot_count: rcv_ballots.len() as i64,
        candidates: candidate_info,
        results: tabulation_results.rounds,
        summary,
    })
}

/// RCV tabulation results
#[derive(Debug)]
struct TabulationResults {
    rounds: Vec<RoundResult>,
    winner: Option<String>,
}

/// Perform instant runoff voting tabulation
fn tabulate_rcv(ballots: &[Vec<String>], all_candidates: &[String]) -> TabulationResults {
    let mut active_candidates: HashSet<String> = all_candidates.iter().cloned().collect();
    let mut rounds = Vec::new();
    let mut round_number = 1;

    loop {
        // Count first-choice votes for active candidates
        let mut vote_counts: HashMap<String, i64> = HashMap::new();

        for ballot in ballots {
            // Find first active candidate in this ballot
            for candidate in ballot {
                if active_candidates.contains(candidate) {
                    *vote_counts.entry(candidate.clone()).or_insert(0) += 1;
                    break;
                }
            }
        }

        // Ensure all active candidates have an entry (even with 0 votes)
        for candidate in &active_candidates {
            vote_counts.entry(candidate.clone()).or_insert(0);
        }

        let total_votes: i64 = vote_counts.values().sum();
        let majority_threshold = total_votes / 2 + 1;

        // Find candidate(s) with most votes
        let max_votes = vote_counts.values().max().copied().unwrap_or(0);
        let leaders: Vec<_> = vote_counts
            .iter()
            .filter(|(_, &votes)| votes == max_votes)
            .map(|(name, _)| name.clone())
            .collect();

        // Check for winner (majority or only one candidate left)
        let winner = if max_votes >= majority_threshold || active_candidates.len() <= 1 {
            leaders.first().cloned()
        } else {
            None
        };

        // Find candidates to eliminate (those with fewest votes)
        let min_votes = vote_counts.values().min().copied().unwrap_or(0);
        let to_eliminate: Vec<_> = vote_counts
            .iter()
            .filter(|(_, &votes)| votes == min_votes)
            .map(|(name, _)| name.clone())
            .collect();

        // Record this round
        let eliminated = if winner.is_some() {
            Vec::new() // No eliminations in final round
        } else {
            to_eliminate.clone()
        };

        rounds.push(RoundResult {
            round: round_number,
            tally: vote_counts,
            eliminated: eliminated.clone(),
        });

        // Check for completion
        if winner.is_some() {
            return TabulationResults { rounds, winner };
        }

        // Eliminate candidates and continue
        for candidate in &eliminated {
            active_candidates.remove(candidate);
        }

        // Safety check to prevent infinite loops
        if active_candidates.is_empty() {
            break;
        }

        round_number += 1;

        // Limit rounds to prevent runaway computation
        if round_number > 50 {
            eprintln!("⚠️  Warning: Tabulation exceeded 50 rounds, stopping");
            break;
        }
    }

    TabulationResults {
        rounds,
        winner: None,
    }
}
