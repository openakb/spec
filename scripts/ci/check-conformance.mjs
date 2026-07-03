import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import Ajv2020 from 'ajv/dist/2020.js';
import addFormats from 'ajv-formats';

const ROOT = fileURLToPath(new URL('../../', import.meta.url));
const CATALOG = new Set([
  'AKB001',
  'AKB002',
  'AKB003',
  'AKB004',
  'AKB005',
  'AKB006',
  'AKB007',
  'AKB008',
  'AKB009',
  'AKB010',
  'AKB011',
  'AKB012',
]);
const SCHEMA_CATCHABLE_CODES = new Set(['AKB003', 'AKB005', 'AKB008', 'AKB009', 'AKB011', 'AKB012']);
const LOCAL_ID_PATTERN = /^[a-z0-9_-]{1,64}$/u;

// Normative JSON-Schema-keyword → error-code mapping (spec §7). Structural special cases are
// resolved first (see mapErrorToCode): the link-level anyOf (target rule) → AKB012, the
// section-level if/then (content-cite rule) → AKB003, and `rel`'s controlled-vocab anyOf →
// AKB008. This table is the fallback for every other keyword.
const KEYWORD_CODES = new Map([
  ['maxLength', 'AKB005'],
  ['maxItems', 'AKB005'],
  ['required', 'AKB009'],
  ['pattern', 'AKB011'],
  ['format', 'AKB011'],
  ['type', 'AKB011'],
  ['minimum', 'AKB011'],
  ['minLength', 'AKB011'],
  ['minItems', 'AKB011'],
  ['uniqueItems', 'AKB011'],
  ['enum', 'AKB011'],
  ['anyOf', 'AKB011'],
  ['propertyNames', 'AKB011'],
]);

const LINK_INSTANCE_PATTERN = /\/links\/\d+$/u;

function mapErrorToCode(error) {
  // Ajv reports schemaPath relative to the $ref-resolved def. The link-level target-rule anyOf
  // (and its branch errors) is recognized as: at a link object, under an anyOf schemaPath.
  if (LINK_INSTANCE_PATTERN.test(error.instancePath) && error.schemaPath.includes('/anyOf')) {
    return 'AKB012';
  }
  // The section content-cite rule is the only if/then in the schema; its `then` branch errors
  // (required source_ids / minItems) are AKB003, not the generic AKB009/AKB011 those keywords map to.
  if (error.schemaPath.includes('/then/')) {
    return 'AKB003';
  }
  // On `rel`, only the controlled-vocab/reverse-DNS anyOf (and its enum/pattern branch errors)
  // is AKB008; a wrong JSON type is a malformed value (AKB011) per spec §7.
  if (error.instancePath.endsWith('/rel')) {
    return error.keyword === 'type' ? 'AKB011' : 'AKB008';
  }
  return KEYWORD_CODES.get(error.keyword);
}

const ajv = new Ajv2020({ allErrors: true, strict: true });
addFormats(ajv);
const validateDescriptor = ajv.compile(readJson('schema/v1/openakb.schema.json'));

let failureCount = 0;
const declaredCodes = new Set();

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

function readJson(path) {
  return JSON.parse(readFileSync(join(ROOT, path), 'utf8'));
}

function parseJson(file) {
  try {
    return JSON.parse(readFileSync(file, 'utf8'));
  } catch (error) {
    report(`not valid JSON: ${error.message}`, file);
    return undefined;
  }
}

function fixtureDirs(path) {
  const dir = join(ROOT, path);
  if (!existsSync(dir)) {
    report(`missing fixture directory: ${path}`);
    return [];
  }

  const cases = readdirSync(dir, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
    .sort();
  if (cases.length === 0) {
    report(`fixture directory has no cases: ${path}`);
  }
  return cases;
}

function requireFile(file) {
  if (!existsSync(file)) {
    report('required file missing', file);
    return false;
  }
  return true;
}

function requireCatalogCodes(codes, file, fieldName) {
  if (!Array.isArray(codes) || codes.length === 0) {
    report(`${fieldName} must be a non-empty array`, file);
    return [];
  }

  const validCodes = [];
  for (const code of codes) {
    if (!CATALOG.has(code)) {
      report(`${fieldName} contains unknown code: ${code}`, file);
      continue;
    }
    declaredCodes.add(code);
    validCodes.push(code);
  }
  return validCodes;
}

function reportDescriptorErrors(file, kind) {
  report(`${kind} descriptor schema validation failed`, file);
  for (const error of validateDescriptor.errors ?? []) {
    console.error(`  ${error.instancePath || '/'} ${error.message}`);
  }
}

function descriptorPassesSchema(descriptor, file, kind) {
  const isValid = validateDescriptor(descriptor);
  if (!isValid && kind) {
    reportDescriptorErrors(file, kind);
  }
  return isValid;
}

function validateCitationExpected(expected, expectedFile) {
  if (!Array.isArray(expected.citations)) {
    report('citations must be an array', expectedFile);
    return;
  }

  expected.citations.forEach((citation, citationIndex) => {
    if (citation === null || typeof citation !== 'object' || Array.isArray(citation)) {
      report(`citations[${citationIndex}] must be an object`, expectedFile);
      return;
    }
    if (!Array.isArray(citation.ids) || citation.ids.length === 0) {
      report(`citations[${citationIndex}].ids must be a non-empty array`, expectedFile);
      return;
    }
    citation.ids.forEach((id, idIndex) => {
      if (typeof id !== 'string') {
        report(`citations[${citationIndex}].ids[${idIndex}] must be a string`, expectedFile);
        return;
      }
      if (!LOCAL_ID_PATTERN.test(id)) {
        report(`citations[${citationIndex}].ids[${idIndex}] must match the local ID grammar`, expectedFile);
      }
    });
  });
}

for (const caseName of fixtureDirs('conformance/valid')) {
  const descriptorFile = join(ROOT, 'conformance/valid', caseName, 'openakb.json');
  if (!requireFile(descriptorFile)) {
    continue;
  }
  const descriptor = parseJson(descriptorFile);
  if (descriptor !== undefined) {
    descriptorPassesSchema(descriptor, descriptorFile, 'valid fixture');
  }
}

for (const caseName of fixtureDirs('conformance/invalid')) {
  const caseDir = join(ROOT, 'conformance/invalid', caseName);
  const descriptorFile = join(caseDir, 'openakb.json');
  const expectedFile = join(caseDir, 'expected.json');
  const hasDescriptor = requireFile(descriptorFile);
  const hasExpected = requireFile(expectedFile);

  const descriptor = hasDescriptor ? parseJson(descriptorFile) : undefined;
  const expected = hasExpected ? parseJson(expectedFile) : undefined;
  let codes = [];

  if (expected !== undefined) {
    codes = requireCatalogCodes(expected.codes, expectedFile, 'codes');
    if (expected.schema !== undefined && expected.schema !== false) {
      report('schema, when present, must be false (not schema-catchable despite the code)', expectedFile);
    }
  }

  if (descriptor !== undefined) {
    // `"schema": false` marks a fixture whose code is normally schema-catchable but whose
    // specific violation is not JSON-Schema-expressible (e.g., the depth cap under AKB005).
    const schemaCatchableCodes =
      expected?.schema === false ? [] : codes.filter((code) => SCHEMA_CATCHABLE_CODES.has(code));
    const isSchemaValid = validateDescriptor(descriptor);
    if (schemaCatchableCodes.length > 0) {
      if (isSchemaValid) {
        report('schema-catchable invalid fixture unexpectedly validates', descriptorFile);
      } else {
        const firedCodes = new Set(
          (validateDescriptor.errors ?? []).map(mapErrorToCode).filter(Boolean),
        );
        const missing = schemaCatchableCodes.filter((code) => !firedCodes.has(code));
        if (missing.length > 0) {
          const fired = [...firedCodes].sort().join(', ') || '(none)';
          report(`declared code(s) not matched by fired schema errors — declared: ${missing.join(', ')}; fired: ${fired}`, descriptorFile);
        }
      }
    } else if (!isSchemaValid) {
      reportDescriptorErrors(descriptorFile, 'semantic invalid fixture');
    }
  }
}

for (const caseName of fixtureDirs('conformance/forward-compat')) {
  const caseDir = join(ROOT, 'conformance/forward-compat', caseName);
  const descriptorFile = join(caseDir, 'openakb.json');
  const expectedFile = join(caseDir, 'expected.json');
  requireFile(descriptorFile);
  const hasExpected = requireFile(expectedFile);
  const expected = hasExpected ? parseJson(expectedFile) : undefined;

  if (expected === undefined) {
    continue;
  }
  if (expected.lenient !== 'valid') {
    report('lenient must equal "valid"', expectedFile);
  }
  requireCatalogCodes(expected.strict, expectedFile, 'strict');
}

for (const caseName of fixtureDirs('conformance/content')) {
  const caseDir = join(ROOT, 'conformance/content', caseName);
  const contentFile = join(caseDir, 'content.md');
  const expectedFile = join(caseDir, 'expected.json');
  requireFile(contentFile);
  const hasExpected = requireFile(expectedFile);
  const expected = hasExpected ? parseJson(expectedFile) : undefined;

  if (expected !== undefined) {
    validateCitationExpected(expected, expectedFile);
  }
}

for (const code of CATALOG) {
  if (!declaredCodes.has(code)) {
    report(`missing fixture declaring ${code}`);
  }
}

if (failureCount > 0) {
  console.error(`Conformance manifest failed: ${failureCount} failure(s).`);
  process.exit(1);
}

console.log(`Conformance manifest OK: ${CATALOG.size}/${CATALOG.size} codes have fixtures.`);
