import { resolve } from 'node:path';

import { syncImmutableFixture } from './support/immutable-fixture.mts';

const commit = 'becd3517c582fa68f041364b02bc597ee822ef1c';
const directory = resolve('fixtures/fonts/oxanium-wght');
const check = process.argv.includes('--check');

await syncImmutableFixture({
  baseUrl: `https://raw.githubusercontent.com/google/fonts/${commit}`,
  check,
  directory,
  files: [
    {
      localName: 'Oxanium[wght].ttf',
      remotePath: 'ofl/oxanium/Oxanium%5Bwght%5D.ttf',
      sha256: '2ce01d946e1e1ffc8d7eecfffbda8623bedd63eaf811a20488c4b69af45babb0',
    },
    {
      localName: 'OFL.txt',
      remotePath: 'ofl/oxanium/OFL.txt',
      sha256: 'fe17c0f2581d71b4e1ea7e636e7f4877c29223e11bb1dd1a871e8c3f2a86336b',
    },
  ],
});
/* @workflow { "name": "font:oxanium:sync", "summary": "Synchronize the authenticated Oxanium variable-font fixture.", "requirements": "Network access to the pinned source.", "writes": "Checked-in font, metadata, and license." } */
/* @workflow { "name": "font:oxanium:check", "summary": "Verify the authenticated Oxanium variable-font fixture.", "requirements": "Checked-in authenticated fixture.", "writes": "Nothing.", "args": ["--check"] } */
