import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import Ajv2020 from 'ajv/dist/2020.js';
import addFormats from 'ajv-formats';

const ROOT = fileURLToPath(new URL('../../', import.meta.url));

function readJson(rel) {
  return JSON.parse(readFileSync(join(ROOT, rel), 'utf8'));
}

function walkFixtures(dir) {
  const fullDir = join(ROOT, dir);
  if (!existsSync(fullDir)) {
    return [];
  }

  return readdirSync(fullDir, { recursive: true, withFileTypes: true })
    .filter((entry) => entry.isFile())
    .map((entry) => join(entry.parentPath, entry.name));
}

function parseFixture(file) {
  try {
    return JSON.parse(readFileSync(file, 'utf8'));
  } catch (error) {
    console.error(`::error file=${relative(ROOT, file)}::not valid JSON: ${error.message}`);
    return undefined;
  }
}

function reportValidationFailure(file, kind, validate) {
  console.error(`::error file=${relative(ROOT, file)}::${kind} validation failed`);
  for (const error of validate.errors ?? []) {
    console.error(`  ${error.instancePath || '/'} ${error.message}`);
  }
}

const ajv = new Ajv2020({ allErrors: true, strict: true });
addFormats(ajv);

const validateDescriptor = ajv.compile(readJson('schema/v1/openakb.schema.json'));
let validateProvenance;
let isProvenanceSchemaMissing = false;

try {
  validateProvenance = ajv.compile(readJson('schema/v1/provenance.schema.json'));
} catch (error) {
  // Provenance sidecar schema lands in Task 3; until then, tolerate only absence
  // when no sidecar fixtures exist.
  if (error.code !== 'ENOENT') {
    throw error;
  }
  isProvenanceSchemaMissing = true;
}

const fixtureFiles = ['examples', 'conformance/valid', 'conformance/forward-compat']
  .flatMap((dir) => walkFixtures(dir));

const descriptorFiles = fixtureFiles.filter((file) => file.endsWith('openakb.json'));
const sidecarFiles = fixtureFiles.filter((file) => file.endsWith('.prov.json'));

let failureCount = 0;

if (isProvenanceSchemaMissing && sidecarFiles.length > 0) {
  console.error(`::error::provenance schema missing but ${sidecarFiles.length} file(s) present`);
  failureCount += 1;
}

for (const file of descriptorFiles) {
  const fixture = parseFixture(file);
  if (fixture === undefined) {
    failureCount += 1;
    continue;
  }
  if (!validateDescriptor(fixture)) {
    reportValidationFailure(file, 'descriptor', validateDescriptor);
    failureCount += 1;
  }
}

for (const file of sidecarFiles) {
  if (isProvenanceSchemaMissing) {
    continue;
  }
  const fixture = parseFixture(file);
  if (fixture === undefined) {
    failureCount += 1;
    continue;
  }
  if (validateProvenance && !validateProvenance(fixture)) {
    reportValidationFailure(file, 'provenance', validateProvenance);
    failureCount += 1;
  }
}

if (failureCount > 0) {
  console.error(`Schema failed: ${failureCount} failure(s).`);
  process.exit(1);
}

console.log(`Schema OK: ${descriptorFiles.length} descriptor(s), ${sidecarFiles.length} sidecar(s) valid.`);
