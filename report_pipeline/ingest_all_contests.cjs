#!/usr/bin/env node
/**
 * NYC All Contests Ingestion Script
 * 
 * This script bypasses Rust compilation issues by directly calling
 * the existing binary for each discovered contest individually.
 */

const { spawn } = require('child_process');
const fs = require('fs');
const path = require('path');

async function runCommand(command, args, options = {}) {
    return new Promise((resolve, reject) => {
        console.log(`🚀 Running: ${command} ${args.join(' ')}`);
        
        const child = spawn(command, args, {
            stdio: 'inherit',
            ...options
        });
        
        child.on('close', (code) => {
            if (code === 0) {
                resolve();
            } else {
                reject(new Error(`Command failed with exit code ${code}`));
            }
        });
        
        child.on('error', reject);
    });
}

async function discoverContests() {
    console.log('🔍 Discovering all NYC contests...');
    
    return new Promise((resolve, reject) => {
        const child = spawn('python3', ['discover_contests.py', 'raw-data/us/ny/nyc/2025/07'], {
            stdio: ['inherit', 'pipe', 'inherit']
        });
        
        let stdout = '';
        child.stdout.on('data', (data) => {
            stdout += data.toString();
        });
        
        child.on('close', (code) => {
            if (code === 0) {
                try {
                    const result = JSON.parse(stdout);
                    resolve(result.contests);
                } catch (e) {
                    reject(new Error(`Failed to parse JSON: ${e.message}`));
                }
            } else {
                reject(new Error(`Discovery failed with exit code ${code}`));
            }
        });
        
        child.on('error', reject);
    });
}

async function ingestSingleContest(contest, contestIndex, totalContests) {
    console.log(`\n📊 [${contestIndex + 1}/${totalContests}] Processing: ${contest.office_name}`);
    console.log(`   Office ID: ${contest.office_id}`);
    console.log(`   Jurisdiction: ${contest.jurisdiction_name}`);
    console.log(`   P Group: ${contest.p_group}`);
    
    // Create a temporary metadata file for this contest
    const tempMetaDir = path.join(__dirname, 'temp_meta');
    const jurisdictionDir = path.join(tempMetaDir, 'us', 'ny', 'nyc');
    
    // Ensure directories exist
    fs.mkdirSync(jurisdictionDir, { recursive: true });
    
    // Create metadata JSON for this specific contest
    const metadata = {
        "name": "New York City",
        "path": "us/ny/nyc",
        "kind": "city",
        "offices": {
            [contest.office_id]: {
                "name": contest.office_name
            }
        },
        "elections": {
            "2025/07": {
                "name": "Primary Election",
                "date": "2025-06-24",
                "dataFormat": "us_ny_nyc",
                "tabulationOptions": null,
                "normalization": "simple",
                "contests": [{
                    "office": contest.office_id,
                    "loaderParams": contest.loader_params
                }],
                "files": {
                    // Placeholder - sync command will fill these
                    "placeholder.xlsx": "placeholder"
                }
            }
        }
    };
    
    const metaFile = path.join(jurisdictionDir, 'nyc.json');
    fs.writeFileSync(metaFile, JSON.stringify(metadata, null, 2));
    
    try {
        // Run the existing report command for this single contest
        await runCommand('./target/release/ranked-vote', [
            'report',
            tempMetaDir,
            'raw-data',
            'preprocessed',
            'reports',
            'true',  // force_preprocess
            'true',  // force_report
            '--jurisdiction', 'us/ny/nyc',
            '--election', '2025/07',
            '--contest', contest.office_id
        ]);
        
        console.log(`   ✅ Successfully processed ${contest.office_name}`);
        
    } catch (error) {
        console.error(`   ❌ Failed to process ${contest.office_name}: ${error.message}`);
        throw error;
    } finally {
        // Clean up temp metadata
        if (fs.existsSync(metaFile)) {
            fs.unlinkSync(metaFile);
        }
    }
}

async function main() {
    try {
        console.log('🎯 NYC All Contests Ingestion');
        console.log('==============================\n');
        
        // Step 1: Discover all contests
        const contests = await discoverContests();
        console.log(`✅ Discovered ${contests.length} contests\n`);
        
        // Step 2: Process each contest individually
        console.log('📋 Contest Summary:');
        const contestTypes = {};
        contests.forEach(contest => {
            const type = contest.office_name.split(' - ')[0];
            contestTypes[type] = (contestTypes[type] || 0) + 1;
        });
        
        for (const [type, count] of Object.entries(contestTypes)) {
            console.log(`   ${type}: ${count}`);
        }
        console.log('');
        
        // Step 3: Process contests one by one
        const startTime = Date.now();
        let successCount = 0;
        let failureCount = 0;
        
        for (let i = 0; i < contests.length; i++) {
            try {
                await ingestSingleContest(contests[i], i, contests.length);
                successCount++;
            } catch (error) {
                console.error(`❌ Contest ${i + 1} failed: ${error.message}`);
                failureCount++;
                
                // Continue with next contest instead of stopping
                continue;
            }
        }
        
        // Step 4: Summary
        const duration = (Date.now() - startTime) / 1000;
        console.log('\n🎉 Ingestion Complete!');
        console.log('======================');
        console.log(`✅ Successful: ${successCount}`);
        console.log(`❌ Failed: ${failureCount}`);
        console.log(`⏱️  Duration: ${duration.toFixed(1)} seconds`);
        console.log(`📊 Success Rate: ${((successCount / contests.length) * 100).toFixed(1)}%`);
        
        if (failureCount > 0) {
            console.log(`\n⚠️  ${failureCount} contests failed. Check logs above for details.`);
            process.exit(1);
        }
        
    } catch (error) {
        console.error(`❌ Fatal error: ${error.message}`);
        process.exit(1);
    }
}

// Clean up temp directory on exit
process.on('exit', () => {
    const tempDir = path.join(__dirname, 'temp_meta');
    if (fs.existsSync(tempDir)) {
        fs.rmSync(tempDir, { recursive: true, force: true });
    }
});

if (require.main === module) {
    main();
}
