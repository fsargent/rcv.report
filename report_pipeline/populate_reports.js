#!/usr/bin/env node

const Database = require('better-sqlite3');
const path = require('path');

// Connect to both databases
const ballotsDb = new Database('ballots.db');
const reportsDb = new Database('reports.db');

console.log('🚀 Populating reports database from ballots database...');

// Get election and contest info from ballots database
const electionInfo = ballotsDb.prepare(`
    SELECT e.path, e.name, e.date, j.name as jurisdiction_name
    FROM elections e
    JOIN jurisdictions j ON e.jurisdiction_id = j.id
    WHERE e.path = '2025/07'
`).get();

const contestInfo = ballotsDb.prepare(`
    SELECT c.office_id, c.office_name, c.jurisdiction_name
    FROM contests c
    JOIN elections e ON c.election_id = e.id
    WHERE e.path = '2025/07'
`).get();

console.log('📊 Election:', electionInfo);
console.log('📊 Contest:', contestInfo);

// Insert into election_index
const insertElection = reportsDb.prepare(`
    INSERT OR REPLACE INTO election_index (path, jurisdiction_name, election_name, date)
    VALUES (?, ?, ?, ?)
`);

insertElection.run(
    electionInfo.path,
    electionInfo.jurisdiction_name,
    electionInfo.name,
    electionInfo.date
);

console.log('✅ Inserted election into election_index');

// Get candidate vote counts for first choice
const firstChoiceVotes = ballotsDb.prepare(`
    SELECT c.name, COUNT(*) as votes
    FROM ballot_choices bc
    JOIN candidates c ON bc.candidate_id = c.id
    WHERE bc.choice_type = 'candidate' AND bc.rank_position = 1
    GROUP BY c.name
    ORDER BY votes DESC
`).all();

console.log('📊 First choice votes:', firstChoiceVotes);

// Find winner (candidate with most first-choice votes)
const winner = firstChoiceVotes[0].name;
const totalCandidates = firstChoiceVotes.length;
const totalBallots = firstChoiceVotes.reduce((sum, candidate) => sum + candidate.votes, 0);

console.log('🏆 Winner:', winner);
console.log('📊 Total candidates:', totalCandidates);
console.log('📊 Total ballots:', totalBallots);

// Create contest summary
const contestPath = `us/ny/nyc/${electionInfo.path}/${contestInfo.office_id}`;

const insertContestSummary = reportsDb.prepare(`
    INSERT OR REPLACE INTO contest_summaries 
    (election_path, office, office_name, name, winner, num_candidates, num_rounds, ballot_count)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?)
`);

insertContestSummary.run(
    electionInfo.path,
    contestInfo.office_id,
    contestInfo.office_name,
    contestInfo.office_name,
    winner,
    totalCandidates,
    1, // For now, assume single round (we'd need to implement RCV tabulation for multiple rounds)
    totalBallots
);

console.log('✅ Inserted contest summary');

// Create a simple report JSON
const reportData = {
    info: {
        name: contestInfo.office_name,
        date: electionInfo.date,
        dataFormat: 'us_ny_nyc',
        jurisdictionPath: 'us/ny/nyc',
        electionPath: electionInfo.path,
        office: contestInfo.office_id,
        officeName: contestInfo.office_name,
        jurisdictionName: contestInfo.jurisdiction_name,
        electionName: electionInfo.name
    },
    ballotCount: totalBallots,
    candidates: firstChoiceVotes.map(candidate => ({
        name: candidate.name,
        votes: candidate.votes,
        winner: candidate.name === winner
    })),
    results: [{
        round: 1,
        tally: firstChoiceVotes.reduce((acc, candidate) => {
            acc[candidate.name] = candidate.votes;
            return acc;
        }, {}),
        eliminated: []
    }],
    summary: {
        winner: winner,
        totalRounds: 1,
        totalBallots: totalBallots
    }
};

const insertContestReport = reportsDb.prepare(`
    INSERT OR REPLACE INTO contest_reports 
    (path, election_path, office, report_json, ballot_count, winner)
    VALUES (?, ?, ?, ?, ?, ?)
`);

insertContestReport.run(
    contestPath,
    electionInfo.path,
    contestInfo.office_id,
    JSON.stringify(reportData),
    totalBallots,
    winner
);

console.log('✅ Inserted contest report');

// Close databases
ballotsDb.close();
reportsDb.close();

console.log('🎉 Reports database populated successfully!');
console.log(`📊 Contest path: ${contestPath}`);
console.log(`🏆 Winner: ${winner}`);
console.log(`📊 Total ballots: ${totalBallots}`);
