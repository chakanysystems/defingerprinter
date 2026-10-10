#!/usr/bin/env node
// Build-only reference harness. Runtime deployment does not require Node.
import { strict as assert } from 'node:assert';
import { mkdir, writeFile, readdir, readFile } from 'node:fs/promises';
import { resolve, dirname } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { spawnSync } from 'node:child_process';
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const reference = resolve(process.env.RAMPART_REFERENCE_DIR ?? `${root}/reference`);
const importReference = path => import(pathToFileURL(resolve(reference, 'node_modules', path)).href);
const { env, AutoTokenizer } = await importReference('@huggingface/transformers/dist/transformers.node.mjs');
const { loadNerClassifier, detectNer, ChatGuard, detectHeuristics } = await importReference('@nationaldesignstudio/rampart/dist/index.js');
const model = resolve(process.env.RAMPART_MODEL_DIR ?? `${root}/models/rampart`);
const rampartVersion = JSON.parse(await readFile(resolve(reference, 'node_modules/@nationaldesignstudio/rampart/package.json'), 'utf8')).version;
const transformersVersion = JSON.parse(await readFile(resolve(reference, 'node_modules/@huggingface/transformers/package.json'), 'utf8')).version;
assert.equal(rampartVersion, '0.1.3', 'review parity reference before changing Rampart');
assert.equal(transformersVersion, '4.3.1', 'review aggregation semantics before changing Transformers');
env.allowRemoteModels = false;
env.allowLocalModels = true;
const tokenizer = await AutoTokenizer.from_pretrained(model, { local_files_only: true });
const classifier = await loadNerClassifier({ model, device: 'cpu' });
const sentences = [
  ['en', 'My name is Alice Johnson and I live at 123 Main Street in Boston.'],
  ['es', 'Me llamo José García y vivo en la calle Mayor 23 de Madrid.'],
  ['fr', 'Je suis Émilie Dupont et mon adresse est 12 rue Victor Hugo à Paris.'],
  ['de', 'Ich heiße Müller und wohne in der Hauptstraße 15 in Berlin.'],
  ['it', 'Mi chiamo Giulia Rossi e abito in via Roma 12 a Milano.'],
  ['pt', 'Meu nome é João Silva e moro na Rua das Flores 42 em Lisboa.'],
  ['nl', 'Mijn naam is Jan de Vries en ik woon op Kerkstraat 12 in Amsterdam.'],
  ['accents', 'José García met Jose\u0301 Garci\u0301a. Émilie Müller João François Łukasz.'],
  ['punctuation', "O’Connor, Anne-Marie, D'Angelo, van der Meer; Straße ﬃ ﬁ ①."],
  ['controls', 'Alice\u0000Johnson\u0007\tmet\nBob\rSmith\u200b in\u00a0Paris.'],
  ['special', 'Before [CLS]Alice[MASK]Johnson[SEP] after [UNK] [PAD].'],
  ['cjk', 'Alice 中文 東京 😀 Johnson'],
  ['unknown', 'a'.repeat(101) + ' zqvvxqz hello'],
];
// Deterministic normalization stress cases cover Latin extensions, combining marks,
// punctuation, cleanup, astral characters and all added special tokens.
const alphabet = [...'aAZéÉñÑüÜßİıŁøœæﬃ①中文😀\u0301\u0327\u200b\u0000\t\n\u00a0.,-’\''];
let seed = 0x5e15;
const next = () => { seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0; return seed; };
const stress = Array.from({ length: 128 }, (_, index) => {
  let text = '';
  for (let i = 0; i < 24; i++) text += alphabet[next() % alphabet.length];
  return [`normalization-${index}`, text + ' [MASK]hello[SEP]'];
});
const tokenizerCases = [...sentences, ...stress].map(([name, text]) => ({ name, text,
  ids: tokenizer.encode(text, { add_special_tokens: false }), tokens: tokenizer.tokenize(text) }));
const heuristicTexts = [
  'SSN 123-45-6789; 000-12-3456; 666-12-3456; 900-12-3456; 123-00-3456; 123-45-0000.',
  '4111 1111 1111 1111, 4111-1111-1111-1112, 378282246310005, 5555555555554444.',
  'alice+tag@example.org; a_b@example.co.uk; invalid@example; José jose@example.net.',
  'https://example.org/private?a=1, www.example.org/user and http://192.168.1.42/account.',
  '192.168.1.42; 255.255.255.255; 256.1.2.3; aa:bb:cc:dd:ee:ff; aa-bb-cc-dd-ee-ff.',
  '::1; 2001:db8::1; 2001:0db8:0000:0000:0000:ff00:0042:8329; [fe80::1]; ::ffff:192.0.2.128.',
  'name:1234:5678:abcd:ef01:2345:6789:abcd:ef01suffix; bad 1:2:3:4:5:6:7:8:9.',
  'é 123-45-6789, 😀 alice@example.org, 中文 https://example.org.',
];
const heuristicCases = heuristicTexts.map((text, index) => ({ name: `heuristic-${index}`, text,
  spans: detectHeuristics(text).map(s => ({ start: Buffer.byteLength(text.slice(0, s.start)),
    end: Buffer.byteLength(text.slice(0, s.end)), label: s.label })) }));
const cases = [];
for (const [name, text] of sentences.filter(([name]) => name !== 'cjk')) {
  const spans = await detectNer(text, classifier);
  cases.push({ name, text, spans: spans.map(s => ({ start: Buffer.byteLength(text.slice(0, s.start)),
    end: Buffer.byteLength(text.slice(0, s.end)), label: s.label, score: s.score })) });
}
// 'status normal' is exactly two content tokens. Place names near the first
// overlap start (436), first end (500), next end (936), and much later windows.
assert.equal(tokenizer.encode('status normal ', { add_special_tokens: false }).length, 2);
for (const count of [215, 245, 250, 465, 940]) {
  const text = 'status normal '.repeat(count) + 'Alice Johnson lives at 123 Main Street in Boston. ' + 'status normal '.repeat(90) + 'Contact José García.';
  const spans = await detectNer(text, classifier);
  cases.push({ name: `seam-${count}`, text, spans: spans.map(s => ({ start: Buffer.byteLength(text.slice(0, s.start)),
    end: Buffer.byteLength(text.slice(0, s.end)), label: s.label, score: s.score })) });
}
const structured = [
  ['structured', 'Alice Johnson email alice@example.org SSN 123-45-6789 card 4111 1111 1111 1111 server 192.168.1.42 and 2001:db8::1 MAC aa:bb:cc:dd:ee:ff.'],
  ['unicode-structured', 'José García: jose@example.org; Émilie Dupont: emilie@example.net; URL https://private.example.org/account.'],
  ['repeated', 'Contact Alice Johnson at alice@example.org. Again alice@example.org.'],
];
const protectCases = [];
for (const [name, text] of structured) {
  const guard = new ChatGuard({ ner: text => detectNer(text, classifier) });
  const result = await guard.protect(text);
  // Compare the retained text and replacement labels, not per-session numeric IDs.
  protectCases.push({ name, text, expected: result.text.replace(/\[([A-Z_]+)_\d+\]/g, '[$1]') });
}
assert(cases.some(c => c.spans.length), 'reference model must detect contextual PII');
const out = resolve(root, 'target/native-parity/reference.json');
await mkdir(dirname(out), { recursive: true });
await writeFile(out, JSON.stringify({ rampartVersion, transformersVersion, tokenizerCases, heuristicCases, cases, protectCases }));
let runtime = process.env.RAMPART_RUNTIME ?? (process.platform === 'darwin' && process.arch === 'arm64'
  ? resolve(root, 'target/native-runtime/package/libonnxruntime.dylib') : undefined);
if (!runtime) {
  const base = resolve(reference, 'node_modules/onnxruntime-node/bin');
  const napi = (await readdir(base)).find(n => n.startsWith('napi-v'));
  const platform = process.platform === 'darwin' ? 'darwin' : 'linux';
  runtime = resolve(base, napi, platform, process.arch, process.platform === 'darwin' ? 'libonnxruntime.1.dylib' : 'libonnxruntime.so.1');
}
const run = spawnSync('cargo', ['test', '--locked', '--no-fail-fast', '--manifest-path', resolve(root, 'Cargo.toml'), '--test', 'parity', '--test', 'protocol', '--test', 'artifacts', '--', '--include-ignored', '--nocapture'], {
  cwd: root, stdio: 'inherit', env: { ...process.env, CARGO_TARGET_DIR: resolve(root, 'target/native-rust'),
    RAMPART_MODEL_DIR: model, RAMPART_RUNTIME: runtime, RAMPART_PARITY_FIXTURE: out }
});
process.exit(run.status ?? 1);
