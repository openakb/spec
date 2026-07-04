import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { basename, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = fileURLToPath(new URL('../../', import.meta.url));

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
  return JSON.parse(readFileSync(file, 'utf8'));
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
    cases.set(`invalid/${name}`, { kind: 'invalid', codes: expected.codes });
  }

  for (const name of caseDirs('forward-compat')) {
    const expected = readJson(join(ROOT, 'conformance', 'forward-compat', name, 'expected.json'));
    cases.set(`forward-compat/${name}`, { kind: 'forward-compat', strict: expected.strict });
  }

  for (const name of caseDirs('content')) {
    const expected = readJson(join(ROOT, 'conformance', 'content', name, 'expected.json'));
    cases.set(`content/${name}`, {
      kind: 'content',
      citations: expected.citations.map((citation) => citation.ids),
    });
  }

  return cases;
}

function sortedStrings(value) {
  return Array.isArray(value) && value.every((item) => typeof item === 'string')
    ? [...value].sort()
    : undefined;
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
  return readJson(path);
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
const reports = reportPaths.map((path) => ({ path, reportData: loadReport(path) }));

for (const { path, reportData } of reports) {
  const fixtures = reportData.fixtures ?? {};
  for (const [key, expected] of cases) {
    checkFixture(path, key, expected, fixtures[key]);
  }
}

checkAgreement(reports);

if (failureCount > 0) {
  console.error(`Conformance report check failed: ${failureCount} failure(s).`);
  process.exit(1);
}

console.log(
  `Conformance report check OK: ${reports.length} report(s) over the full fixture suite.`,
);
