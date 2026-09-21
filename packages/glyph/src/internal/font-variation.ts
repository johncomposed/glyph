import type { FontVariationRequest } from '../font-baker/index.js';

/** The baker's `fvar` tag rule: exactly four printable ASCII bytes. */
const AXIS_TAG = /^[\x20-\x7e]{4}$/u;

/** Freeze caller-authored axis settings; an absent or empty axis map is the `fvar` default instance. */
export function normalizeFontVariation(value: unknown, label: string): FontVariationRequest | undefined {
  if (value === undefined) return undefined;
  if (!isNonArrayObject(value) || !isNonArrayObject(value.axes)) {
    throw new TypeError(`${label} must be an object with an axes map, for example { axes: { wght: 700 } }`);
  }
  const axes: Record<string, number> = {};
  for (const [tag, setting] of Object.entries(value.axes)) {
    if (!AXIS_TAG.test(tag)) {
      throw new TypeError(`${label} axis tag ${JSON.stringify(tag)} must be exactly four printable ASCII bytes`);
    }
    if (typeof setting !== 'number' || !Number.isFinite(setting)) {
      throw new TypeError(`${label} axis ${tag} must be a finite number`);
    }
    axes[tag] = setting;
  }
  if (Object.keys(axes).length === 0) return undefined;
  return Object.freeze({ axes: Object.freeze(axes) });
}

function isNonArrayObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
