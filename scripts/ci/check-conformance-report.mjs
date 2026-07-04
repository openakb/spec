import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { basename, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = fileURLToPath(new URL('../../', import.meta.url));
const DESCRIPTOR_KINDS = new Set(['valid', 'invalid', 'forward-compat']);

let failureCount = 0;

function usage() {
  console.error('Usage: check-conformance-report.mjs <report.json> [<report.json> ...]');
  process.exit(2);
}

function rel(path) {
  return relative(ROOT, path);
}

function report(message, file) {
  if (file) {
    console.error(`::error file=${rel(file)}::${message}`);
  } else {
    console.error(`::error::${message}`);
  }
  failureCount += 1;
}

function readJson(file) {
  try {
    return JSON.parse(readFileSync(file, 'utf8'));
  } catch (error) {
    const reason = error instanceof Error ? error.message : String(error);
    report(`unable to read JSON: ${reason}`, file);
    return undefined;
  }
}

function caseDirs(kind) {
  const dir = join(ROOT, 'conformance', kind);
  if (!existsSync(dir)) {
    report(`missing fixture directory: conformance/${kind}`);
    return [];
  }
  return readdirSync(dir, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
    .sort();
}

function expectedCases() {
  const cases = new Map();

  for (const name of caseDirs('valid')) {
    cases.set(`valid/${name}`, { kind: 'valid' });
  }

  for (const name of caseDirs('invalid')) {
    const expected = readJson(join(ROOT, 'conformance', 'invalid', name, 'expected.json'));
    if (!isObject(expected) || !isStringArray(expected.codes)) {
      report(`invalid/${name} expected.json codes must be an array of strings`);
      continue;
    }
    cases.set(`invalid/${name}`, { kind: 'invalid', codes: expected.codes });
  }

  for (const name of caseDirs('forward-compat')) {
    const expected = readJson(join(ROOT, 'conformance', 'forward-compat', name, 'expected.json'));
    if (!isObject(expected) || !isStringArray(expected.strict)) {
      report(`forward-compat/${name} expected.json strict must be an array of strings`);
      continue;
    }
    cases.set(`forward-compat/${name}`, { kind: 'forward-compat', strict: expected.strict });
  }

  for (const name of caseDirs('content')) {
    const expected = readJson(join(ROOT, 'conformance', 'content', name, 'expected.json'));
    if (!isObject(expected) || !Array.isArray(expected.citations)) {
      report(`content/${name} expected.json citations must be an array`);
      continue;
    }
    const citations = expected.citations.map((citation) => (isObject(citation) ? citation.ids : undefined));
    if (!isCitationArrays(citations)) {
      report(`content/${name} expected.json citations[].ids must be arrays of strings`);
      continue;
    }
    cases.set(`content/${name}`, {
      kind: 'content',
      citations,
    });
  }

  return cases;
}

function isObject(value) {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function isStringArray(value) {
  return Array.isArray(value) && value.every((item) => typeof item === 'string');
}

function isCitationArrays(value) {
  return Array.isArray(value) && value.every(isStringArray);
}

function sortedStrings(value) {
  return [...value].sort();
}

function sameJson(left, right) {
  return JSON.stringify(left) === JSON.stringify(right);
}

function hasEvery(actual, expected) {
  const actualSet = new Set(actual ?? []);
  return expected.every((code) => actualSet.has(code));
}

function checkFixture(reportPath, key, expected, actual) {
  if (actual === undefined) {
    report(`missing report fixture: ${key}`, reportPath);
    return;
  }

  if (expected.kind === 'valid') {
    if (!sameJson(sortedStrings(actual.lenient), [])) {
      report(`${key} lenient must be empty`, reportPath);
    }
    if (!sameJson(sortedStrings(actual.strict), [])) {
      report(`${key} strict must be empty`, reportPath);
    }
    return;
  }

  if (expected.kind === 'invalid') {
    if (!hasEvery(sortedStrings(actual.lenient), expected.codes)) {
      report(`${key} lenient missing expected code(s): ${expected.codes.join(', ')}`, reportPath);
    }
    return;
  }

  if (expected.kind === 'forward-compat') {
    if (!sameJson(sortedStrings(actual.lenient), [])) {
      report(`${key} lenient must be empty`, reportPath);
    }
    if (!hasEvery(sortedStrings(actual.strict), expected.strict)) {
      report(`${key} strict missing expected code(s): ${expected.strict.join(', ')}`, reportPath);
    }
    return;
  }

  if (!sameJson(actual.citations, expected.citations)) {
    report(`${key} citations do not match expected ids`, reportPath);
  }
}

function loadReport(path) {
  const reportData = readJson(path);
  if (reportData === undefined) {
    return undefined;
  }
  if (!isObject(reportData)) {
    report('report must be an object', path);
    return undefined;
  }
  if (!isObject(reportData.fixtures)) {
    report('report fixtures must be an object', path);
    return undefined;
  }
  return reportData;
}

function fixtureKind(key) {
  return key.slice(0, key.indexOf('/'));
}

function validateFixtureShape(reportPath, key, actual) {
  if (!isObject(actual)) {
    report(`${key} fixture must be an object`, reportPath);
    return false;
  }

  if (DESCRIPTOR_KINDS.has(fixtureKind(key))) {
    let valid = true;
    if (!isStringArray(actual.lenient)) {
      report(`${key} lenient must be an array of strings`, reportPath);
      valid = false;
    }
    if (!isStringArray(actual.strict)) {
      report(`${key} strict must be an array of strings`, reportPath);
      valid = false;
    }
    return valid;
  }

  if (!isCitationArrays(actual.citations)) {
    report(`${key} citations must be an array of string arrays`, reportPath);
    return false;
  }
  return true;
}

function validateReportFixtures(reportPath, fixtures, cases) {
  let valid = true;

  for (const key of Object.keys(fixtures).sort()) {
    if (!cases.has(key)) {
      report(`unknown report fixture: ${key}`, reportPath);
      valid = false;
      continue;
    }
    if (!validateFixtureShape(reportPath, key, fixtures[key])) {
      valid = false;
    }
  }

  for (const key of cases.keys()) {
    if (fixtures[key] === undefined) {
      report(`missing report fixture: ${key}`, reportPath);
      valid = false;
    }
  }

  return valid;
}

function checkAgreement(reports) {
  if (reports.length < 2) {
    return;
  }

  const keys = new Set();
  for (const { reportData } of reports) {
    for (const key of Object.keys(reportData.fixtures ?? {})) {
      keys.add(key);
    }
  }

  const first = reports[0];
  for (const key of [...keys].sort()) {
    const expected = first.reportData.fixtures?.[key];
    for (const other of reports.slice(1)) {
      if (!sameJson(other.reportData.fixtures?.[key], expected)) {
        report(
          `${basename(other.path)} disagrees with ${basename(first.path)} for fixture ${key}`,
          other.path,
        );
      }
    }
  }
}

const reportPaths = process.argv.slice(2);
if (reportPaths.length === 0) {
  usage();
}

const cases = expectedCases();
const loadedReports = reportPaths.map((path) => ({ path, reportData: loadReport(path) }));
const validReports = loadedReports.filter(({ reportData }) => reportData !== undefined);

let shapeValid = true;
for (const { path, reportData } of validReports) {
  const fixtures = reportData.fixtures;
  if (!validateReportFixtures(path, fixtures, cases)) {
    shapeValid = false;
    continue;
  }
  for (const [key, expected] of cases) {
    checkFixture(path, key, expected, fixtures[key]);
  }
}

if (loadedReports.length !== validReports.length) {
  shapeValid = false;
}

if (shapeValid) {
  checkAgreement(validReports);
}

if (failureCount > 0) {
  console.error(`Conformance report check failed: ${failureCount} failure(s).`);
  process.exit(1);
}

console.log(
  `Conformance report check OK: ${validReports.length} report(s) over the full fixture suite.`,
);
