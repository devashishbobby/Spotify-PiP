// Verify the release embeds the current ICO image data, catching stale native resources.
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';
import assert from 'node:assert/strict';

const root = fileURLToPath(new URL('../', import.meta.url));
const icon = readFileSync(resolve(root, 'src-tauri/icons/icon.ico'));
const executable = readFileSync(resolve(root, 'src-tauri/target/release/spotify-pip.exe'));
assert.equal(icon.readUInt16LE(2), 1, 'Expected an ICO file');
const frames = icon.readUInt16LE(4);
assert.ok(frames > 0, 'Icon has image frames');
for (let index = 0; index < frames; index++) {
  const entry = 6 + index * 16;
  const size = icon.readUInt32LE(entry + 8);
  const offset = icon.readUInt32LE(entry + 12);
  assert.ok(size > 0 && offset + size <= icon.length, 'Valid icon frame');
  assert.ok(executable.includes(icon.subarray(offset, offset + size)), `Release embeds icon frame ${index + 1}`);
}
console.log(`Verified all ${frames} current icon frames in the release executable.`);
