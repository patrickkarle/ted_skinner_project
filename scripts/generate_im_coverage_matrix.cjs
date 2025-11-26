#!/usr/bin/env node
/**
 * IM Coverage Matrix Generator
 *
 * Generates Appendix A for L5-TESTPLAN by cross-referencing all 351 IM codes
 * from L4-MANIFEST against test specifications.
 *
 * Usage: node scripts/generate_im_coverage_matrix.js
 * Output: docs/se-cpm/APPENDIX_A_IM_COVERAGE_MATRIX.md
 */

const fs = require('fs');
const path = require('path');

// Configuration
const CONFIG = {
  l4ManifestPath: path.join(__dirname, '../docs/se-cpm/L4-MANIFEST-ImplementationInventory.md'),
  l5TestPlanPath: path.join(__dirname, '../docs/se-cpm/L5-TESTPLAN-TestSpecification.md'),
  imCodesPath: path.join(__dirname, '../docs/se-cpm/im_codes_extracted.txt'),
  outputPath: path.join(__dirname, '../docs/se-cpm/APPENDIX_A_IM_COVERAGE_MATRIX.md')
};

/**
 * Extract IM code details from L4-MANIFEST
 */
function extractIMDetails(manifestContent) {
  const imMap = new Map();
  const lines = manifestContent.split('\n');

  let currentIMCode = null;
  let currentComponent = null;
  let currentContext = '';

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];

    // Match IM code definitions
    const imMatch = line.match(/^####\s+(IM-\d{4}(-[FPVBE]\d+)?)[:\s]/);
    if (imMatch) {
      currentIMCode = imMatch[1];
      // Extract component name from header
      const componentMatch = line.match(/####\s+IM-\d{4}(-[FPVBE]\d+)?[:\s]+(.+)$/);
      currentComponent = componentMatch ? componentMatch[2].trim() : 'Unknown';
      currentContext = '';
      continue;
    }

    // Collect context for current IM code
    if (currentIMCode && line.trim()) {
      if (line.startsWith('**Location:**')) {
        const locationMatch = line.match(/\*\*Location:\*\*\s+(.+)/);
        if (locationMatch) {
          currentContext += ` | Location: ${locationMatch[1]}`;
        }
      }
      if (line.startsWith('**Type:**')) {
        const typeMatch = line.match(/\*\*Type:\*\*\s+(.+)/);
        if (typeMatch) {
          currentContext += ` | Type: ${typeMatch[1]}`;
        }
      }
    }

    // When we hit a new section or reach significant content, store the IM code
    if (currentIMCode && (line.startsWith('###') || line.startsWith('##')) && !line.includes(currentIMCode)) {
      if (!imMap.has(currentIMCode)) {
        imMap.set(currentIMCode, {
          code: currentIMCode,
          component: currentComponent,
          context: currentContext.trim()
        });
      }
      currentIMCode = null;
      currentComponent = null;
      currentContext = '';
    }
  }

  return imMap;
}

/**
 * Extract test mappings from L5-TESTPLAN
 */
function extractTestMappings(testPlanContent) {
  const testMap = new Map();
  const lines = testPlanContent.split('\n');

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];

    // Match test definitions
    const testMatch = line.match(/^####\s+(TEST-[A-Z]+-\d+)/);
    if (testMatch) {
      const testId = testMatch[1];

      // Look ahead for Manifest Reference
      for (let j = i + 1; j < Math.min(i + 10, lines.length); j++) {
        const nextLine = lines[j];
        const refMatch = nextLine.match(/\*\*Manifest Reference:\*\*\s+(.+)/);
        if (refMatch) {
          const refs = refMatch[1].split(/[,\s]+/).filter(r => r.trim());
          refs.forEach(ref => {
            const imCode = ref.trim();
            if (imCode.startsWith('IM-')) {
              if (!testMap.has(imCode)) {
                testMap.set(imCode, []);
              }
              testMap.get(imCode).push(testId);
            }
          });
          break;
        }
        // Stop if we hit next test
        if (nextLine.startsWith('####')) break;
      }
    }

    // Match battery test ranges
    const batteryMatch = line.match(/^####\s+TEST-UNIT-(\d+)\s+through\s+TEST-UNIT-(\d+)/);
    if (batteryMatch) {
      const startNum = parseInt(batteryMatch[1]);
      const endNum = parseInt(batteryMatch[2]);

      // Look for IM code range
      for (let j = i + 1; j < Math.min(i + 15, lines.length); j++) {
        const nextLine = lines[j];
        const imRangeMatch = nextLine.match(/IM-(\d{4})(-[FPVBE]\d+)?\s+through\s+IM-(\d{4})(-[FPVBE]\d+)?/);
        if (imRangeMatch) {
          const startIM = parseInt(imRangeMatch[1]);
          const endIM = parseInt(imRangeMatch[3]);

          // Map all IM codes in range to battery test range
          for (let imNum = startIM; imNum <= endIM; imNum++) {
            const imCode = `IM-${String(imNum).padStart(4, '0')}`;
            if (!testMap.has(imCode)) {
              testMap.set(imCode, []);
            }
            testMap.get(imCode).push(`TEST-UNIT-${startNum}-${endNum} (Battery)`);
          }
          break;
        }
      }
    }
  }

  return testMap;
}

/**
 * Determine coverage type based on IM code suffix
 */
function getCoverageType(imCode) {
  if (imCode.includes('-F')) return 'Field (F)';
  if (imCode.includes('-P')) return 'Parameter (P)';
  if (imCode.includes('-V')) return 'Variable (V)';
  if (imCode.includes('-B')) return 'Branch (B)';
  if (imCode.includes('-E')) return 'Error (E)';
  return 'Component';
}

/**
 * Determine section reference based on IM code prefix
 */
function getSectionReference(imCode) {
  const prefix = imCode.split('-')[1].substring(0, 1);
  const mapping = {
    '1': '9.14', // AppState, Config, Manifest
    '2': '9.15-9.20', // Agent, Tools
    '3': '9.16-9.17', // LLM Providers
    '4': '9.18-9.19', // Quality, Retry
    '5': '9.21-9.22', // StateManager, Database
    '6': '9.23-9.25', // Export, Resume
    '7': '9.26-9.27'  // Frontend components
  };
  return mapping[prefix] || '9.x';
}

/**
 * Generate markdown table
 */
function generateCoverageMatrix(imCodes, imDetails, testMappings) {
  let markdown = `# Appendix A: Complete IM Coverage Matrix

**Generated:** ${new Date().toISOString()}
**Total IM Codes:** ${imCodes.length}
**Coverage:** ${imCodes.filter(code => testMappings.has(code)).length}/${imCodes.length} (${Math.round(imCodes.filter(code => testMappings.has(code)).length / imCodes.length * 100)}%)

---

## Coverage Summary by Category

| Category | IM Code Range | Count | Covered | Coverage % |
|----------|---------------|-------|---------|------------|
| App State & Config | IM-1001 to IM-1104 | ${imCodes.filter(c => c.startsWith('IM-1')).length} | ${imCodes.filter(c => c.startsWith('IM-1') && testMappings.has(c)).length} | ${Math.round(imCodes.filter(c => c.startsWith('IM-1') && testMappings.has(c)).length / imCodes.filter(c => c.startsWith('IM-1')).length * 100)}% |
| Agent & Tools | IM-2001 to IM-2200 | ${imCodes.filter(c => c.startsWith('IM-2')).length} | ${imCodes.filter(c => c.startsWith('IM-2') && testMappings.has(c)).length} | ${Math.round(imCodes.filter(c => c.startsWith('IM-2') && testMappings.has(c)).length / imCodes.filter(c => c.startsWith('IM-2')).length * 100)}% |
| LLM Integration | IM-3001 to IM-3400 | ${imCodes.filter(c => c.startsWith('IM-3')).length} | ${imCodes.filter(c => c.startsWith('IM-3') && testMappings.has(c)).length} | ${Math.round(imCodes.filter(c => c.startsWith('IM-3') && testMappings.has(c)).length / imCodes.filter(c => c.startsWith('IM-3')).length * 100)}% |
| Quality & Retry | IM-4001 to IM-4301 | ${imCodes.filter(c => c.startsWith('IM-4')).length} | ${imCodes.filter(c => c.startsWith('IM-4') && testMappings.has(c)).length} | ${Math.round(imCodes.filter(c => c.startsWith('IM-4') && testMappings.has(c)).length / imCodes.filter(c => c.startsWith('IM-4')).length * 100)}% |
| State Management | IM-5001 to IM-5104 | ${imCodes.filter(c => c.startsWith('IM-5')).length} | ${imCodes.filter(c => c.startsWith('IM-5') && testMappings.has(c)).length} | ${Math.round(imCodes.filter(c => c.startsWith('IM-5') && testMappings.has(c)).length / imCodes.filter(c => c.startsWith('IM-5')).length * 100)}% |
| Export & Resume | IM-6001 to IM-6303 | ${imCodes.filter(c => c.startsWith('IM-6')).length} | ${imCodes.filter(c => c.startsWith('IM-6') && testMappings.has(c)).length} | ${Math.round(imCodes.filter(c => c.startsWith('IM-6') && testMappings.has(c)).length / imCodes.filter(c => c.startsWith('IM-6')).length * 100)}% |
| Frontend Components | IM-7001 to IM-7201 | ${imCodes.filter(c => c.startsWith('IM-7')).length} | ${imCodes.filter(c => c.startsWith('IM-7') && testMappings.has(c)).length} | ${Math.round(imCodes.filter(c => c.startsWith('IM-7') && testMappings.has(c)).length / imCodes.filter(c => c.startsWith('IM-7')).length * 100)}% |

---

## Complete IM Coverage Table

| IM Code | Component | Test Case(s) | Coverage Type | Section |
|---------|-----------|--------------|---------------|---------|
`;

  imCodes.forEach(code => {
    const details = imDetails.get(code) || { component: 'Unknown', context: '' };
    const tests = testMappings.get(code) || ['❌ NO COVERAGE'];
    const coverageType = getCoverageType(code);
    const section = getSectionReference(code);

    markdown += `| ${code} | ${details.component} | ${tests.join(', ')} | ${coverageType} | ${section} |\n`;
  });

  markdown += `\n---

## Verification

To verify this coverage matrix:

\`\`\`bash
# Extract all IM codes from L4-MANIFEST
grep -oE "IM-[0-9]{4}(-[FPVBE][0-9]+)?" docs/se-cpm/L4-MANIFEST-ImplementationInventory.md | sort -u > im_codes.txt

# Count total
wc -l im_codes.txt
# Expected: 351 lines

# Extract all test references from L5-TESTPLAN
grep "Manifest Reference:" docs/se-cpm/L5-TESTPLAN-TestSpecification.md | grep -oE "IM-[0-9]{4}(-[FPVBE][0-9]+)?" | sort -u > tested_im_codes.txt

# Count covered
wc -l tested_im_codes.txt

# Find uncovered (should be empty for 100% coverage)
comm -23 im_codes.txt tested_im_codes.txt
\`\`\`

---

**Status:** Complete IM Coverage Matrix Generated
**Next:** Integrate into L5-TESTPLAN as Appendix A
`;

  return markdown;
}

/**
 * Main execution
 */
async function main() {
  try {
    console.log('🚀 Starting IM Coverage Matrix Generation...\n');

    // Read input files
    console.log('📖 Reading L4-MANIFEST...');
    const manifestContent = fs.readFileSync(CONFIG.l4ManifestPath, 'utf8');

    console.log('📖 Reading L5-TESTPLAN...');
    const testPlanContent = fs.readFileSync(CONFIG.l5TestPlanPath, 'utf8');

    console.log('📖 Reading extracted IM codes...');
    const imCodes = fs.readFileSync(CONFIG.imCodesPath, 'utf8')
      .split('\n')
      .map(code => code.trim())
      .filter(code => code.length > 0);

    console.log(`✓ Found ${imCodes.length} IM codes\n`);

    // Extract details
    console.log('🔍 Extracting IM code details from L4-MANIFEST...');
    const imDetails = extractIMDetails(manifestContent);
    console.log(`✓ Extracted details for ${imDetails.size} IM codes\n`);

    // Extract test mappings
    console.log('🔍 Extracting test mappings from L5-TESTPLAN...');
    const testMappings = extractTestMappings(testPlanContent);
    console.log(`✓ Found test mappings for ${testMappings.size} IM codes\n`);

    // Generate matrix
    console.log('📝 Generating coverage matrix...');
    const markdown = generateCoverageMatrix(imCodes, imDetails, testMappings);

    // Write output
    console.log(`💾 Writing to ${CONFIG.outputPath}...`);
    fs.writeFileSync(CONFIG.outputPath, markdown, 'utf8');

    // Summary
    const covered = imCodes.filter(code => testMappings.has(code)).length;
    const total = imCodes.length;
    const percentage = Math.round((covered / total) * 100);

    console.log('\n✅ Coverage Matrix Generated Successfully!\n');
    console.log(`📊 Coverage Summary:`);
    console.log(`   Total IM Codes: ${total}`);
    console.log(`   Covered: ${covered}`);
    console.log(`   Uncovered: ${total - covered}`);
    console.log(`   Coverage: ${percentage}%\n`);

    if (percentage < 100) {
      console.log('⚠️  WARNING: Coverage is not 100%. Some IM codes lack test coverage.');
      const uncovered = imCodes.filter(code => !testMappings.has(code));
      console.log(`\nUncovered IM Codes (${uncovered.length}):`);
      uncovered.slice(0, 20).forEach(code => console.log(`   - ${code}`));
      if (uncovered.length > 20) {
        console.log(`   ... and ${uncovered.length - 20} more`);
      }
    }

  } catch (error) {
    console.error('❌ Error generating coverage matrix:', error.message);
    process.exit(1);
  }
}

// Run if called directly
if (require.main === module) {
  main();
}

module.exports = { extractIMDetails, extractTestMappings, generateCoverageMatrix };
