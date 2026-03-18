import fs from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';
import { chromium } from 'playwright';

const ROOT = process.cwd();
const BUNDLE_PATH = path.join(ROOT, 'fixtures', 'package', 'dist', 'fp.umd.min.js');
const DEFAULT_OUTPUT_PATH = path.join(
  ROOT,
  'fixtures',
  'fingerprintjs',
  'browser-chromium-harness.json',
);

function parseArgs(argv) {
  let outputPath = DEFAULT_OUTPUT_PATH;

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];

    if (arg === '--output') {
      outputPath = path.resolve(ROOT, argv[index + 1]);
      index += 1;
    }
  }

  return { outputPath };
}

async function buildHarnessReport(page) {
  return page.evaluate(async () => {
    try {
      const agent = await FingerprintJS.load({
        debug: false,
        monitoring: false,
      });
      const result = await agent.get();
      const componentKeys = Object.keys(result.components).sort();
      const failedComponents = componentKeys.filter(
        (key) => 'error' in result.components[key],
      );
      const failedComponentErrors = failedComponents.map((key) => ({
        key,
        name: result.components[key].error?.name ?? 'Error',
        message: String(
          result.components[key].error?.message ??
          result.components[key].error,
        ),
      }));

      return {
        ok: true,
        version: result.version ?? null,
        visitor_id: result.visitorId ?? null,
        confidence_score: result.confidence?.score ?? null,
        component_count: componentKeys.length,
        error_component_count: failedComponents.length,
        component_keys: componentKeys,
        failed_components: failedComponents,
        failed_component_errors: failedComponentErrors,
        top_level_error: null,
      };
    } catch (error) {
      return {
        ok: false,
        version: null,
        visitor_id: null,
        confidence_score: null,
        component_count: 0,
        error_component_count: 0,
        component_keys: [],
        failed_components: [],
        failed_component_errors: [],
        top_level_error: {
          name: error?.name ?? 'Error',
          message: String(error?.message ?? error),
        },
      };
    }
  });
}

async function main() {
  const { outputPath } = parseArgs(process.argv.slice(2));
  const bundle = await fs.readFile(BUNDLE_PATH, 'utf8');
  const browser = await chromium.launch({ headless: true });

  try {
    const page = await browser.newPage();
    await page.setContent('<!doctype html><html><head></head><body></body></html>');
    await page.addScriptTag({ content: bundle });

    const report = await buildHarnessReport(page);

    await fs.mkdir(path.dirname(outputPath), { recursive: true });
    await fs.writeFile(outputPath, `${JSON.stringify(report, null, 2)}\n`);
    process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
  } finally {
    await browser.close();
  }
}

main().catch((error) => {
  process.stderr.write(`${error?.stack ?? error}\n`);
  process.exitCode = 1;
});
