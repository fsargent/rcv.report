import puppeteer from 'puppeteer';
import fs from 'fs/promises';
import path from 'path';

async function generateShareImages() {
  console.log('Starting share image generation...');

  const chromePath = process.env.CHROME_PATH || 'google-chrome-stable';
  console.log('Using Chrome at:', chromePath);

  let browser = await puppeteer.launch({
    headless: 'new',
    args: [
      '--no-sandbox',
      '--disable-setuid-sandbox',
      '--disable-dev-shm-usage',
      '--disable-gpu',
      '--disable-extensions',
      '--single-process', // Important for CI
      '--no-zygote', // Important for CI
    ],
    executablePath: chromePath,
  });

  // Read reports index
  const indexRaw = await fs.readFile('report_pipeline/reports/index.json', 'utf8');
  const index = JSON.parse(indexRaw);
  const reports = index.contests || [];

  console.log(`Found ${reports.length} reports to process`);

  let page = await browser.newPage();
  await page.setDefaultTimeout(30000);
  await page.setViewport({
    width: 1200,
    height: 630,
    deviceScaleFactor: 4,
  });

  page.on('console', (msg) => console.log('Browser console:', msg.text()));
  page.on('pageerror', (err) => console.error('Browser error:', err));

  let successCount = 0;
  let failureCount = 0;

  for (const report of reports) {
    const reportPath = report.path;
    const outputPath = `static/share/${reportPath}.png`;
    const outputDir = path.dirname(outputPath);

    try {
      console.log(`Processing: ${reportPath}`);

      await fs.mkdir(outputDir, { recursive: true });

      const url = `http://localhost:3000/card/${reportPath}`;
      console.log(`Loading URL: ${url}`);

      await page.goto(url, {
        waitUntil: 'domcontentloaded',
        timeout: 10000,
      });

      console.log('Waiting for .card element...');
      await page.waitForSelector('.card', { timeout: 5000 });

      console.log('Taking screenshot...');
      const element = await page.$('.card');
      if (!element) {
        throw new Error('Card element not found');
      }

      await element.screenshot({
        path: outputPath,
        type: 'png',
        omitBackground: false,
      });

      successCount++;
      console.log(`✓ Generated: ${outputPath} (${successCount}/${reports.length})`);

      await new Promise((resolve) => setTimeout(resolve, 100));
    } catch (error) {
      failureCount++;
      console.error(`✗ Failed ${reportPath}:`, error.message);

      try {
        await page.reload({ waitUntil: 'domcontentloaded', timeout: 5000 });
      } catch (reloadError) {
        console.error('Failed to reload page:', reloadError.message);
      }
    }

    if (successCount > 0 && successCount % 10 === 0) {
      console.log('Restarting browser to clear memory...');
      await page.close();
      await browser.close();

      browser = await puppeteer.launch({
        headless: 'new',
        args: [
          '--no-sandbox',
          '--disable-setuid-sandbox',
          '--disable-dev-shm-usage',
          '--disable-gpu',
          '--disable-extensions',
          '--single-process',
          '--no-zygote',
        ],
        executablePath: chromePath,
      });

      page = await browser.newPage();
      await page.setDefaultTimeout(30000);
      await page.setViewport({
        width: 1200,
        height: 630,
        deviceScaleFactor: 4,
      });
      page.on('console', (msg) => console.log('Browser console:', msg.text()));
      page.on('pageerror', (err) => console.error('Browser error:', err));
    }
  }

  await page.close();
  await browser.close();

  console.log('\nGeneration complete!');
  console.log(`Successful: ${successCount}`);
  console.log(`Failed: ${failureCount}`);
}

// Make sure the dev server is running
async function checkDevServer() {
  try {
    const response = await fetch('http://localhost:3000');
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    console.log('Dev server is running');
    return true;
  } catch {
    console.error('ERROR: Dev server is not running at http://localhost:3000');
    console.error("Please start the dev server with './dev.sh' first");
    process.exit(1);
  }
}

// Run the script
checkDevServer()
  .then(() => generateShareImages())
  .catch((error) => {
    console.error('Fatal error:', error);
    process.exit(1);
  });
