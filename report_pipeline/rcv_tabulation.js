#!/usr/bin/env node

import Database from 'better-sqlite3';

// Connect to ballots database
const ballotsDb = new Database('ballots.db');
const reportsDb = new Database('reports.db');

console.log('🗳️  Starting RCV Tabulation...');

/**
 * Perform instant runoff voting tabulation
 * @param {Array} ballots - Array of ballot objects with ranked choices
 * @param {Array} candidates - Array of candidate names
 * @returns {Object} - Tabulation results with rounds, winner, etc.
 */
function tabulateRCV(ballots, candidates) {
    let activeCandidates = new Set(candidates);
    let rounds = [];
    let roundNumber = 1;
    
    console.log(`    📊 Starting tabulation with ${ballots.length} ballots and ${candidates.length} candidates`);
    
    while (activeCandidates.size > 1) {
        // Count votes for each active candidate
        let voteCounts = {};
        let exhaustedCount = 0;
        
        // Initialize vote counts
        for (let candidate of activeCandidates) {
            voteCounts[candidate] = 0;
        }
        
        // Count first-choice votes among active candidates
        for (let ballot of ballots) {
            let voted = false;
            for (let choice of ballot.choices) {
                if (activeCandidates.has(choice)) {
                    voteCounts[choice]++;
                    voted = true;
                    break;
                }
            }
            if (!voted) {
                exhaustedCount++;
            }
        }
        
        const totalVotes = Object.values(voteCounts).reduce((sum, count) => sum + count, 0);
        const majorityThreshold = Math.floor(totalVotes / 2) + 1;
        
        console.log(`    Round ${roundNumber}: ${totalVotes} active votes, ${exhaustedCount} exhausted`);
        
        // Find candidate with most votes
        const maxVotes = Math.max(...Object.values(voteCounts));
        const winner = Object.keys(voteCounts).find(candidate => voteCounts[candidate] === maxVotes);
        
        // Check if we have a winner (majority or only one candidate left)
        const hasWinner = maxVotes >= majorityThreshold || activeCandidates.size <= 1;
        
        // Find candidates to eliminate (those with fewest votes)
        const minVotes = Math.min(...Object.values(voteCounts));
        const toEliminate = Object.keys(voteCounts).filter(candidate => voteCounts[candidate] === minVotes);
        
        // Convert tally to allocations array format expected by Sankey component
        const allocations = Object.entries(voteCounts).map(([candidate, votes]) => ({
            allocatee: candidate,
            votes: votes
        }));
        
        // Add exhausted votes if any
        if (exhaustedCount > 0) {
            allocations.push({
                allocatee: "X", // X represents exhausted votes
                votes: exhaustedCount
            });
        }
        
        // Record this round
        rounds.push({
            round: roundNumber,
            tally: { ...voteCounts },
            allocations: allocations,
            eliminated: hasWinner ? [] : toEliminate,
            exhausted: exhaustedCount,
            winner: hasWinner ? winner : null
        });
        
        console.log(`      Vote counts:`, Object.entries(voteCounts).map(([name, votes]) => `${name}: ${votes}`).join(', '));
        
        if (hasWinner) {
            console.log(`    🏆 Winner: ${winner} with ${maxVotes} votes (${((maxVotes / totalVotes) * 100).toFixed(1)}%)`);
            return {
                rounds,
                winner,
                totalRounds: roundNumber,
                totalBallots: ballots.length,
                finalTally: voteCounts
            };
        }
        
        // Eliminate candidates with fewest votes
        for (let candidate of toEliminate) {
            activeCandidates.delete(candidate);
            console.log(`      Eliminated: ${candidate} (${minVotes} votes)`);
        }
        
        roundNumber++;
    }
    
    // Fallback if we exit the loop (shouldn't happen)
    const remainingCandidate = Array.from(activeCandidates)[0];
    return {
        rounds,
        winner: remainingCandidate,
        totalRounds: roundNumber - 1,
        totalBallots: ballots.length,
        finalTally: { [remainingCandidate]: ballots.length }
    };
}

/**
 * Get ballots for a contest from the database
 */
function getBallotsForContest(contestId) {
    const ballotQuery = ballotsDb.prepare(`
        SELECT b.id, b.ballot_id
        FROM ballots b
        WHERE b.contest_id = ?
    `);
    
    const choiceQuery = ballotsDb.prepare(`
        SELECT bc.rank_position, c.name as candidate_name
        FROM ballot_choices bc
        JOIN candidates c ON bc.candidate_id = c.id
        WHERE bc.ballot_id = ? AND bc.choice_type = 'candidate'
        ORDER BY bc.rank_position
    `);
    
    const ballotRows = ballotQuery.all(contestId);
    const ballots = [];
    
    for (let ballotRow of ballotRows) {
        const choices = choiceQuery.all(ballotRow.id);
        if (choices.length > 0) {
            ballots.push({
                id: ballotRow.ballot_id,
                choices: choices.map(choice => choice.candidate_name)
            });
        }
    }
    
    return ballots;
}

/**
 * Generate candidate vote data in the format expected by the frontend
 */
function generateCandidateVoteData(rounds, candidates) {
    const candidateVotes = [];
    
    for (let candidateName of candidates) {
        // Find first round votes
        const firstRoundVotes = rounds[0]?.tally[candidateName] || 0;
        
        // Calculate transfer votes (final round - first round)
        const finalRound = rounds[rounds.length - 1];
        const finalVotes = finalRound?.tally[candidateName] || 0;
        const transferVotes = Math.max(0, finalVotes - firstRoundVotes);
        
        // Find elimination round (if any)
        let roundEliminated = null;
        for (let i = 0; i < rounds.length; i++) {
            if (rounds[i].eliminated.includes(candidateName)) {
                roundEliminated = i + 1;
                break;
            }
        }
        
        candidateVotes.push({
            candidate: candidateName,
            name: candidateName,
            firstRoundVotes,
            transferVotes,
            votes: firstRoundVotes, // For compatibility with simple format
            roundEliminated,
            winner: candidateName === finalRound?.winner
        });
    }
    
    // Sort by total votes (descending)
    candidateVotes.sort((a, b) => (b.firstRoundVotes + b.transferVotes) - (a.firstRoundVotes + a.transferVotes));
    
    return candidateVotes;
}

// Main execution
async function main() {
    try {
        // Get contest info
        const contestInfo = ballotsDb.prepare(`
            SELECT c.id, c.office_id, c.office_name, c.jurisdiction_name,
                   e.path as election_path, e.name as election_name, e.date,
                   j.name as jurisdiction_name_full, j.path as jurisdiction_path
            FROM contests c
            JOIN elections e ON c.election_id = e.id
            JOIN jurisdictions j ON e.jurisdiction_id = j.id
            WHERE e.path = '2025/07'
        `).get();
        
        if (!contestInfo) {
            throw new Error('No contest found for election 2025/07');
        }
        
        console.log('📊 Contest:', contestInfo.office_name);
        
        // Get candidates
        const candidates = ballotsDb.prepare(`
            SELECT name FROM candidates WHERE contest_id = ? ORDER BY name
        `).all(contestInfo.id);
        
        const candidateNames = candidates.map(c => c.name);
        console.log('👥 Candidates:', candidateNames.join(', '));
        
        // Get ballots
        const ballots = getBallotsForContest(contestInfo.id);
        console.log(`🗳️  Loaded ${ballots.length} ballots`);
        
        // Perform RCV tabulation
        const results = tabulateRCV(ballots, candidateNames);
        
        // Generate candidate vote data for frontend
        const candidateVotes = generateCandidateVoteData(results.rounds, candidateNames);
        
        // Create comprehensive report data
        const reportData = {
            info: {
                name: contestInfo.office_name,
                date: contestInfo.date,
                dataFormat: 'us_ny_nyc',
                jurisdictionPath: contestInfo.jurisdiction_path,
                electionPath: contestInfo.election_path,
                office: contestInfo.office_id,
                officeName: contestInfo.office_name,
                jurisdictionName: contestInfo.jurisdiction_name,
                electionName: contestInfo.election_name
            },
            ballotCount: ballots.length,
            numCandidates: candidateNames.length,
            winner: results.winner,
            condorcet: results.winner, // Simplified - would need proper Condorcet analysis
            candidates: candidateVotes.reduce((acc, cv) => {
                acc[cv.name] = { name: cv.name, writeIn: false };
                return acc;
            }, {}),
            totalVotes: candidateVotes, // This is what VoteCounts component expects
            rounds: results.rounds.map(round => ({
                round: round.round,
                tally: round.tally,
                eliminated: round.eliminated,
                transfers: [] // TODO: Calculate actual transfers
            })),
            summary: {
                winner: results.winner,
                totalRounds: results.totalRounds,
                totalBallots: results.totalBallots
            }
        };
        
        console.log('\n📋 Final Results:');
        console.log(`🏆 Winner: ${results.winner}`);
        console.log(`📊 Total Rounds: ${results.totalRounds}`);
        console.log(`🗳️  Total Ballots: ${results.totalBallots}`);
        
        // Update reports database
        const contestPath = `${contestInfo.jurisdiction_path}/${contestInfo.election_path}/${contestInfo.office_id}`;
        
        // Clear and recreate reports (disable foreign keys temporarily)
        reportsDb.exec('PRAGMA foreign_keys = OFF');
        reportsDb.exec('DELETE FROM contest_reports');
        reportsDb.exec('DELETE FROM election_index');
        reportsDb.exec('PRAGMA foreign_keys = ON');
        
        // Insert election index
        const insertElection = reportsDb.prepare(`
            INSERT INTO election_index (path, jurisdiction_name, election_name, date)
            VALUES (?, ?, ?, ?)
        `);
        
        insertElection.run(
            contestInfo.election_path,
            contestInfo.jurisdiction_name_full,
            contestInfo.election_name,
            contestInfo.date
        );
        
        // Insert contest report
        const insertReport = reportsDb.prepare(`
            INSERT INTO contest_reports (path, election_path, office, report_json, ballot_count, winner)
            VALUES (?, ?, ?, ?, ?, ?)
        `);
        
        insertReport.run(
            contestPath,
            contestInfo.election_path,
            contestInfo.office_id,
            JSON.stringify(reportData),
            ballots.length,
            results.winner
        );
        
        console.log(`\n✅ Report generated successfully!`);
        console.log(`📁 Contest path: ${contestPath}`);
        
    } catch (error) {
        console.error('❌ Error:', error.message);
        process.exit(1);
    } finally {
        ballotsDb.close();
        reportsDb.close();
    }
}

main();
