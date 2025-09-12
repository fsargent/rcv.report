#!/usr/bin/env node

const Database = require('better-sqlite3');
const XLSX = require('xlsx');
const yauzl = require('yauzl');
const fs = require('fs').promises;
const fsSync = require('fs');
const path = require('path');
const { program } = require('commander');
const ProgressBar = require('progress');

// Simple color functions
const chalk = {
  blue: (text) => `\x1b[34m${text}\x1b[0m`,
  red: (text) => `\x1b[31m${text}\x1b[0m`,
  green: (text) => `\x1b[32m${text}\x1b[0m`,
  yellow: (text) => `\x1b[33m${text}\x1b[0m`,
  cyan: (text) => `\x1b[36m${text}\x1b[0m`,
  gray: (text) => `\x1b[90m${text}\x1b[0m`
};

/**
 * Simple Election Data Ingestion Pipeline
 * 
 * Converts raw election data files into SQLite with dynamic schemas
 * that mirror the original file structure exactly.
 */

class ElectionIngester {
  constructor(electionPath, options = {}) {
    this.electionPath = path.resolve(electionPath);
    this.electionId = this.getElectionId();
    this.dbPath = path.join(this.electionPath, `ballots_${this.electionId}.db`);
    this.verbose = options.verbose || false;
    
    this.log(`🚀 Initializing ingestion for ${this.electionPath}`);
    this.log(`📊 Database: ${this.dbPath}`);
  }
  
  log(message) {
    if (this.verbose) {
      console.log(chalk.blue(`[${new Date().toISOString()}]`), message);
    }
  }
  
  error(message) {
    console.error(chalk.red(`[ERROR]`), message);
  }
  
  success(message) {
    console.log(chalk.green(`[SUCCESS]`), message);
  }
  
  getElectionId() {
    // Extract election ID from path: us/ny/nyc/2025/07 -> 2025_07
    const pathParts = this.electionPath.split(path.sep);
    const year = pathParts[pathParts.length - 2];
    const month = pathParts[pathParts.length - 1];
    return `${year}_${month}`;
  }
  
  async ingestFullElection() {
    try {
      console.log(chalk.yellow(`🚀 Starting ingestion for ${this.electionPath}`));
      
      // 1. Initialize SQLite database
      await this.initializeDatabase();
      
      // 2. Scan directory for files
      const files = await this.scanDirectory();
      console.log(chalk.cyan(`📁 Found ${files.length} files to process`));
      
      // 3. Filter out already processed files
      const filesToProcess = await this.getUnprocessedFiles(files);
      
      if (filesToProcess.length === 0) {
        console.log(chalk.green(`✅ All files already processed!`));
        return await this.generateStats();
      }
      
      console.log(chalk.cyan(`📋 ${filesToProcess.length} files remaining to process`));
      
      // 4. Process files sequentially with resume capability
      const startTime = Date.now();
      await this.processFiles(filesToProcess);
      const duration = Date.now() - startTime;
      
      // 5. Generate summary statistics
      const stats = await this.generateStats();
      
      this.success(`✅ Ingestion complete in ${duration}ms`);
      this.success(`📊 Processed ${stats.totalRows} rows across ${stats.totalTables} tables`);
      this.success(`💾 Database: ${this.dbPath}`);
      
      return stats;
      
    } catch (error) {
      this.error(`Failed to ingest election: ${error.message}`);
      throw error;
    }
  }
  
  async initializeDatabase() {
    this.log('🏗️  Initializing database...');
    
    const db = new Database(this.dbPath);
    
    // Simple tracking table for processed files
    db.exec(`
      CREATE TABLE IF NOT EXISTS _processed_files (
          id INTEGER PRIMARY KEY,
          filename TEXT UNIQUE NOT NULL,
          table_name TEXT,
          file_size INTEGER,
          rows_count INTEGER DEFAULT 0,
          status TEXT DEFAULT 'pending',
          error_message TEXT,
          processed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
      );
    `);
    
    db.close();
    this.log('✅ Database initialized');
  }
  
  async scanDirectory() {
    this.log('🔍 Scanning directory for files...');
    
    const files = [];
    const entries = await fs.readdir(this.electionPath, { withFileTypes: true });
    
    for (const entry of entries) {
      if (entry.isFile()) {
        const filePath = path.join(this.electionPath, entry.name);
        const ext = path.extname(entry.name).toLowerCase();
        const stats = await fs.stat(filePath);
        
        let fileType = 'unknown';
        if (ext === '.xlsx') fileType = 'xlsx';
        else if (ext === '.json') fileType = 'json';
        else if (ext === '.zip') fileType = 'zip';
        else if (ext === '.txt') fileType = 'txt';
        else if (ext === '.csv') fileType = 'csv';
        else if (ext === '.rcr') fileType = 'rcr';
        
        if (fileType !== 'unknown' && !entry.name.startsWith('.')) {
          files.push({
            path: filePath,
            name: entry.name,
            type: fileType,
            size: stats.size
          });
        }
      }
    }
    
    // Sort by size (process smaller files first)
    files.sort((a, b) => a.size - b.size);
    
    this.log(`📁 Found ${files.length} processable files`);
    return files;
  }
  
  async getUnprocessedFiles(allFiles) {
    const db = new Database(this.dbPath);
    
    const processedFiles = db.prepare(`
      SELECT filename FROM _processed_files WHERE status IN ('completed', 'error')
    `).all().map(row => row.filename);
    
    db.close();
    
    return allFiles.filter(file => !processedFiles.includes(file.name));
  }
  
  async processFiles(files) {
    if (files.length === 0) return;
    
    this.log(`⚡ Processing ${files.length} files sequentially...`);
    
    // Create progress bar
    const progressBar = new ProgressBar('Processing [:bar] :percent :current/:total :etas', {
      complete: '█',
      incomplete: '░',
      width: 40,
      total: files.length
    });
    
    const db = new Database(this.dbPath);
    let totalRows = 0;
    
    for (let i = 0; i < files.length; i++) {
      const file = files[i];
      
      try {
        // Mark file as processing
        const markProcessing = db.prepare(`
          INSERT OR REPLACE INTO _processed_files (filename, file_size, status)
          VALUES (?, ?, 'processing')
        `);
        markProcessing.run(file.name, file.size);
        
        // Process the file
        let result = { rowsProcessed: 0, tableName: null };
        
        switch (file.type) {
          case 'xlsx':
            result = await this.processXLSXFile(db, file);
            break;
          case 'csv':
            result = await this.processCSVFile(db, file);
            break;
          case 'json':
            result = await this.processJSONFile(db, file);
            break;
          case 'txt':
            result = await this.processTXTFile(db, file);
            break;
          case 'zip':
            result = await this.processZIPFile(db, file);
            break;
          default:
            console.log(`⚠️  Unknown file type: ${file.type} for ${file.name}`);
        }
        
        // Mark file as completed
        const markCompleted = db.prepare(`
          UPDATE _processed_files 
          SET status = 'completed', table_name = ?, rows_count = ?, processed_at = CURRENT_TIMESTAMP
          WHERE filename = ?
        `);
        markCompleted.run(result.tableName || 'skipped', result.rowsProcessed, file.name);
        
        totalRows += result.rowsProcessed;
        progressBar.tick();
        
        if (this.verbose) {
          console.log(`\n✅ ${file.name} → ${result.tableName}: ${result.rowsProcessed} rows`);
        }
        
      } catch (error) {
        // Mark file as error
        const markError = db.prepare(`
          UPDATE _processed_files 
          SET status = 'error', error_message = ?, processed_at = CURRENT_TIMESTAMP
          WHERE filename = ?
        `);
        markError.run(error.message, file.name);
        
        console.error(`\n❌ Error processing ${file.name}: ${error.message}`);
        progressBar.tick();
      }
    }
    
    db.close();
    console.log(`\n✅ Processed ${totalRows} total rows from ${files.length} files`);
  }
  
  async processXLSXFile(db, file) {
    // Skip candidates file for now - just process CVR files
    if (file.name.includes('CandidacyID_To_Name')) {
      if (this.verbose) console.log(`📋 Skipping candidates file: ${file.name}`);
      return { rowsProcessed: 0, tableName: 'candidates_file_skipped' };
    }
    
    console.log(`📄 Processing ${file.name} (${(file.size / 1024 / 1024).toFixed(1)}MB)...`);
    
    try {
      const tableName = this.sanitizeTableName(file.name);
      let headers = null;
      let rowsProcessed = 0;
      let insertStmt = null;
      let currentBatch = [];
      const batchSize = 1000;
      
      console.log(`  📖 Reading XLSX file with standard xlsx library...`);
      console.log(`  📊 File stats: ${(file.size / 1024 / 1024).toFixed(1)}MB`);
      
      const startTime = Date.now();
      console.log(`  ⏰ Starting file read at ${new Date().toISOString()}`);
      
      // Read the workbook with minimal options for better performance
      const workbook = XLSX.readFile(file.path, { 
        cellDates: false,
        cellNF: false,
        cellStyles: false,
        sheetStubs: false,
        dense: false,
        bookVBA: false,
        bookSheets: false,
        bookProps: false,
        bookFiles: false,
        bookDeps: false
      });
      
      const readTime = Date.now() - startTime;
      console.log(`  ✅ File read completed in ${readTime}ms`);
      
      // Get the first worksheet
      console.log(`  📚 Workbook contains ${workbook.SheetNames.length} sheets: ${workbook.SheetNames.join(', ')}`);
      const sheetName = workbook.SheetNames[0];
      const worksheet = workbook.Sheets[sheetName];
      
      console.log(`  📊 Processing worksheet: ${sheetName}`);
      
      // Convert to array of arrays for easier processing
      const range = XLSX.utils.decode_range(worksheet['!ref'] || 'A1:A1');
      const totalRows = range.e.r + 1;
      const totalCols = range.e.c + 1;
      console.log(`  📏 Worksheet range: ${worksheet['!ref']} (${totalRows} rows, ${totalCols} columns)`);
      console.log(`  💾 Estimated memory usage: ~${Math.round(totalRows * totalCols * 50 / 1024 / 1024)}MB`);
      
      const processingStartTime = Date.now();
      
      // Process row by row to avoid loading everything into memory at once
      for (let rowNum = range.s.r; rowNum <= range.e.r; rowNum++) {
        // Progress logging every 1000 rows
        if (rowNum % 1000 === 0) {
          const progress = ((rowNum - range.s.r) / (range.e.r - range.s.r + 1) * 100).toFixed(1);
          const elapsed = Date.now() - processingStartTime;
          console.log(`    🔄 Processing row ${rowNum + 1}/${totalRows} (${progress}%) - ${elapsed}ms elapsed`);
          
          // Force garbage collection every 5000 rows
          if (rowNum % 5000 === 0 && global.gc) {
            global.gc();
          }
        }
        
        const row = [];
        
        // Extract values for this row
        for (let colNum = range.s.c; colNum <= range.e.c; colNum++) {
          const cellAddress = XLSX.utils.encode_cell({ r: rowNum, c: colNum });
          const cell = worksheet[cellAddress];
          row.push(cell ? (cell.v !== undefined ? cell.v : '') : '');
        }
        
        // Skip empty rows
        if (row.every(cell => cell === '' || cell === null || cell === undefined)) {
          continue;
        }
        
        if (headers === null) {
          // First non-empty row becomes headers
          headers = row.map((header, index) => {
            const headerStr = String(header || '').trim();
            return this.sanitizeColumnName(headerStr || `column_${index}`);
          });
          
          console.log(`  🏗️  Found headers at row ${rowNum + 1}, creating table ${tableName} with ${headers.length} columns...`);
          console.log(`  📋 Headers: ${headers.slice(0, 5).join(', ')}...`);
          
          // Create table with dynamic schema
          const columnDefs = headers.map(header => `"${header}" TEXT`).join(', ');
          const createTableSQL = `CREATE TABLE IF NOT EXISTS "${tableName}" (id INTEGER PRIMARY KEY, ${columnDefs})`;
          db.exec(createTableSQL);
          
          // Prepare insert statement
          const placeholders = headers.map(() => '?').join(', ');
          const insertSQL = `INSERT INTO "${tableName}" (${headers.map(h => `"${h}"`).join(', ')}) VALUES (${placeholders})`;
          insertStmt = db.prepare(insertSQL);
          
          console.log(`  💾 Processing rows in batches of ${batchSize}...`);
          continue;
        }
        
        // Data row - pad to match header length
        const paddedRow = [...row];
        while (paddedRow.length < headers.length) {
          paddedRow.push('');
        }
        
        // Convert all values to strings
        const stringRow = paddedRow.slice(0, headers.length).map(val => 
          val === null || val === undefined ? '' : String(val)
        );
        
        currentBatch.push(stringRow);
        
        // Process batch when it reaches batchSize
        if (currentBatch.length >= batchSize) {
          this.processBatch(db, insertStmt, currentBatch);
          rowsProcessed += currentBatch.length;
          
          if (rowsProcessed % (batchSize * 10) === 0) {
            console.log(`    📦 Processed ${rowsProcessed} rows...`);
            if (global.gc) global.gc(); // Garbage collect periodically
          }
          
          currentBatch = [];
        }
      }
      
      // Process remaining batch
      if (currentBatch.length > 0 && insertStmt !== null) {
        this.processBatch(db, insertStmt, currentBatch);
        rowsProcessed += currentBatch.length;
      }
      
      console.log(`  ✅ Completed ${file.name}: ${rowsProcessed} rows inserted`);
      
      // Force garbage collection
      if (global.gc) {
        global.gc();
      }
      
      return { rowsProcessed, tableName };
      
    } catch (error) {
      console.error(`Error processing XLSX file ${file.name}:`, error);
      throw error;
    }
  }
  
  processBatch(db, insertStmt, batch) {
    if (!insertStmt) {
      console.error('Insert statement is null - skipping batch');
      return;
    }
    
    const insertMany = db.transaction((rows) => {
      for (const row of rows) {
        insertStmt.run(...row);
      }
    });
    
    insertMany(batch);
  }
  
  async processCSVFile(db, file) {
    if (this.verbose) console.log(`📄 CSV processing not yet implemented: ${file.name}`);
    return { rowsProcessed: 0, tableName: 'csv_not_implemented' };
  }
  
  async processJSONFile(db, file) {
    if (this.verbose) console.log(`📄 JSON processing not yet implemented: ${file.name}`);
    return { rowsProcessed: 0, tableName: 'json_not_implemented' };
  }
  
  async processTXTFile(db, file) {
    if (this.verbose) console.log(`📄 TXT processing not yet implemented: ${file.name}`);
    return { rowsProcessed: 0, tableName: 'txt_not_implemented' };
  }
  
  async processZIPFile(db, file) {
    if (this.verbose) console.log(`📦 ZIP processing not yet implemented: ${file.name}`);
    return { rowsProcessed: 0, tableName: 'zip_not_implemented' };
  }
  
  sanitizeTableName(filename) {
    // Convert filename to valid SQLite table name
    return filename
      .replace(/\.[^.]+$/, '') // Remove extension
      .replace(/[^a-zA-Z0-9_]/g, '_') // Replace invalid chars with underscore
      .replace(/^(\d)/, '_$1') // Prefix with underscore if starts with number
      .toLowerCase();
  }
  
  sanitizeColumnName(columnName) {
    // Convert column name to valid SQLite column name
    if (!columnName || columnName.trim() === '') {
      return 'unnamed_column';
    }
    
    const sanitized = columnName
      .replace(/[^a-zA-Z0-9_\s]/g, '_') // Replace invalid chars
      .replace(/\s+/g, '_') // Replace spaces with underscore
      .replace(/_{2,}/g, '_') // Replace multiple underscores with single
      .replace(/^_+|_+$/g, '') // Remove leading/trailing underscores
      .toLowerCase();
      
    return sanitized || 'unnamed_column';
  }
  
  async generateStats() {
    const db = new Database(this.dbPath);
    
    // Get all user tables (not starting with _)
    const tables = db.prepare(`
      SELECT name FROM sqlite_master 
      WHERE type='table' AND name NOT LIKE '_%'
    `).all();
    
    let totalRows = 0;
    for (const table of tables) {
      const count = db.prepare(`SELECT COUNT(*) as count FROM "${table.name}"`).get().count;
      totalRows += count;
    }
    
    const stats = {
      totalTables: tables.length,
      totalRows: totalRows,
      tables: tables.map(t => t.name)
    };
    
    db.close();
    return stats;
  }
}

// CLI interface
if (require.main === module) {
  program
    .name('ingest-election')
    .description('Simple election data ingestion pipeline')
    .version('1.0.0')
    .argument('<election-path>', 'Path to election directory')
    .option('-v, --verbose', 'Enable verbose logging')
    .action(async (electionPath, options) => {
      try {
        const ingester = new ElectionIngester(electionPath, options);
        await ingester.ingestFullElection();
        process.exit(0);
      } catch (error) {
        console.error(chalk.red('Ingestion failed:'), error.message);
        process.exit(1);
      }
    });
  
  program.parse();
}

module.exports = { ElectionIngester };
