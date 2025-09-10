// Advanced tabulation algorithms for RCV analysis
// This module will contain more sophisticated tabulation methods in the future

use std::collections::HashMap;

/// Tabulation method for RCV elections
#[derive(Debug, Clone)]
pub enum TabulationMethod {
    /// Standard instant runoff voting
    InstantRunoff,
    /// Batch elimination (eliminate multiple candidates at once when safe)
    BatchElimination,
    /// Bottom-two runoff
    BottomTwoRunoff,
}

/// Tabulation options
#[derive(Debug, Clone)]
pub struct TabulationOptions {
    pub method: TabulationMethod,
    pub eager_elimination: bool,
    pub exhaust_on_duplicate_rankings: bool,
}

impl Default for TabulationOptions {
    fn default() -> Self {
        Self {
            method: TabulationMethod::InstantRunoff,
            eager_elimination: true,
            exhaust_on_duplicate_rankings: false,
        }
    }
}

/// Vote transfer analysis for understanding voter behavior
#[derive(Debug)]
pub struct VoteTransfer {
    pub from_candidate: String,
    pub to_candidate: Option<String>, // None for exhausted ballots
    pub vote_count: i64,
    pub percentage: f64,
}

/// Detailed round information with transfer analysis
#[derive(Debug)]
pub struct DetailedRound {
    pub round: i64,
    pub vote_counts: HashMap<String, i64>,
    pub eliminated: Vec<String>,
    pub transfers: Vec<VoteTransfer>,
    pub exhausted_ballots: i64,
}

// TODO: Implement advanced tabulation methods
// This will be expanded to support:
// - Batch elimination optimization
// - Vote transfer tracking
// - Exhausted ballot analysis
// - Alternative tabulation methods
