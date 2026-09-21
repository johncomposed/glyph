import type { FontFaceConfig, FontFaceFormat, FontFaceSource } from '../font-face.js';
import type { FontVariationRequest } from '../font-baker/index.js';
import { normalizeFontVariation } from './font-variation.js';
import { isRasterFormat, rasterFormatDescriptor } from './raster-format-registry.js';
import { canonicalJson } from './raster-identity.js';

const sourceIds = new WeakMap<object, number>();
let nextSourceId = 1;

/** Canonical identity for one adapter-owned FontFace declaration, including its pinned variation instance. */
export function fontResourceKey(
  source: FontFaceSource,
  format: FontFaceConfig['format'],
  variation?: FontVariationRequest,
): string {
  return `${fontFaceSourceKey(source)}:${fontFaceFormatIdentity(format)}:${fontFaceVariationIdentity(variation)}`;
}

function fontFaceSourceKey(source: FontFaceSource): string {
  if (typeof source === 'string') return `string:${source}`;
  if (source instanceof URL) return `url:${source.href}`;
  let id = sourceIds.get(source);
  if (id === undefined) {
    id = nextSourceId++;
    sourceIds.set(source, id);
  }
  return `object:${id}`;
}

function fontFaceFormatIdentity(format: FontFaceConfig['format']): string {
  if (format === undefined) return 'default';
  return isFontFaceFormatList(format) ? format.map(singleFormatIdentity).join('|') : singleFormatIdentity(format);
}

function isFontFaceFormatList(
  format: FontFaceConfig['format'],
): format is readonly [FontFaceFormat, ...FontFaceFormat[]] {
  return Array.isArray(format);
}

function singleFormatIdentity(format: FontFaceFormat): string {
  if (typeof format === 'string') return `key:${format}`;
  const raster = isRasterFormat(format) ? format : format.raster;
  return `raster:${raster.id}:${canonicalJson(rasterFormatDescriptor(format))}`;
}

function fontFaceVariationIdentity(variation: FontVariationRequest | undefined): string {
  const normalized = normalizeFontVariation(variation, 'FontFace variation');
  if (normalized === undefined) return 'default';
  return `variation:${canonicalJson(normalized.axes)}`;
}
